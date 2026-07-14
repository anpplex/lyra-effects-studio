import CryptoKit
import Foundation
import LyraPackKit

public struct RegistrySigner {
    private let privateKey: Curve25519.Signing.PrivateKey

    public init(privateKey: Curve25519.Signing.PrivateKey) {
        self.privateKey = privateKey
    }

    public func sign(_ catalog: RegistryCatalog) throws -> String {
        try privateKey.signature(for: CanonicalJSON.encode(catalog)).base64EncodedString()
    }

    public func signPackChecksum(_ lowercaseSHA256: String) throws -> String {
        try privateKey.signature(for: Data(lowercaseSHA256.utf8)).base64EncodedString()
    }

    public var publicKeyBase64: String {
        privateKey.publicKey.rawRepresentation.base64EncodedString()
    }
}
