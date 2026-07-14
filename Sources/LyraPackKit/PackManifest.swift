import Foundation

private struct AnyCodingKey: CodingKey, Hashable {
    let stringValue: String
    let intValue: Int? = nil

    init(_ value: String) { stringValue = value }
    init?(stringValue: String) { self.init(stringValue) }
    init?(intValue: Int) { return nil }
}

private extension KeyedDecodingContainer where Key == AnyCodingKey {
    func decode<T: Decodable>(_ type: T.Type, key: String) throws -> T {
        try decode(type, forKey: AnyCodingKey(key))
    }

    func decodeIfPresent<T: Decodable>(_ type: T.Type, key: String) throws -> T? {
        try decodeIfPresent(type, forKey: AnyCodingKey(key))
    }

    func additionalFields(excluding known: Set<String>) throws -> [String: JSONValue] {
        try allKeys.reduce(into: [:]) { result, key in
            guard !known.contains(key.stringValue) else { return }
            result[key.stringValue] = try decode(JSONValue.self, forKey: key)
        }
    }
}

private extension KeyedEncodingContainer where Key == AnyCodingKey {
    mutating func encode<T: Encodable>(_ value: T, key: String) throws {
        try encode(value, forKey: AnyCodingKey(key))
    }

    mutating func encodeIfPresent<T: Encodable>(_ value: T?, key: String) throws {
        try encodeIfPresent(value, forKey: AnyCodingKey(key))
    }

    mutating func encodeAdditional(_ fields: [String: JSONValue]) throws {
        for key in fields.keys.sorted() {
            try encode(fields[key], forKey: AnyCodingKey(key))
        }
    }
}

public struct PackAuthor: Codable, Equatable, Sendable {
    public var name: String
    public var url: String?
    public var additionalFields: [String: JSONValue]

    public init(name: String, url: String? = nil, additionalFields: [String: JSONValue] = [:]) {
        self.name = name; self.url = url; self.additionalFields = additionalFields
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: AnyCodingKey.self)
        name = try container.decode(String.self, key: "name")
        url = try container.decodeIfPresent(String.self, key: "url")
        additionalFields = try container.additionalFields(excluding: ["name", "url"])
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: AnyCodingKey.self)
        try container.encodeAdditional(additionalFields)
        try container.encode(name, key: "name")
        try container.encodeIfPresent(url, key: "url")
    }
}

public struct PackLicense: Codable, Equatable, Sendable {
    public var spdx: String
    public var notice: String?
    public var additionalFields: [String: JSONValue]

    public init(spdx: String, notice: String? = nil, additionalFields: [String: JSONValue] = [:]) {
        self.spdx = spdx; self.notice = notice; self.additionalFields = additionalFields
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: AnyCodingKey.self)
        spdx = try container.decode(String.self, key: "spdx")
        notice = try container.decodeIfPresent(String.self, key: "notice")
        additionalFields = try container.additionalFields(excluding: ["spdx", "notice"])
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: AnyCodingKey.self)
        try container.encodeAdditional(additionalFields)
        try container.encode(spdx, key: "spdx")
        try container.encodeIfPresent(notice, key: "notice")
    }
}

public struct PackCompatibility: Codable, Equatable, Sendable {
    public var packSchema: VersionRange
    public var runtimeAPI: VersionRange
    public var bridgeAPI: VersionRange
    public var additionalFields: [String: JSONValue]

    public init(packSchema: VersionRange, runtimeAPI: VersionRange, bridgeAPI: VersionRange, additionalFields: [String: JSONValue] = [:]) {
        self.packSchema = packSchema; self.runtimeAPI = runtimeAPI; self.bridgeAPI = bridgeAPI; self.additionalFields = additionalFields
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: AnyCodingKey.self)
        packSchema = try container.decode(VersionRange.self, key: "packSchema")
        runtimeAPI = try container.decode(VersionRange.self, key: "runtimeApi")
        bridgeAPI = try container.decode(VersionRange.self, key: "bridgeApi")
        additionalFields = try container.additionalFields(excluding: ["packSchema", "runtimeApi", "bridgeApi"])
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: AnyCodingKey.self)
        try container.encodeAdditional(additionalFields)
        try container.encode(packSchema, key: "packSchema")
        try container.encode(runtimeAPI, key: "runtimeApi")
        try container.encode(bridgeAPI, key: "bridgeApi")
    }
}

