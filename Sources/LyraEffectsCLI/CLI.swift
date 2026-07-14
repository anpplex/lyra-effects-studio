import Foundation
import LyraPackKit

public struct CLIEnvironment {
    public var validatePack: (URL) throws -> [PackDiagnostic]
    public var buildPack: (URL, URL) throws -> PackArtifact

    public init(
        validatePack: @escaping (URL) throws -> [PackDiagnostic],
        buildPack: @escaping (URL, URL) throws -> PackArtifact
    ) {
        self.validatePack = validatePack
        self.buildPack = buildPack
    }

    public static var live: Self {
        Self(
            validatePack: { try PackValidator().validate(at: $0) },
            buildPack: { try PackArchiver().build(source: $0, destination: $1) }
        )
    }
}

public enum CLI {
    public static let version = "0.1.0-dev"

    @discardableResult
    public static func run(
        arguments: [String],
        environment: CLIEnvironment = .live,
        write: (String) -> Void = { print($0) }
    ) -> Int32 {
        if ["--version", "-V"].contains(arguments.first) {
            write(version)
            return 0
        }

        let result: CLICommandResult
        do {
            switch arguments.first {
            case "validate": result = try ValidateCommand.execute(arguments: Array(arguments.dropFirst()), environment: environment)
            case "pack": result = try PackCommand.execute(arguments: Array(arguments.dropFirst()), environment: environment)
            case "registry": result = try RegistryCommand.execute(arguments: Array(arguments.dropFirst()))
            case "license-audit": result = try LicenseAuditCommand.execute(arguments: Array(arguments.dropFirst()))
            default: throw CLIError.usage(Self.usage)
            }
        } catch let error as CLIError {
            result = .failure(command: "usage", code: 64, message: error.message)
        } catch {
            result = .failure(command: arguments.first ?? "unknown", code: 70, message: String(describing: error))
        }

        do {
            let data = try CanonicalJSON.encode(result.response)
            write(String(decoding: data.dropLast(), as: UTF8.self))
        } catch {
            write(#"{"command":"internal","message":"Unable to encode CLI response","ok":false}"#)
            return 70
        }
        return result.code
    }

    public static let usage = "Usage: lyra-effects <validate|pack|registry|license-audit> ..."
}

enum CLIError: Error {
    case usage(String)
    case invalidData(String)

    var message: String {
        switch self {
        case let .usage(message), let .invalidData(message): message
        }
    }
}

struct CLIResponse: Codable, Equatable {
    var command: String
    var ok: Bool
    var data: JSONValue?
    var message: String?
}

struct CLICommandResult {
    var code: Int32
    var response: CLIResponse

    static func success(command: String, data: JSONValue? = nil) -> Self {
        .init(code: 0, response: .init(command: command, ok: true, data: data, message: nil))
    }

    static func failure(command: String, code: Int32 = 65, message: String, data: JSONValue? = nil) -> Self {
        .init(code: code, response: .init(command: command, ok: false, data: data, message: message))
    }
}
