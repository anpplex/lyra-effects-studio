import CryptoKit
import Foundation
import Testing
@testable import LyraEffectsCLI
import LyraPackKit

@Suite("CLI")
struct CLITests {
    @Test func validateEmitsMachineReadableSuccess() throws {
        let result = run(["validate", packageRoot().appending(path: "Fixtures/Packs/valid-theme").path])
        let json = try CanonicalJSON.decode(JSONValue.self, from: Data(result.output.joined(separator: "\n").utf8))

        #expect(result.code == 0)
        #expect(json["command"] == .string("validate"))
        #expect(json["ok"] == .bool(true))
    }

    @Test func packCreatesDeterministicArtifact() throws {
        let destination = FileManager.default.temporaryDirectory.appending(path: "cli-pack-\(UUID().uuidString).zip")
        defer { try? FileManager.default.removeItem(at: destination) }

        let result = run([
            "pack",
            packageRoot().appending(path: "Fixtures/Packs/valid-theme").path,
            destination.path,
        ])

        #expect(result.code == 0)
        #expect(FileManager.default.fileExists(atPath: destination.path))
    }

    @Test func verifiesCommittedRegistryFixture() {
        let fixture = packageRoot().appending(path: "Fixtures/Registry")
        let result = run([
            "registry", "verify",
            fixture.appending(path: "registry-v1.json").path,
            fixture.appending(path: "registry-v1.sig").path,
            fixture.appending(path: "public-key.txt").path,
        ])

        #expect(result.code == 0)
        #expect(result.output.joined().contains(#""valid":true"#))
    }

    @Test func buildsRegistryArtifactsWithSuppliedTestKey() throws {
        let output = FileManager.default.temporaryDirectory.appending(path: "cli-registry-\(UUID().uuidString)")
        let keyURL = output.appending(path: "test-private-key.txt")
        try FileManager.default.createDirectory(at: output, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: output) }
        let key = try Curve25519.Signing.PrivateKey(rawRepresentation: Data(repeating: 7, count: 32))
        try key.rawRepresentation.base64EncodedString().write(to: keyURL, atomically: true, encoding: .utf8)

        let result = run([
            "registry", "build",
            packageRoot().appending(path: "Fixtures/Registry/registry-v1.json").path,
            output.path,
            keyURL.path,
        ])

        #expect(result.code == 0)
        #expect(FileManager.default.fileExists(atPath: output.appending(path: "registry-v1.sig").path))
        #expect(FileManager.default.fileExists(atPath: output.appending(path: "public-key.txt").path))
    }

    @Test func reportsUsageErrorsAsJSON() throws {
        let result = run(["unknown"])
        let json = try CanonicalJSON.decode(JSONValue.self, from: Data(result.output.joined(separator: "\n").utf8))

        #expect(result.code == 64)
        #expect(json["ok"] == .bool(false))
    }

    private func run(_ arguments: [String]) -> (code: Int32, output: [String]) {
        var output: [String] = []
        let code = CLI.run(arguments: arguments, write: { output.append($0) })
        return (code, output)
    }

    private func packageRoot() -> URL {
        URL(filePath: #filePath).deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
    }
}
