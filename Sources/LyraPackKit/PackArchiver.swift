import Foundation

public protocol ArchiveCommandRunning {
    func createArchive(workingDirectory: URL, relativeFiles: [String], destination: URL) throws
}

public struct ZipArchiveCommandRunner: ArchiveCommandRunning {
    public init() {}

    public func createArchive(workingDirectory: URL, relativeFiles: [String], destination: URL) throws {
        let process = Process()
        let stderr = Pipe()
        process.executableURL = URL(filePath: "/usr/bin/zip")
        process.currentDirectoryURL = workingDirectory
        process.arguments = ["-X", "-q", destination.path] + relativeFiles
        process.standardError = stderr
        try process.run()
        process.waitUntilExit()

        guard process.terminationStatus == 0 else {
            let data = stderr.fileHandleForReading.readDataToEndOfFile()
            throw PackArchiveError.commandFailed(
                status: process.terminationStatus,
                message: String(decoding: data, as: UTF8.self)
            )
        }
    }
}

public struct PackArtifact: Equatable, Sendable {
    public let url: URL
    public let sha256: String
    public let byteCount: Int
}

public enum PackArchiveError: Error {
    case invalidPack([PackDiagnostic])
    case noPackFiles
    case commandFailed(status: Int32, message: String)
}

public struct PackArchiver {
    public static let normalizedTimestamp = Date(timeIntervalSince1970: 315_532_800)

    private let validator: PackValidator
    private let runner: any ArchiveCommandRunning
    private let fileManager: FileManager

    public init(
        validator: PackValidator = .init(),
        runner: any ArchiveCommandRunning = ZipArchiveCommandRunner(),
        fileManager: FileManager = .default
    ) {
        self.validator = validator
        self.runner = runner
        self.fileManager = fileManager
    }

    public func build(source: URL, destination: URL) throws -> PackArtifact {
        let diagnostics = try validator.validate(at: source)
        guard diagnostics.allSatisfy({ $0.severity != .error }) else {
            throw PackArchiveError.invalidPack(diagnostics)
        }

        let staging = fileManager.temporaryDirectory
            .appending(path: "lyra-pack-staging-\(UUID().uuidString)", directoryHint: .isDirectory)
        try fileManager.createDirectory(at: staging, withIntermediateDirectories: true)
        defer { try? fileManager.removeItem(at: staging) }

        let relativeFiles = try stageFiles(from: source.standardizedFileURL, at: staging)
        guard !relativeFiles.isEmpty else { throw PackArchiveError.noPackFiles }

        try fileManager.createDirectory(at: destination.deletingLastPathComponent(), withIntermediateDirectories: true)
        if fileManager.fileExists(atPath: destination.path) {
            try fileManager.removeItem(at: destination)
        }
        try runner.createArchive(workingDirectory: staging, relativeFiles: relativeFiles, destination: destination)

        let attributes = try fileManager.attributesOfItem(atPath: destination.path)
        let byteCount = (attributes[.size] as? NSNumber)?.intValue ?? 0
        return PackArtifact(url: destination, sha256: try SHA256Digest.hex(fileAt: destination), byteCount: byteCount)
    }

    private func stageFiles(from source: URL, at staging: URL) throws -> [String] {
        let keys: [URLResourceKey] = [.isDirectoryKey, .isRegularFileKey, .isSymbolicLinkKey]
        guard let enumerator = fileManager.enumerator(
            at: source,
            includingPropertiesForKeys: keys,
            options: [.skipsHiddenFiles]
        ) else { return [] }

        let excludedDirectories: Set<String> = [".build", ".git", "DerivedData", "Site"]
        var files: [String] = []
        for case let url as URL in enumerator {
            let values = try url.resourceValues(forKeys: Set(keys))
            if values.isDirectory == true, excludedDirectories.contains(url.lastPathComponent) {
                enumerator.skipDescendants()
                continue
            }
            guard values.isRegularFile == true || values.isSymbolicLink == true else { continue }

            let relative = String(url.path.dropFirst(source.path.count + 1))
            let target = staging.appending(path: relative)
            try fileManager.createDirectory(at: target.deletingLastPathComponent(), withIntermediateDirectories: true)
            let contents = try Data(contentsOf: url.resolvingSymlinksInPath())
            try contents.write(to: target, options: .atomic)
            try fileManager.setAttributes([
                .posixPermissions: 0o644,
                .creationDate: Self.normalizedTimestamp,
                .modificationDate: Self.normalizedTimestamp,
            ], ofItemAtPath: target.path)
            files.append(relative)
        }
        return files.sorted()
    }
}
