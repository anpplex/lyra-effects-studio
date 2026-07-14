import Foundation
import LyraPackKit

enum LicenseAuditCommand {
    static func execute(arguments: [String]) throws -> CLICommandResult {
        guard arguments.count == 1 else { throw CLIError.usage("Usage: lyra-effects license-audit <registry-directory>") }
        let auditURL = URL(filePath: arguments[0]).appending(path: "license-audit.json")
        let value = try CanonicalJSON.decode(JSONValue.self, from: Data(contentsOf: auditURL))
        guard case let .array(included)? = value["included"],
              case let .array(excluded)? = value["excluded"] else {
            throw CLIError.invalidData("license-audit.json must contain included and excluded arrays")
        }
        return .success(command: "license-audit", data: .object([
            "includedCount": .number(Double(included.count)),
            "excludedCount": .number(Double(excluded.count)),
        ]))
    }
}