public struct PackEntry: Codable, Equatable, Sendable {
    public var style: String?
    public var html: String?
    public var additionalFields: [String: JSONValue]

    public init(style: String? = nil, html: String? = nil, additionalFields: [String: JSONValue] = [:]) {
        self.style = style; self.html = html; self.additionalFields = additionalFields
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: AnyCodingKey.self)
        style = try container.decodeIfPresent(String.self, key: "style")
        html = try container.decodeIfPresent(String.self, key: "html")
        additionalFields = try container.additionalFields(excluding: ["style", "html"])
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: AnyCodingKey.self)
        try container.encodeAdditional(additionalFields)
        try container.encodeIfPresent(style, key: "style")
        try container.encodeIfPresent(html, key: "html")
    }
}

public struct PackManifest: Codable, Equatable, Sendable {
    public enum Kind: String, Codable, Equatable, Sendable { case theme, webEffect = "web-effect" }

    public var schemaVersion: Int
    public var id: String
    public var name: String
    public var version: SemanticVersion
    public var kind: Kind
    public var family: String
    public var author: PackAuthor
    public var license: PackLicense
    public var compatibility: PackCompatibility
    public var entry: PackEntry
    public var capabilities: [String]
    public var parameters: String?
    public var scenarios: [String]
    public var integrity: String?
    public var additionalFields: [String: JSONValue]

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: AnyCodingKey.self)
        schemaVersion = try container.decode(Int.self, key: "schemaVersion")
        guard schemaVersion == 1 else {
            throw DecodingError.dataCorrupted(.init(codingPath: decoder.codingPath, debugDescription: "Unsupported Pack schemaVersion \(schemaVersion)"))
        }
        id = try container.decode(String.self, key: "id")
        name = try container.decode(String.self, key: "name")
        version = try container.decode(SemanticVersion.self, key: "version")
        kind = try container.decode(Kind.self, key: "kind")
        family = try container.decode(String.self, key: "family")
        author = try container.decode(PackAuthor.self, key: "author")
        license = try container.decode(PackLicense.self, key: "license")
        compatibility = try container.decode(PackCompatibility.self, key: "compatibility")
        entry = try container.decode(PackEntry.self, key: "entry")
        capabilities = try container.decode([String].self, key: "capabilities")
        parameters = try container.decodeIfPresent(String.self, key: "parameters")
        scenarios = try container.decodeIfPresent([String].self, key: "scenarios") ?? []
        integrity = try container.decodeIfPresent(String.self, key: "integrity")
        additionalFields = try container.additionalFields(excluding: [
            "schemaVersion", "id", "name", "version", "kind", "family", "author", "license",
            "compatibility", "entry", "capabilities", "parameters", "scenarios", "integrity",
        ])
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: AnyCodingKey.self)
        try container.encodeAdditional(additionalFields)
        try container.encode(schemaVersion, key: "schemaVersion")
        try container.encode(id, key: "id")
        try container.encode(name, key: "name")
        try container.encode(version, key: "version")
        try container.encode(kind, key: "kind")
        try container.encode(family, key: "family")
        try container.encode(author, key: "author")
        try container.encode(license, key: "license")
        try container.encode(compatibility, key: "compatibility")
        try container.encode(entry, key: "entry")
        try container.encode(capabilities, key: "capabilities")
        try container.encodeIfPresent(parameters, key: "parameters")
        if !scenarios.isEmpty { try container.encode(scenarios, key: "scenarios") }
        try container.encodeIfPresent(integrity, key: "integrity")
    }
}
