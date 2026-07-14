import Foundation
import Testing
@testable import LyraPackKit

@Suite("SHA-256")
struct SHA256DigestTests {
    @Test func hashesDataAsLowercaseHex() {
        #expect(SHA256Digest.hex(Data("abc".utf8)) == "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")
    }
}
