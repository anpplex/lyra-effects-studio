import Foundation
import Testing
@testable import LyraProjectKit

@Suite("Parameter schema")
struct ParameterSchemaTests {
    @Test func supportsV1ControlsAndValidatesBounds() throws {
        let schema = try ParameterSchema(
            schemaVersion: 1,
            groups: [
                .init(id: "appearance", label: "Appearance", parameters: [
                    .init(id: "accent", label: "Accent", control: .color, binding: .init(cssVariable: "--lyra-accent"), defaultValue: .string("#8b5cf6")),
                    .init(id: "size", label: "Size", control: .length, binding: .init(cssVariable: "--lyra-size"), defaultValue: .number(42), unit: "px", minimum: 20, maximum: 96, step: 1),
                    .init(id: "motion", label: "Motion", control: .toggle, binding: .init(cssVariable: "--lyra-motion"), defaultValue: .bool(true)),
                ])
            ]
        )

        #expect(ParameterSchemaValidator().validate(schema).isEmpty)
    }

    @Test func rejectsDuplicateIdsBadBindingsAndOutOfRangeDefaults() throws {
        let parameter = ParameterDefinition(
            id: "size", label: "Size", control: .number,
            binding: .init(cssVariable: "not-a-variable"), defaultValue: .number(120),
            minimum: 20, maximum: 96
        )
        let schema = try ParameterSchema(schemaVersion: 1, groups: [
            .init(id: "one", label: "One", parameters: [parameter, parameter]),
        ])

        let codes = Set(ParameterSchemaValidator().validate(schema).map(\.code))
        #expect(codes.contains("parameter.idDuplicate"))
        #expect(codes.contains("binding.cssVariableInvalid"))
        #expect(codes.contains("parameter.defaultOutOfRange"))
    }
}
