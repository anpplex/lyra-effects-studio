import Foundation
import LyraPackKit

public struct LicenseAuditIncluded: Codable, Equatable, Sendable {
    public var themeId: String
    public var packId: String
    public var version: SemanticVersion
    public var sourceRepository: String
    public var sourceRevision: String
    public var sourceURL: String
    public var licenseSPDX: String
    public var licenseEvidenceURL: String
    public var sourceCSSPath: String
    public var sourceCSSSHA256: String
}

public struct LicenseAuditExcluded: Codable, Equatable, Sendable {
    public var themeId: String
    public var packId: String
    public var version: SemanticVersion
    public var sourceRepository: String
    public var sourceRevision: String
    public var reasonCode: String
    public var reason: String
}

public struct LicenseAuditReport: Codable, Equatable, Sendable {
    public var schemaVersion: Int
    public var generatedAt: String
    public var sourceCatalogPath: String
    public var sourceRevision: String
    public var included: [LicenseAuditIncluded]
    public var excluded: [LicenseAuditExcluded]
}

public struct LicenseAuditDiagnostic: Codable, Equatable, Sendable {
    public var code: String
    public var themeId: String?
    public var message: String

    public init(code: String, themeId: String? = nil, message: String) {
        self.code = code; self.themeId = themeId; self.message = message
    }
}

public struct LicenseAuditValidator {
    private let fileManager: FileManager

    public init(fileManager: FileManager = .default) {
        self.fileManager = fileManager
    }

    public func validate(_ report: LicenseAuditReport, registryRoot: URL) throws -> [LicenseAuditDiagnostic] {
        var result: [LicenseAuditDiagnostic] = []
        if report.schemaVersion != 1 {
            result.append(.init(code: "audit.schemaUnsupported", message: "Only license audit schemaVersion 1 is supported"))
        }

        var themeIDs: Set<String> = []
        var packIDs: Set<String> = []
        for entry in report.included {
            if !themeIDs.insert(entry.themeId).inserted {
                result.append(.init(code: "audit.themeDuplicate", themeId: entry.themeId, message: "Theme appears more than once"))
            }
            if !packIDs.insert(entry.packId).inserted {
                result.append(.init(code: "audit.packDuplicate", themeId: entry.themeId, message: "Pack id appears more than once"))
            }
            if ["", "NOASSERTION", "MISSING"].contains(entry.licenseSPDX.uppercased()) {
                result.append(.init(code: "audit.licenseInvalid", themeId: entry.themeId, message: "Included Pack requires an identified SPDX license"))
            }
            if entry.sourceRevision.wholeMatch(of: /[a-f0-9]{40}/) == nil {
                result.append(.init(code: "audit.revisionInvalid", themeId: entry.themeId, message: "Source revision must be an immutable 40-character commit SHA"))
            }
            if entry.sourceCSSSHA256.wholeMatch(of: /[a-f0-9]{64}/) == nil {
                result.append(.init(code: "audit.checksumInvalid", themeId: entry.themeId, message: "Source CSS checksum must be lowercase SHA-256"))
            }
            if !entry.sourceURL.contains(entry.sourceRevision) || !entry.licenseEvidenceURL.contains(entry.sourceRevision) {
                result.append(.init(code: "audit.evidenceMutable", themeId: entry.themeId, message: "Source and license evidence URLs must contain the immutable revision"))
            }

            let packRoot = registryRoot.appending(path: "Packs/\(entry.packId)")
            for required in ["lyra-pack.json", "theme/lyra.css", "LICENSE", "NOTICE", "upstream.json"]
            where !fileManager.fileExists(atPath: packRoot.appending(path: required).path) {
                result.append(.init(code: "audit.packFileMissing", themeId: entry.themeId, message: "Missing \(required)"))
            }
            let cssURL = packRoot.appending(path: "theme/lyra.css")
            if fileManager.fileExists(atPath: cssURL.path), try SHA256Digest.hex(fileAt: cssURL) != entry.sourceCSSSHA256 {
                result.append(.init(code: "audit.cssHashMismatch", themeId: entry.themeId, message: "Adapted CSS differs from the audited source"))
            }
            let manifestURL = packRoot.appending(path: "lyra-pack.json")
            if fileManager.fileExists(atPath: manifestURL.path),
               let manifest = try? CanonicalJSON.decode(PackManifest.self, from: Data(contentsOf: manifestURL)),
               manifest.id != entry.packId || manifest.version != entry.version {
                result.append(.init(code: "audit.manifestMismatch", themeId: entry.themeId, message: "Manifest identity differs from audit evidence"))
            }
        }

        for entry in report.excluded {
            if !themeIDs.insert(entry.themeId).inserted {
                result.append(.init(code: "audit.themeDuplicate", themeId: entry.themeId, message: "Theme appears more than once"))
            }
            let packRoot = registryRoot.appending(path: "Packs/\(entry.packId)")
            if fileManager.fileExists(atPath: packRoot.path) {
                result.append(.init(code: "audit.excludedPackPublished", themeId: entry.themeId, message: "Excluded Theme has a publishable Pack directory"))
            }
            if entry.reasonCode.isEmpty || entry.reason.isEmpty {
                result.append(.init(code: "audit.exclusionReasonMissing", themeId: entry.themeId, message: "Excluded Theme requires a structured reason"))
            }
        }
        return result
    }
}
