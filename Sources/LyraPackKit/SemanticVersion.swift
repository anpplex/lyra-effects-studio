import Foundation

public enum SemanticVersionError: Error, Equatable, CustomStringConvertible {
    case invalid(String)

    public var description: String {
        switch self {
        case let .invalid(source): "Invalid semantic version: \(source)"
        }
    }
}

public struct SemanticVersion: Hashable, Sendable, Comparable, Codable, CustomStringConvertible {
    public let major: Int
    public let minor: Int
    public let patch: Int
    public let prerelease: [String]
    public let build: [String]

    public init(_ source: String) throws {
        let pattern = #"^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(?:-((?:0|[1-9][0-9]*|[0-9]*[A-Za-z-][0-9A-Za-z-]*)(?:\.(?:0|[1-9][0-9]*|[0-9]*[A-Za-z-][0-9A-Za-z-]*))*))?(?:\+([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$"#
        guard let expression = try? NSRegularExpression(pattern: pattern),
              let match = expression.firstMatch(in: source, range: NSRange(source.startIndex..., in: source)),
              match.range == NSRange(source.startIndex..., in: source),
              let major = Int(Self.capture(1, match: match, source: source) ?? ""),
              let minor = Int(Self.capture(2, match: match, source: source) ?? ""),
              let patch = Int(Self.capture(3, match: match, source: source) ?? "")
        else { throw SemanticVersionError.invalid(source) }

        self.major = major
        self.minor = minor
        self.patch = patch
        self.prerelease = Self.capture(4, match: match, source: source)?.split(separator: ".").map(String.init) ?? []
        self.build = Self.capture(5, match: match, source: source)?.split(separator: ".").map(String.init) ?? []
    }

    public var description: String {
        var result = "\(major).\(minor).\(patch)"
        if !prerelease.isEmpty { result += "-" + prerelease.joined(separator: ".") }
        if !build.isEmpty { result += "+" + build.joined(separator: ".") }
        return result
    }

    public static func < (lhs: Self, rhs: Self) -> Bool {
        if lhs.major != rhs.major { return lhs.major < rhs.major }
        if lhs.minor != rhs.minor { return lhs.minor < rhs.minor }
        if lhs.patch != rhs.patch { return lhs.patch < rhs.patch }
        if lhs.prerelease.isEmpty { return !rhs.prerelease.isEmpty ? false : false }
        if rhs.prerelease.isEmpty { return true }

        for (left, right) in zip(lhs.prerelease, rhs.prerelease) where left != right {
            switch (Int(left), Int(right)) {
            case let (l?, r?): return l < r
            case (_?, nil): return true
            case (nil, _?): return false
            case (nil, nil): return left < right
            }
        }
        return lhs.prerelease.count < rhs.prerelease.count
    }

    public init(from decoder: Decoder) throws {
        let source = try decoder.singleValueContainer().decode(String.self)
        try self.init(source)
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.singleValueContainer()
        try container.encode(description)
    }

    private static func capture(_ index: Int, match: NSTextCheckingResult, source: String) -> String? {
        let range = match.range(at: index)
        guard range.location != NSNotFound, let swiftRange = Range(range, in: source) else { return nil }
        return String(source[swiftRange])
    }
}

public struct VersionRange: Hashable, Sendable, Codable, CustomStringConvertible {
    private enum Operator: String, Hashable, Sendable { case greaterEqual = ">=", greater = ">", lessEqual = "<=", less = "<", equal = "=" }
    private struct Constraint: Hashable, Sendable { let operation: Operator; let version: SemanticVersion }

    public let description: String
    private let constraints: [Constraint]

    public init(_ source: String) throws {
        let tokens = source.split(whereSeparator: \.isWhitespace).map(String.init)
        guard !tokens.isEmpty else { throw SemanticVersionError.invalid(source) }
        self.constraints = try tokens.map { token in
            let operation = [Operator.greaterEqual, .lessEqual, .greater, .less, .equal]
                .first(where: { token.hasPrefix($0.rawValue) }) ?? .equal
            let rawVersion = operation == .equal && !token.hasPrefix("=") ? token : String(token.dropFirst(operation.rawValue.count))
            let componentCount = rawVersion.split(separator: ".", omittingEmptySubsequences: false).count
            guard (1...3).contains(componentCount) else { throw SemanticVersionError.invalid(source) }
            let normalized = rawVersion + String(repeating: ".0", count: 3 - componentCount)
            return Constraint(operation: operation, version: try SemanticVersion(normalized))
        }
        self.description = source
    }

    public func contains(_ version: SemanticVersion) -> Bool {
        constraints.allSatisfy { constraint in
            switch constraint.operation {
            case .greaterEqual: version >= constraint.version
            case .greater: version > constraint.version
            case .lessEqual: version <= constraint.version
            case .less: version < constraint.version
            case .equal: version == constraint.version
            }
        }
    }

    public init(from decoder: Decoder) throws {
        try self.init(try decoder.singleValueContainer().decode(String.self))
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.singleValueContainer()
        try container.encode(description)
    }
}
