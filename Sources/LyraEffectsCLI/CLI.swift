import Foundation

public enum CLI {
    public static let version = "0.1.0-dev"

    @discardableResult
    public static func run(
        arguments: [String],
        write: (String) -> Void = { print($0) }
    ) -> Int32 {
        switch arguments.first {
        case "--version", "-V":
            write(version)
            return 0
        default:
            write("Lyra Effects Studio CLI \(version)")
            write("Usage: lyra-effects --version")
            return arguments.isEmpty ? 0 : 64
        }
    }
}
