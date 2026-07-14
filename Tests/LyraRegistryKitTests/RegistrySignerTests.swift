import CryptoKit
import Foundation
import Testing
@testable import LyraRegistryKit
import LyraPackKit

@Suite("Registry signatures")
struct RegistrySignerTests {
    @Test func signsCanonicalCatalogAndRejectsAlteration() throws {
        let privateKey = try Curve25519.Signing.PrivateKey(rawRepresentation: Data(repeating: 7, count: 32))
        let signer = RegistrySigner(privateKey: privateKey)
        let catalog = try sampleCatalog()
        let signature = try signer.sign(catalog)
        let verifier = RegistryVerifier(publicKey: privateKey.publicKey)

        #expect(try verifier.verify(catalog, signatureBase64: signature))
        var altered = catalog
        altered.name = "Altered"
        #expect(try !verifier.verify(altered, signatureBase64: signature))
    }

    @Test func verifiesPackChecksumAndDetachedSignature() throws {
        let privateKey = try Curve25519.Signing.PrivateKey(rawRepresentation: Data(repeating: 9, count: 32))
        let data = Data("pack bytes".utf8)
        let checksum = SHA256Digest.hex(data)
        let signature = try privateKey.signature(for: Data(checksum.utf8)).base64EncodedString()
        let verifier = RegistryVerifier(publicKey: privateKey.publicKey)

        #expect(verifier.verifyPack(data: data, expectedSHA256: checksum, signatureBase64: signature))
        #expect(!verifier.verifyPack(data: Data("altered".utf8), expectedSHA256: checksum, signatureBase64: signature))
    }

    @Test func committedFixtureVerifies() throws {
        let root = URL(filePath: #filePath)
            .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
            .appending(path: "Fixtures/Registry")
        let catalog = try CanonicalJSON.decode(
            RegistryCatalog.self,
            from: Data(contentsOf: root.appending(path: "registry-v1.json"))
        )
        let signature = try String(contentsOf: root.appending(path: "registry-v1.sig"), encoding: .utf8)
            .trimmingCharacters(in: .whitespacesAndNewlines)
        let publicKey = try String(contentsOf: root.appending(path: "public-key.txt"), encoding: .utf8)
            .trimmingCharacters(in: .whitespacesAndNewlines)

        #expect(try RegistryVerifier(publicKeyBase64: publicKey).verify(catalog, signatureBase64: signature))
    }

    private func sampleCatalog() throws -> RegistryCatalog {
        RegistryCatalog(
            schemaVersion: 1,
            registryId: "org.lyra.effects.official",
            name: "Official",
            generatedAt: "2026-07-14T00:00:00Z",
            keyId: "test-key",
            packs: []
        )
    }
}
