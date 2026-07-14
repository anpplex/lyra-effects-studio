import CryptoKit
import Foundation
import LyraPackKit
import LyraRegistryKit

enum RegistryCommand {
    static func execute(arguments: [String]) throws -> CLICommandResult {
        guard let operation = arguments.first else {
            throw CLIError.usage("Usage: lyra-effects registry <build|verify> ...")
        }
        switch operation {
        case "build": return try build(Array(arguments.dropFirst()))
        case "verify": return try verify(Array(arguments.dropFirst()))
        default: throw CLIError.usage("Usage: lyra-effects registry <build|verify> ...")
        }
    }

    private static func build(_ arguments: [String]) throws -> CLICommandResult {
        guard arguments.count == 3 else {
            throw CLIError.usage("Usage: lyra-effects registry build <catalog.json> <output-directory> <private-key-file>")
        }
        let catalogURL = URL(filePath: arguments[0])
        let output = URL(filePath: arguments[1])
        let keyURL = URL(filePath: arguments[2])
        var catalog = try CanonicalJSON.decode(RegistryCatalog.self, from: Data(contentsOf: catalogURL))
        catalog.packs.sort { ($0.id, $0.version) < ($1.id, $1.version) }
        try validateCatalogPacks(catalog.packs)

        let keyText = try String(contentsOf: keyURL, encoding: .utf8).trimmingCharacters(in: .whitespacesAndNewlines)
        guard let keyData = Data(base64Encoded: keyText),
              let privateKey = try? Curve25519.Signing.PrivateKey(rawRepresentation: keyData) else {
            throw CLIError.invalidData("Private key must be a base64-encoded Ed25519 raw key")
        }
        let signer = RegistrySigner(privateKey: privateKey)
        let signature = try signer.sign(catalog)

        try FileManager.default.createDirectory(at: output, withIntermediateDirectories: true)
        try CanonicalJSON.encode(catalog).write(to: output.appending(path: "registry-v1.json"), options: .atomic)
        try (signature + "\n").write(to: output.appending(path: "registry-v1.sig"), atomically: true, encoding: .utf8)
        try (signer.publicKeyBase64 + "\n").write(to: output.appending(path: "public-key.txt"), atomically: true, encoding: .utf8)
        return .success(command: "registry build", data: .object([
            "packCount": .number(Double(catalog.packs.count)),
            "output": .string(output.path),
        ]))
    }

    private static func verify(_ arguments: [String]) throws -> CLICommandResult {
        guard arguments.count == 3 else {
            throw CLIError.usage("Usage: lyra-effects registry verify <catalog.json> <signature-file> <public-key-file>")
        }
        let catalog = try CanonicalJSON.decode(RegistryCatalog.self, from: Data(contentsOf: URL(filePath: arguments[0])))
        try validateCatalogPacks(catalog.packs)
        let signature = try String(contentsOfFile: arguments[1], encoding: .utf8).trimmingCharacters(in: .whitespacesAndNewlines)
        let publicKey = try String(contentsOfFile: arguments[2], encoding: .utf8).trimmingCharacters(in: .whitespacesAndNewlines)
        let valid = try RegistryVerifier(publicKeyBase64: publicKey).verify(catalog, signatureBase64: signature)
        let data = JSONValue.object(["valid": .bool(valid), "packCount": .number(Double(catalog.packs.count))])
        return valid ? .success(command: "registry verify", data: data) : .failure(command: "registry verify", message: "Registry signature is invalid", data: data)
    }

    private static func validateCatalogPacks(_ packs: [RegistryPack]) throws {
        var seen: Set<String> = []
        for pack in packs {
            let key = "\(pack.id)@\(pack.version.description)"
            guard seen.insert(key).inserted else {
                throw RegistryBuildError.duplicatePackVersion(id: pack.id, version: pack.version.description)
            }
            _ = try RegistryPackArtifact(
                id: pack.id, name: pack.name, family: pack.family, version: pack.version,
                manifestURL: pack.manifestURL, downloadURL: pack.downloadURL,
                sha256: pack.sha256, signature: pack.signature, size: pack.size,
                minimumRuntimeAPI: pack.minimumRuntimeAPI
            )
        }
    }
}
