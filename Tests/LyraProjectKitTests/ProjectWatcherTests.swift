import Foundation
import Testing
@testable import LyraProjectKit

@Suite("Project watcher")
struct ProjectWatcherTests {
    @Test func debouncesCoalescesAndSortsSourceChanges() throws {
        let root = URL(filePath: "/tmp/lyra-project")
        let source = FakeProjectEventSource(batches: [[
            .init(url: root.appending(path: "theme/b.css"), timestamp: 1.0),
            .init(url: root.appending(path: "theme/a.css"), timestamp: 1.1),
            .init(url: root.appending(path: "theme/a.css"), timestamp: 1.2),
        ]])
        var watcher = ProjectWatcher(root: root, eventSource: source, debounceInterval: 0.25)

        #expect(try watcher.poll(now: 1.3).isEmpty)
        #expect(try watcher.poll(now: 1.5).map(\.lastPathComponent) == ["a.css", "b.css"])
    }

    @Test func ignoresGeneratedAndRepositoryMetadata() throws {
        let root = URL(filePath: "/tmp/lyra-project")
        let source = FakeProjectEventSource(batches: [[
            .init(url: root.appending(path: ".build/output"), timestamp: 1),
            .init(url: root.appending(path: ".git/index"), timestamp: 1),
            .init(url: root.appending(path: "Registry/Site/registry-v1.json"), timestamp: 1),
            .init(url: root.appending(path: "theme/lyra.css"), timestamp: 1),
        ]])
        var watcher = ProjectWatcher(root: root, eventSource: source, debounceInterval: 0)

        #expect(try watcher.poll(now: 2).map(\.lastPathComponent) == ["lyra.css"])
    }
}

private final class FakeProjectEventSource: ProjectEventSource {
    private var batches: [[ProjectFileEvent]]
    init(batches: [[ProjectFileEvent]]) { self.batches = batches }

    func readEvents() throws -> [ProjectFileEvent] {
        batches.isEmpty ? [] : batches.removeFirst()
    }
}
