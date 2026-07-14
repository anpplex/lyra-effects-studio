import Foundation
import Testing
@testable import LyraPackKit

@Suite("Pack archiver")
struct PackArchiverTests {
    @Test func buildsByteIdenticalArchives() throws {
        let root = fixtureRoot()
        let outputRoot = FileManager.default.temporaryDirectory.appending(path: "pack-output-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: outputRoot, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: outputRoot) }

        let first = try PackArchiver().build(source: root, destination: outputRoot.appending(path: "first.lyra-pack.zip"))
        let second = try PackArchiver().build(source: root, destination: outputRoot.appending(path: "second.lyra-pack.zip"))

        #expect(first.sha256 == second.sha256)
        #expect(try Data(contentsOf: first.url) == Data(contentsOf: second.url))
        #expect(first.byteCount > 0)
    }

    @Test func passesSortedNormalizedFilesToRunner() throws {
        let root = fixtureRoot()
        let output = FileManager.default.temporaryDirectory.appending(path: "fake-\(UUID().uuidString).zip")
        defer { try? FileManager.default.removeItem(at: output) }
        let runner = InspectingRunner()

        _ = try PackArchiver(runner: runner).build(source: root, destination: output)

        #expect(runner.files == runner.files.sorted())
        #expect(runner.files == ["lyra-pack.json", "theme/lyra.css"])
        #expect(runner.permissions.allSatisfy { $0 == 0o644 })
        #expect(runner.modificationDates.allSatisfy { $0 == PackArchiver.normalizedTimestamp })
    }

    private func fixtureRoot() -> URL {
        URL(filePath: #filePath)
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .appending(path: "Fixtures/Packs/valid-theme")
    }
}

private final class InspectingRunner: ArchiveCommandRunning {
    var files: [String] = []
    var permissions: [Int] = []
    var modificationDates: [Date] = []

    func createArchive(workingDirectory: URL, relativeFiles: [String], destination: URL) throws {
        files = relativeFiles
        for path in relativeFiles {
            let attributes = try FileManager.default.attributesOfItem(atPath: workingDirectory.appending(path: path).path)
            permissions.append((attributes[.posixPermissions] as? NSNumber)?.intValue ?? -1)
            modificationDates.append(attributes[.modificationDate] as? Date ?? .distantPast)
        }
        try Data("fake archive".utf8).write(to: destination)
    }
}
