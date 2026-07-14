import Testing
@testable import LyraProjectKit

@Suite("CSS variable patcher")
struct CSSVariablePatcherTests {
    @Test func replacesOnlyTheRequestedValue() throws {
        let source = "/* keep */\n:root {\n  --lyra-size: 42px; /* comment */\n  --other: red;\n}\n"

        let result = try CSSVariablePatcher.patch(source, variable: "--lyra-size", value: "56px")

        #expect(result == "/* keep */\n:root {\n  --lyra-size: 56px; /* comment */\n  --other: red;\n}\n")
    }

    @Test func insertsMissingVariableIntoRootWithoutReformatting() throws {
        let source = ":root {\n  color: white;\n}\nbody { margin: 0; }\n"

        let result = try CSSVariablePatcher.patch(source, variable: "--lyra-accent", value: "#fff")

        #expect(result == ":root {\n  --lyra-accent: #fff;\n  color: white;\n}\nbody { margin: 0; }\n")
    }

    @Test func rejectsUnsafeVariableNamesAndValues() {
        #expect(throws: CSSVariablePatchError.self) {
            try CSSVariablePatcher.patch(":root {}", variable: "color", value: "red")
        }
        #expect(throws: CSSVariablePatchError.self) {
            try CSSVariablePatcher.patch(":root {}", variable: "--safe", value: "red; } body {")
        }
    }
}
