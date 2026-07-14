import Foundation
import LyraPackKit

public enum ParameterControl: String, Codable, CaseIterable, Sendable {
    case color, number, length, font
    case enumeration = "enum"
    case toggle, asset
}

public struct ParameterBinding: Codable, Equatable, Sendable {
    public var cssVariable: String
    public init(cssVariable: String) { self.cssVariable = cssVariable }
}

public struct ParameterOption: Codable, Equatable, Sendable {
    public var value: String
    public var label: String
    public init(value: String, label: String) { self.value = value; self.label = label }
}

public struct ParameterDefinition: Codable, Equatable, Sendable {
    public var id: String
    public var label: String
    public var control: ParameterControl
    public var binding: ParameterBinding
    public var defaultValue: JSONValue
    public var unit: String?
    public var minimum: Double?
    public var maximum: Double?
    public var step: Double?
    public var options: [ParameterOption]?

    enum CodingKeys: String, CodingKey {
        case id, label, control, binding, unit, minimum, maximum, step, options
        case defaultValue = "default"
    }

    public init(
        id: String,
        label: String,
        control: ParameterControl,
        binding: ParameterBinding,
        defaultValue: JSONValue,
        unit: String? = nil,
        minimum: Double? = nil,
        maximum: Double? = nil,
        step: Double? = nil,
        options: [ParameterOption]? = nil
    ) {
        self.id = id; self.label = label; self.control = control; self.binding = binding
        self.defaultValue = defaultValue; self.unit = unit; self.minimum = minimum
        self.maximum = maximum; self.step = step; self.options = options
    }
}

public struct ParameterGroup: Codable, Equatable, Sendable {
    public var id: String
    public var label: String
    public var parameters: [ParameterDefinition]

    public init(id: String, label: String, parameters: [ParameterDefinition]) {
        self.id = id; self.label = label; self.parameters = parameters
    }
}

public struct ParameterSchema: Codable, Equatable, Sendable {
    public var schemaVersion: Int
    public var groups: [ParameterGroup]

    public init(schemaVersion: Int, groups: [ParameterGroup]) throws {
        guard schemaVersion == 1 else { throw ParameterSchemaError.unsupportedSchema(schemaVersion) }
        self.schemaVersion = schemaVersion
        self.groups = groups
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        try self.init(
            schemaVersion: container.decode(Int.self, forKey: .schemaVersion),
            groups: container.decode([ParameterGroup].self, forKey: .groups)
        )
    }

    enum CodingKeys: String, CodingKey { case schemaVersion, groups }
}

public enum ParameterSchemaError: Error, Equatable {
    case unsupportedSchema(Int)
}

public struct ParameterSchemaValidator {
    public init() {}

    public func validate(_ schema: ParameterSchema) -> [ProjectDiagnostic] {
        var diagnostics: [ProjectDiagnostic] = []
        var groupIDs: Set<String> = []
        var parameterIDs: Set<String> = []

        for group in schema.groups {
            if !groupIDs.insert(group.id).inserted {
                diagnostics.append(.init(code: "group.idDuplicate", path: group.id, message: "Parameter group ids must be unique"))
            }
            for parameter in group.parameters {
                if !parameterIDs.insert(parameter.id).inserted {
                    diagnostics.append(.init(code: "parameter.idDuplicate", path: parameter.id, message: "Parameter ids must be unique"))
                }
                if parameter.binding.cssVariable.wholeMatch(of: /--[A-Za-z_][A-Za-z0-9_-]*/) == nil {
                    diagnostics.append(.init(code: "binding.cssVariableInvalid", path: parameter.id, message: "CSS binding must be a custom property"))
                }
                if case let .number(value) = parameter.defaultValue,
                   (parameter.minimum.map { value < $0 } ?? false || parameter.maximum.map { value > $0 } ?? false) {
                    diagnostics.append(.init(code: "parameter.defaultOutOfRange", path: parameter.id, message: "Numeric default is outside declared bounds"))
                }
                if let minimum = parameter.minimum, let maximum = parameter.maximum, minimum > maximum {
                    diagnostics.append(.init(code: "parameter.boundsInvalid", path: parameter.id, message: "Minimum cannot exceed maximum"))
                }
            }
        }
        return diagnostics
    }
}
