import Foundation

public struct ProjectFileEvent: Equatable, Sendable {
    public var url: URL
    public var timestamp: TimeInterval

    public init(url: URL, timestamp: TimeInterval) {
        self.url = url.standardizedFileURL
        self.timestamp = timestamp
    }
}

public protocol ProjectEventSource: AnyObject {
    func readEvents() throws -> [ProjectFileEvent]
}

public struct ProjectWatcher {
    public let root: URL
    public let debounceInterval: TimeInterval
    private let eventSource: any ProjectEventSource
    private var pending: Set<URL> = []
    private var latestEventTimestamp: TimeInterval?

    public init(root: URL, eventSource: any ProjectEventSource, debounceInterval: TimeInterval = 0.25) {
        self.root = root.standardizedFileURL
        self.eventSource = eventSource
        self.debounceInterval = max(0, debounceInterval)
    }

    public mutating func poll(now: TimeInterval = Date().timeIntervalSince1970) throws -> [URL] {
        for event in try eventSource.readEvents() where shouldInclude(event.url) {
            pending.insert(event.url.standardizedFileURL)
            latestEventTimestamp = max(latestEventTimestamp ?? event.timestamp, event.timestamp)
        }
        guard let latestEventTimestamp, now - latestEventTimestamp >= debounceInterval else { return [] }

        let result = pending.sorted(by: { $0.path < $1.path })
        pending.removeAll(keepingCapacity: true)
        self.latestEventTimestamp = nil
        return result
    }

    private func shouldInclude(_ url: URL) -> Bool {
        let candidate = url.standardizedFileURL
        guard candidate.path.hasPrefix(root.path + "/") else { return false }
        let relative = String(candidate.path.dropFirst(root.path.count + 1))
        let components = relative.split(separator: "/").map(String.init)
        if components.contains(where: { [".build", ".git", ".swiftpm", "DerivedData"].contains($0) }) {
            return false
        }
        if components.starts(with: ["Registry", "Site"]) { return false }
        return true
    }
}

/// Portable polling source used until the AppKit layer supplies an FSEvents adapter.
public final class DirectorySnapshotEventSource: ProjectEventSource {
    private let root: URL
    private let fileManager: FileManager
    private var previous: [URL: Date] = [:]

    public init(root: URL, fileManager: FileManager = .default) {
        self.root = root.standardizedFileURL
        self.fileManager = fileManager
    }

    public func readEvents() throws -> [ProjectFileEvent] {
        var current: [URL: Date] = [:]
        let keys: [URLResourceKey] = [.isRegularFileKey, .contentModificationDateKey]
        if let enumerator = fileManager.enumerator(at: root, includingPropertiesForKeys: keys, options: [.skipsHiddenFiles]) {
            for case let url as URL in enumerator {
                let values = try url.resourceValues(forKeys: Set(keys))
                if values.isRegularFile == true {
                    current[url.standardizedFileURL] = values.contentModificationDate ?? .distantPast
                }
            }
        }

        let now = Date().timeIntervalSince1970
        let changed = Set(current.keys.filter { previous[$0] != current[$0] })
            .union(previous.keys.filter { current[$0] == nil })
        previous = current
        return changed.sorted(by: { $0.path < $1.path }).map { ProjectFileEvent(url: $0, timestamp: now) }
    }
}
