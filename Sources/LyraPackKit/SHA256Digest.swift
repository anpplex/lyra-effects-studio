import CryptoKit
import Foundation

public enum SHA256Digest {
    public static func hex(_ data: Data) -> String {
        SHA256.hash(data: data).map { String(format: "%02x", $0) }.joined()
    }

    public static func hex(fileAt url: URL) throws -> String {
        try hex(Data(contentsOf: url, options: [.mappedIfSafe]))
    }
}
