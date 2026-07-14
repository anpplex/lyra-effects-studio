import Foundation

public enum ProjectMode: String, Codable, Equatable, Sendable {
    case repoBound
    case standalone
}

public struct ProjectDescriptor: Codable, Equatable, Sendable {
    public var mode: ProjectMode
    public var root: URL
    public var effectsRoot: URL

    public init(mode: ProjectMode, root: URL, effectsRoot: URL) {
        self.mode = mode
        self.root = root.standardizedFileURL
        self.effectsRoot = effectsRoot.standardizedFileURL
    }
}
