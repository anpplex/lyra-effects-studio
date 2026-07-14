import Foundation

public struct ProjectDiagnostic: Codable, Equatable, Sendable {
    public enum Severity: String, Codable, Sendable { case error, warning }
    public let severity: Severity
    public let code: String
    public let path: String?
    public let message: String

    public init(severity: Severity = .error, code: String, path: String? = nil, message: String) {
        self.severity = severity; self.code = code; self.path = path; self.message = message
    }
}
