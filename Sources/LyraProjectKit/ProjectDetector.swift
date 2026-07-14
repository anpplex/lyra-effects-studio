import Foundation

public enum ProjectDetectionError: Error, Equatable {
    case pathDoesNotExist(String)
    case unrecognizedProject(String)
}

public struct ProjectDetector {
    private let fileManager: FileManager

    public init(fileManager: FileManager = .default) {
        self.fileManager = fileManager
    }

    public func detect(from selectedURL: URL) throws -> ProjectDescriptor {
        var selectedIsDirectory: ObjCBool = false
        guard fileManager.fileExists(atPath: selectedURL.path, isDirectory: &selectedIsDirectory) else {
            throw ProjectDetectionError.pathDoesNotExist(selectedURL.path)
        }

        var candidate = (selectedIsDirectory.boolValue ? selectedURL : selectedURL.deletingLastPathComponent()).standardizedFileURL
        var standaloneRoot: URL?
        while true {
            let effectsRoot = candidate.appending(path: "lyric-effects", directoryHint: .isDirectory)
            if isDirectory(at: effectsRoot) {
                return ProjectDescriptor(mode: .repoBound, root: candidate, effectsRoot: effectsRoot)
            }
            if standaloneRoot == nil,
               fileManager.fileExists(atPath: candidate.appending(path: "lyra-pack.json").path) {
                standaloneRoot = candidate
            }

            if candidate.path == "/" { break }
            let parent = candidate.deletingLastPathComponent().standardizedFileURL
            if parent.path == candidate.path { break }
            candidate = parent
        }

        if let standaloneRoot {
            return ProjectDescriptor(mode: .standalone, root: standaloneRoot, effectsRoot: standaloneRoot)
        }
        throw ProjectDetectionError.unrecognizedProject(selectedURL.path)
    }

    private func isDirectory(at url: URL) -> Bool {
        var isDirectory: ObjCBool = false
        return fileManager.fileExists(atPath: url.path, isDirectory: &isDirectory) && isDirectory.boolValue
    }
}
