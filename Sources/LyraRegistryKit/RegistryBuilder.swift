import Foundation
import LyraPackKit

public struct RegistryPackArtifact: Equatable, Sendable {
    public var id: String
    public var name: String
    public var family: String
    public var version: SemanticVersion
    public var manifestURL: String
    public var downloadURL: String
    public var sha256: String
    public var signature: String
    public var size: Int
    public var minimumRuntimeAPI: SemanticVersion

    public init(
        id: String, name: String, family: String, version: SemanticVersion,
        manifestURL: String, downloadURL: String, sha256: String, signature: String,
        size: Int, minimumRuntimeAPI: SemanticVersion
    ) throws {
        guard id.wholeMatch(of: /[a-z0-9]+(?:\.[a-z0-9][a-z0-9-]*)+/) != nil else {
            throw RegistryBuildError.invalidPack(id: id, reason: "Invalid reverse-domain id")
        }
        guard sha256.wholeMatch(of: /[a-f0-9]{64}/) != nil else {
            throw RegistryBuildError.invalidPack(id: id, reason: "Invalid SHA-256")
        }
        guard Data(base64Encoded: signature) != nil else {
            throw RegistryBuildError.invalidPack(id: id, reason: "Invalid signature encoding")
        }
        let expectedPrefix = "packs/\(id)/\(version.description)/"
        for url in [manifestURL, downloadURL] {
            guard url.hasPrefix(expectedPrefix), Self.isImmutableRelativeURL(url) else {
                throw RegistryBuildError.invalidPack(id: id, reason: "Pack URLs must be immutable relative version paths")
            }
        }
        guard size >= 0 else { throw RegistryBuildError.invalidPack(id: id, reason: "Pack size cannot be negative") }

        self.id = id; self.name = name; self.family = family; self.version = version
        self.manifestURL = manifestURL; self.downloadURL = downloadURL; self.sha256 = sha256
        self.signature = signature; self.size = size; self.minimumRuntimeAPI = minimumRuntimeAPI
    }

    private static func isImmutableRelativeURL(_ source: String) -> Bool {
        guard !source.hasPrefix("/"), !source.split(separator: "/").contains(".."),
              let components = URLComponents(string: source) else { return false }
        return components.scheme == nil && components.host == nil && components.query == nil && components.fragment == nil
    }
}

public enum RegistryBuildError: Error, Equatable {
    case duplicatePackVersion(id: String, version: String)
    case invalidPack(id: String, reason: String)
}

public struct RegistryBuilder {
    public init() {}

    public func build(
        registryId: String,
        name: String,
        generatedAt: String,
        keyId: String,
        packArtifacts: [RegistryPackArtifact]
    ) throws -> RegistryCatalog {
        var seen: Set<String> = []
        for artifact in packArtifacts {
            let key = "\(artifact.id)@\(artifact.version.description)"
            guard seen.insert(key).inserted else {
                throw RegistryBuildError.duplicatePackVersion(id: artifact.id, version: artifact.version.description)
            }
        }

        let packs = packArtifacts
            .sorted { ($0.id, $0.version) < ($1.id, $1.version) }
            .map {
                RegistryPack(
                    id: $0.id, name: $0.name, family: $0.family, version: $0.version,
                    manifestURL: $0.manifestURL, downloadURL: $0.downloadURL,
                    sha256: $0.sha256, signature: $0.signature, size: $0.size,
                    minimumRuntimeAPI: $0.minimumRuntimeAPI
                )
            }
        return RegistryCatalog(
            schemaVersion: 1,
            registryId: registryId,
            name: name,
            generatedAt: generatedAt,
            keyId: keyId,
            packs: packs
        )
    }
}
