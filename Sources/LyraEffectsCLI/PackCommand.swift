import Foundation
import LyraPackKit

enum PackCommand {
    static func execute(arguments: [String], environment: CLIEnvironment) throws -> CLICommandResult {
        guard arguments.count == 2 else { throw CLIError.usage("Usage: lyra-effects pack <pack-directory> <output.zip>") }
        let artifact = try environment.buildPack(URL(filePath: arguments[0]), URL(filePath: arguments[1]))
        return .success(command: "pack", data: .object([
            "path": .string(artifact.url.path),
            "sha256": .string(artifact.sha256),
            "byteCount": .number(Double(artifact.byteCount)),
        ]))
    }
}
