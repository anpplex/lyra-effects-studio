import Foundation
import LyraPackKit

private struct RegistryCodingKey: CodingKey, Hashable {
    let stringValue: String
    let intValue: Int? = nil
    init(_ value: String) { stringValue = value }
    init?(stringValue: String) { self.init(stringValue) }
    init?(intValue: Int) { return nil }
}

private extension KeyedDecodingContainer where Key == RegistryCodingKey {
    func value<T: Decodable>(_ type: T.Type, _ key: String) throws -> T {
        try decode(type, forKey: RegistryCodingKey(key))
    }
    func additional(excluding known: Set<String>) throws -> [String: JSONValue] {
        try allKeys.reduce(into: [:]) { fields, key in
            guard !known.contains(key.stringValue) else { return }
            fields[key.stringValue] = try decode(JSONValue.self, forKey: key)
        }
    }
}

private extension KeyedEncodingContainer where Key == RegistryCodingKey {
    mutating func value<T: Encodable>(_ value: T, _ key: String) throws {
        try encode(value, forKey: RegistryCodingKey(key))
    }
    mutating func additional(_ fields: [String: JSONValue]) throws {
        for key in fields.keys.sorted() { try encode(fields[key], forKey: RegistryCodingKey(key)) }
    }
}

public struct RegistryPack: Codable, Equatable, Sendable {
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
    public var additionalFields: [String: JSONValue]

    public init(
        id: String, name: String, family: String, version: SemanticVersion,
        manifestURL: String, downloadURL: String, sha256: String, signature: String,
        size: Int, minimumRuntimeAPI: SemanticVersion, additionalFields: [String: JSONValue] = [:]
    ) {
        self.id = id; self.name = name; self.family = family; self.version = version
        self.manifestURL = manifestURL; self.downloadURL = downloadURL; self.sha256 = sha256
        self.signature = signature; self.size = size; self.minimumRuntimeAPI = minimumRuntimeAPI
        self.additionalFields = additionalFields
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: RegistryCodingKey.self)
        id = try container.value(String.self, "id")
        name = try container.value(String.self, "name")
        family = try container.value(String.self, "family")
        version = try container.value(SemanticVersion.self, "version")
        manifestURL = try container.value(String.self, "manifestUrl")
        downloadURL = try container.value(String.self, "downloadUrl")
        sha256 = try container.value(String.self, "sha256")
        signature = try container.value(String.self, "signature")
        size = try container.value(Int.self, "size")
        minimumRuntimeAPI = try container.value(SemanticVersion.self, "minimumRuntimeApi")
        additionalFields = try container.additional(excluding: [
            "id", "name", "family", "version", "manifestUrl", "downloadUrl", "sha256",
            "signature", "size", "minimumRuntimeApi",
        ])
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: RegistryCodingKey.self)
        try container.additional(additionalFields)
        try container.value(id, "id")
        try container.value(name, "name")
        try container.value(family, "family")
        try container.value(version, "version")
        try container.value(manifestURL, "manifestUrl")
        try container.value(downloadURL, "downloadUrl")
        try container.value(sha256, "sha256")
        try container.value(signature, "signature")
        try container.value(size, "size")
        try container.value(minimumRuntimeAPI, "minimumRuntimeApi")
    }
}

public struct RegistryCatalog: Codable, Equatable, Sendable {
    public var schemaVersion: Int
    public var registryId: String
    public var name: String
    public var generatedAt: String
    public var keyId: String
    public var packs: [RegistryPack]
    public var additionalFields: [String: JSONValue]

    public init(
        schemaVersion: Int,
        registryId: String,
        name: String,
        generatedAt: String,
        keyId: String,
        packs: [RegistryPack],
        additionalFields: [String: JSONValue] = [:]
    ) {
        self.schemaVersion = schemaVersion; self.registryId = registryId; self.name = name
        self.generatedAt = generatedAt; self.keyId = keyId; self.packs = packs
        self.additionalFields = additionalFields
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: RegistryCodingKey.self)
        schemaVersion = try container.value(Int.self, "schemaVersion")
        guard schemaVersion == 1 else {
            throw DecodingError.dataCorrupted(.init(codingPath: decoder.codingPath, debugDescription: "Unsupported Registry schemaVersion \(schemaVersion)"))
        }
        registryId = try container.value(String.self, "registryId")
        name = try container.value(String.self, "name")
        generatedAt = try container.value(String.self, "generatedAt")
        keyId = try container.value(String.self, "keyId")
        packs = try container.value([RegistryPack].self, "packs")
        additionalFields = try container.additional(excluding: ["schemaVersion", "registryId", "name", "generatedAt", "keyId", "packs"])
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: RegistryCodingKey.self)
        try container.additional(additionalFields)
        try container.value(schemaVersion, "schemaVersion")
        try container.value(registryId, "registryId")
        try container.value(name, "name")
        try container.value(generatedAt, "generatedAt")
        try container.value(keyId, "keyId")
        try container.value(packs, "packs")
    }
}
