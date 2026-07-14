import CryptoKit
import Foundation
import LyraPackKit

public enum RegistryVerificationError: Error, Equatable {
    case invalidPublicKeyEncoding
}

public struct RegistryVerifier {
    private let publicKey: Curve25519.Signing.PublicKey

    public init(publicKey: Curve25519.Signing.PublicKey) {
        self.publicKey = publicKey
    }

    public init(publicKeyBase64: String) throws {
        guard let data = Data(base64Encoded: publicKeyBase64),
              let key = try? Curve25519.Signing.PublicKey(rawRepresentation: data) else {
            throw RegistryVerificationError.invalidPublicKeyEncoding
        }
        self.publicKey = key
    }

    public func verify(_ catalog: RegistryCatalog, signatureBase64: String) throws -> Bool {
        guard let signature = Data(base64Encoded: signatureBase64) else { return false }
        return publicKey.isValidSignature(signature, for: try CanonicalJSON.encode(catalog))
    }

    public func verifyPack(data: Data, expectedSHA256: String, signatureBase64: String) -> Bool {
        guard SHA256Digest.hex(data) == expectedSHA256,
              let signature = Data(base64Encoded: signatureBase64) else { return false }
        return publicKey.isValidSignature(signature, for: Data(expectedSHA256.utf8))
    }
}
