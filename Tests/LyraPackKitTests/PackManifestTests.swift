import Foundation
import Testing
@testable import LyraPackKit

@Suite("Pack manifest")
struct PackManifestTests {
    @Test func decodesPublicContractAndPreservesUnknownFields() throws {
        let source = Data(#"{"schemaVersion":1,"id":"io.github.example.refine","name":"Refine","version":"1.0.0","kind":"theme","family":"better-lyrics","author":{"name":"Author","future":"kept"},"license":{"spdx":"MIT","notice":"NOTICE"},"compatibility":{"packSchema":">=1 <2","runtimeApi":">=1.0.0 <2.0.0","bridgeApi":">=1.0.0 <2.0.0"},"entry":{"style":"theme/lyra.css"},"capabilities":["styles"],"future":{"nested":true}}"#.utf8)

        let manifest = try CanonicalJSON.decode(PackManifest.self, from: source)
        let roundTrip = try CanonicalJSON.decode(JSONValue.self, from: CanonicalJSON.encode(manifest))
        let expectedVersion = try SemanticVersion("1.0.0")

        #expect(manifest.id == "io.github.example.refine")
        #expect(manifest.version == expectedVersion)
        #expect(manifest.author.additionalFields["future"] == .string("kept"))
        #expect(roundTrip["future"]?["nested"] == .bool(true))
        #expect(roundTrip["author"]?["future"] == .string("kept"))
    }

    @Test func rejectsUnknownManifestSchema() {
        let source = Data(#"{"schemaVersion":2,"id":"io.example.pack","name":"Pack","version":"1.0.0","kind":"theme","family":"better-lyrics","author":{"name":"A"},"license":{"spdx":"MIT"},"compatibility":{"packSchema":">=1 <2","runtimeApi":">=1.0.0 <2.0.0","bridgeApi":">=1.0.0 <2.0.0"},"entry":{"style":"theme.css"},"capabilities":["styles"]}"#.utf8)

        #expect(throws: (any Error).self) {
            try CanonicalJSON.decode(PackManifest.self, from: source)
        }
    }
}
