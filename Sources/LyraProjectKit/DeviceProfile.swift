import Foundation
import LyraPackKit

public struct DeviceCanvas: Codable, Equatable, Sendable {
    public var physicalWidth: Int
    public var physicalHeight: Int
    public var cssWidth: Int
    public var cssHeight: Int
    public var rotation: Int
}

public struct DeviceInsets: Codable, Equatable, Sendable {
    public var top: Int
    public var right: Int
    public var bottom: Int
    public var left: Int
}

public struct DeviceMask: Codable, Equatable, Sendable {
    public var id: String
    public var shape: String
    public var x: Int
    public var y: Int
    public var width: Int
    public var height: Int
}

public struct DeviceADBProfile: Codable, Equatable, Sendable {
    public var userId: Int
    public var packageId: String
    public var externalPackRoot: String
}

public struct DeviceProfile: Codable, Equatable, Sendable {
    public var schemaVersion: Int
    public var id: String
    public var name: String
    public var canvas: DeviceCanvas
    public var safeArea: DeviceInsets
    public var masks: [DeviceMask]
    public var adb: DeviceADBProfile
    public var capabilities: [String]

    public static func builtInAvatrStarRing() throws -> Self {
        guard let url = Bundle.module.url(forResource: "avatr-star-ring-4032x284", withExtension: "json") else {
            throw DeviceProfileError.resourceMissing
        }
        let profile = try CanonicalJSON.decode(Self.self, from: Data(contentsOf: url))
        guard profile.schemaVersion == 1 else { throw DeviceProfileError.unsupportedSchema(profile.schemaVersion) }
        return profile
    }
}

public enum DeviceProfileError: Error, Equatable {
    case resourceMissing
    case unsupportedSchema(Int)
}

public struct DeviceProfileValidator {
    public init() {}

    public func validate(_ profile: DeviceProfile) -> [ProjectDiagnostic] {
        var result: [ProjectDiagnostic] = []
        let canvas = profile.canvas
        if [canvas.physicalWidth, canvas.physicalHeight, canvas.cssWidth, canvas.cssHeight].contains(where: { $0 <= 0 }) {
            result.append(.init(code: "profile.canvasInvalid", message: "Canvas dimensions must be positive"))
        }
        if profile.adb.userId < 0 || profile.adb.packageId.isEmpty {
            result.append(.init(code: "profile.adbInvalid", message: "ADB identity must be declarative and complete"))
        }
        if profile.adb.externalPackRoot.contains(";") || profile.adb.externalPackRoot.contains("\n") {
            result.append(.init(code: "profile.shellForbidden", message: "Device Profile cannot contain shell commands"))
        }
        return result
    }
}
