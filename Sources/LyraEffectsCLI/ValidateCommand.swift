import Foundation
import LyraPackKit

enum ValidateCommand {
    static func execute(arguments: [String], environment: CLIEnvironment) throws -> CLICommandResult {
        guard arguments.count == 1 else { throw CLIError.usage("Usage: lyra-effects validate <pack-directory>") }
        let diagnostics = try environment.validatePack(URL(filePath: arguments[0]))
        let data = JSONValue.object([
            "diagnostics": .array(diagnostics.map(\.jsonValue)),
            "errorCount": .number(Double(diagnostics.filter { $0.severity == .error }.count)),
        ])
        if diagnostics.contains(where: { $0.severity == .error }) {
            return .failure(command: "validate", message: "Pack validation failed", data: data)
        }
        return .success(command: "validate", data: data)
    }
}

private extension PackDiagnostic {
    var jsonValue: JSONValue {
        var object: [String: JSONValue] = [
            "severity": .string(severity.rawValue),
            "code": .string(code),
            "message": .string(message),
        ]
        if let path { object["path"] = .string(path) }
        return .object(object)
    }
}
