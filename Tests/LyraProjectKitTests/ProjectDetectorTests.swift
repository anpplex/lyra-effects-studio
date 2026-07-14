import Foundation
import Testing
@testable import LyraProjectKit

@Suite("Project detector")
struct ProjectDetectorTests {
    @Test func detectsRepoBoundProjectFromNestedFolder() throws {
        let root = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: root) }
        let nested = root.appending(path: "lyric-effects/packs/better-lyrics")
        try FileManager.default.createDirectory(at: nested, withIntermediateDirectories: true)

        let descriptor = try ProjectDetector().detect(from: nested)

        #expect(descriptor.mode == .repoBound)
        #expect(descriptor.root == root.standardizedFileURL)
        #expect(descriptor.effectsRoot == root.appending(path: "lyric-effects").standardizedFileURL)
    }

    @Test func detectsStandalonePack() throws {
        let root = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: root) }
        try "{}".write(to: root.appending(path: "lyra-pack.json"), atomically: true, encoding: .utf8)

        let descriptor = try ProjectDetector().detect(from: root)

        #expect(descriptor.mode == .standalone)
        #expect(descriptor.effectsRoot == root.standardizedFileURL)
    }

    @Test func rejectsUnrecognizedFolder() {
        let root = temporaryDirectory()
        defer { try? FileManager.default.removeItem(at: root) }

        #expect(throws: ProjectDetectionError.self) {
            try ProjectDetector().detect(from: root)
        }
    }

    private func temporaryDirectory() -> URL {
        let url = FileManager.default.temporaryDirectory.appending(path: "project-detect-\(UUID().uuidString)")
        try? FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
        return url
    }
}
