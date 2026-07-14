import Foundation
import Testing
@testable import LyraPackKit

@Suite("Pack validation")
struct PackValidatorTests {
    @Test func acceptsMinimalThemePack() throws {
        let root = try makePack()
        defer { try? FileManager.default.removeItem(at: root) }

        #expect(try PackValidator().validate(at: root).isEmpty)
    }

    @Test func rejectsThemeScriptsTraversalMissingFilesAndExecutables() throws {
        let root = try makePack(style: "../outside.css")
        defer { try? FileManager.default.removeItem(at: root) }
        try "alert(1)".write(to: root.appending(path: "effect.js"), atomically: true, encoding: .utf8)
        let executable = root.appending(path: "run.sh")
        try "#!/bin/sh".write(to: executable, atomically: true, encoding: .utf8)
        try FileManager.default.setAttributes([.posixPermissions: 0o755], ofItemAtPath: executable.path)

        let codes = Set(try PackValidator().validate(at: root).map(\.code))

        #expect(codes.contains("path.traversal"))
        #expect(codes.contains("theme.scriptForbidden"))
        #expect(codes.contains("file.executableForbidden"))
    }

    @Test func rejectsEscapingSymlink() throws {
        let root = try makePack()
        defer { try? FileManager.default.removeItem(at: root) }
        let link = root.appending(path: "escape.css")
        try FileManager.default.createSymbolicLink(at: link, withDestinationURL: URL(filePath: "/tmp/outside.css"))

        let codes = Set(try PackValidator().validate(at: root).map(\.code))

        #expect(codes.contains("symlink.escapesRoot"))
    }

    private func makePack(style: String = "theme/lyra.css") throws -> URL {
        let root = FileManager.default.temporaryDirectory
            .appending(path: "lyra-pack-tests-\(UUID().uuidString)", directoryHint: .isDirectory)
        try FileManager.default.createDirectory(at: root.appending(path: "theme"), withIntermediateDirectories: true)
        try "body {}".write(to: root.appending(path: "theme/lyra.css"), atomically: true, encoding: .utf8)
        let manifest = #"{"schemaVersion":1,"id":"io.github.example.refine","name":"Refine","version":"1.0.0","kind":"theme","family":"better-lyrics","author":{"name":"Author"},"license":{"spdx":"MIT"},"compatibility":{"packSchema":">=1 <2","runtimeApi":">=1.0.0 <2.0.0","bridgeApi":">=1.0.0 <2.0.0"},"entry":{"style":"\#(style)"},"capabilities":["styles"]}"#
        try manifest.write(to: root.appending(path: "lyra-pack.json"), atomically: true, encoding: .utf8)
        return root
    }
}
