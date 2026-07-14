import Foundation

public enum CSSVariablePatchError: Error, Equatable {
    case invalidVariable(String)
    case unsafeValue
    case rootRuleMissing
}

public enum CSSVariablePatcher {
    public static func patch(_ source: String, variable: String, value: String) throws -> String {
        guard variable.wholeMatch(of: /--[A-Za-z_][A-Za-z0-9_-]*/) != nil else {
            throw CSSVariablePatchError.invalidVariable(variable)
        }
        guard !value.contains(";"), !value.contains("{"), !value.contains("}"), !value.contains("\n"), !value.contains("\r") else {
            throw CSSVariablePatchError.unsafeValue
        }

        let escaped = NSRegularExpression.escapedPattern(for: variable)
        let declaration = try NSRegularExpression(pattern: "(\(escaped)\\s*:\\s*)([^;]*)(;)")
        let fullRange = NSRange(source.startIndex..., in: source)
        if let match = declaration.firstMatch(in: source, range: fullRange),
           let valueRange = Range(match.range(at: 2), in: source) {
            var result = source
            result.replaceSubrange(valueRange, with: value)
            return result
        }

        let root = try NSRegularExpression(pattern: ":root\\s*\\{")
        guard let match = root.firstMatch(in: source, range: fullRange),
              let insertionIndex = Range(match.range, in: source)?.upperBound else {
            throw CSSVariablePatchError.rootRuleMissing
        }
        var result = source
        result.insert(contentsOf: "\n  \(variable): \(value);", at: insertionIndex)
        return result
    }
}
