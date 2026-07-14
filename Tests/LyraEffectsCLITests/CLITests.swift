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
        #expect(run(["registry", "verify-site", output.path]).code == 0)
    }

    @Test func signsPackChecksumsForPublication() throws {
        let output = FileManager.default.temporaryDirectory.appending(path: "cli-sign-\(UUID().uuidString)")
        let keyURL = output.appending(path: "test-private-key.txt")
        try FileManager.default.createDirectory(at: output, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: output) }
        let key = try Curve25519.Signing.PrivateKey(rawRepresentation: Data(repeating: 5, count: 32))
        try key.rawRepresentation.base64EncodedString().write(to: keyURL, atomically: true, encoding: .utf8)
        let checksum = String(repeating: "a", count: 64)

        let result = run(["registry", "sign-checksum", checksum, keyURL.path])
        let response = try CanonicalJSON.decode(JSONValue.self, from: Data(result.output.joined().utf8))
        guard case let .string(signature)? = response["data"]?["signature"] else {
            Issue.record("CLI response has no signature")
            return
        }

        #expect(result.code == 0)
        #expect(key.publicKey.isValidSignature(Data(base64Encoded: signature)!, for: Data(checksum.utf8)))
    }

    @Test func reportsUsageErrorsAsJSON() throws {
        let result = run(["unknown"])
        let json = try CanonicalJSON.decode(JSONValue.self, from: Data(result.output.joined(separator: "\n").utf8))

        #expect(result.code == 64)
        #expect(json["ok"] == .bool(false))
    }

    @Test func auditsRegistryLicenses() {
        let result = run(["license-audit", packageRoot().appending(path: "Registry").path])

        #expect(result.code == 0)
        #expect(result.output.joined().contains(#""includedCount":3"#))
        #expect(result.output.joined().contains(#""excludedCount":15"#))
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
