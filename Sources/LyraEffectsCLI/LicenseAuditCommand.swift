import Foundation
import LyraPackKit
import LyraRegistryKit

enum LicenseAuditCommand {
    static func execute(arguments: [String]) throws -> CLICommandResult {
        guard arguments.count == 1 else { throw CLIError.usage("Usage: lyra-effects license-audit <registry-directory>") }
        let registryRoot = URL(filePath: arguments[0])
        let auditURL = registryRoot.appending(path: "license-audit.json")
        let report = try CanonicalJSON.decode(LicenseAuditReport.self, from: Data(contentsOf: auditURL))
        let diagnostics = try LicenseAuditValidator().validate(report, registryRoot: registryRoot)
        let data = JSONValue.object([
            "includedCount": .number(Double(report.included.count)),
            "excludedCount": .number(Double(report.excluded.count)),
            "diagnosticCount": .number(Double(diagnostics.count)),
        ])
        if let first = diagnostics.first {
            return .failure(command: "license-audit", message: "\(first.code): \(first.message)", data: data)
        }
        return .success(command: "license-audit", data: data)
    }
}
