import Foundation
import Testing
@testable import LyraRegistryKit
import LyraPackKit

@Suite("Registry catalog")
struct RegistryCatalogTests {
    @Test func preservesUnknownCatalogAndPackFields() throws {
        let source = Data(#"{"schemaVersion":1,"registryId":"org.lyra.effects.official","name":"Official","generatedAt":"2026-07-14T00:00:00Z","keyId":"test-key","packs":[{"id":"org.lyra.effects.one","name":"One","family":"better-lyrics","version":"1.0.0","manifestUrl":"packs/org.lyra.effects.one/1.0.0/lyra-pack.json","downloadUrl":"packs/org.lyra.effects.one/1.0.0/pack.lyra-pack.zip","sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","signature":"c2ln","size":10,"minimumRuntimeApi":"1.0.0","packFuture":true}],"future":{"kept":true}}"#.utf8)

        let catalog = try CanonicalJSON.decode(RegistryCatalog.self, from: source)
        let roundTrip = try CanonicalJSON.decode(JSONValue.self, from: CanonicalJSON.encode(catalog))

        #expect(roundTrip["future"]?["kept"] == .bool(true))
        #expect(roundTrip["packs"] == .array([.object([
            "id": .string("org.lyra.effects.one"),
            "name": .string("One"),
            "family": .string("better-lyrics"),
            "version": .string("1.0.0"),
            "manifestUrl": .string("packs/org.lyra.effects.one/1.0.0/lyra-pack.json"),
            "downloadUrl": .string("packs/org.lyra.effects.one/1.0.0/pack.lyra-pack.zip"),
            "sha256": .string(String(repeating: "a", count: 64)),
            "signature": .string("c2ln"),
            "size": .number(10),
            "minimumRuntimeApi": .string("1.0.0"),
            "packFuture": .bool(true),
        ])]))
    }

    @Test func builderSortsAndRejectsDuplicateVersions() throws {
        let one = try artifact(id: "org.lyra.effects.zed", version: "1.0.0")
        let two = try artifact(id: "org.lyra.effects.alpha", version: "2.0.0")
        let catalog = try RegistryBuilder().build(
            registryId: "org.lyra.effects.official", name: "Official", generatedAt: "2026-07-14T00:00:00Z",
            keyId: "test-key", packArtifacts: [one, two]
        )

        #expect(catalog.packs.map(\.id) == ["org.lyra.effects.alpha", "org.lyra.effects.zed"])
        #expect(throws: RegistryBuildError.self) {
            try RegistryBuilder().build(
                registryId: "org.lyra.effects.official", name: "Official", generatedAt: "2026-07-14T00:00:00Z",
                keyId: "test-key", packArtifacts: [one, one]
            )
        }
    }

    private func artifact(id: String, version: String) throws -> RegistryPackArtifact {
        try RegistryPackArtifact(
            id: id, name: id, family: "better-lyrics", version: SemanticVersion(version),
            manifestURL: "packs/\(id)/\(version)/lyra-pack.json",
            downloadURL: "packs/\(id)/\(version)/pack.lyra-pack.zip",
            sha256: String(repeating: "a", count: 64), signature: "c2ln", size: 10,
            minimumRuntimeAPI: SemanticVersion("1.0.0")
        )
    }
}
