import Foundation
import Testing
@testable import LyraPackKit

@Suite("Canonical JSON")
struct CanonicalJSONTests {
    private struct Example: Codable, Equatable {
        let b: String
        let a: Int
    }

    @Test func emitsSortedCompactKeysAndOneTrailingNewline() throws {
        let encoded = try CanonicalJSON.encode(Example(b: "two", a: 1))

        #expect(String(decoding: encoded, as: UTF8.self) == #"{"a":1,"b":"two"}"# + "\n")
    }

    @Test func outputIsByteForByteRepeatable() throws {
        let value = Example(b: "https://example.com/packs/one", a: 42)

        #expect(try CanonicalJSON.encode(value) == CanonicalJSON.encode(value))
    }

    @Test func decodesAndEncodesEveryJSONValueKind() throws {
        let source = Data(#"{"array":[true,null,2.5,"text"],"nested":{"future":7}}"#.utf8)

        let decoded = try CanonicalJSON.decode(JSONValue.self, from: source)
        let encoded = try CanonicalJSON.encode(decoded)
        let roundTrip = try CanonicalJSON.decode(JSONValue.self, from: encoded)

        #expect(decoded == roundTrip)
        #expect(decoded["nested"]?["future"] == .number(7))
    }

    @Test func rejectsTrailingNonWhitespace() {
        let source = Data(#"{"valid":true} garbage"#.utf8)

        #expect(throws: (any Error).self) {
            try CanonicalJSON.decode(JSONValue.self, from: source)
        }
    }
}
