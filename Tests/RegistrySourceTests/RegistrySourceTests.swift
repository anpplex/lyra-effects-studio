import Foundation
import Testing
import LyraPackKit
import LyraRegistryKit

@Suite("Registry source")
struct RegistrySourceTests {
    @Test func auditAccountsForEveryCurrentAdaptedTheme() throws {
        let report = try loadReport()

        #expect(report.included.count == 3)
        #expect(report.excluded.count == 15)
        #expect(Set(report.included.map(\.themeId) + report.excluded.map(\.themeId)).count == 18)
        #expect(Set(report.included.map(\.themeId)) == ["dynamic-background", "modern-player", "sustain"])
    }

    @Test func includedPacksHaveEvidenceValidManifestsAndMatchingHashes() throws {
        let root = packageRoot().appending(path: "Registry")
        let report = try loadReport()

        #expect(try LicenseAuditValidator().validate(report, registryRoot: root).isEmpty)
        for entry in report.included {
            let packRoot = root.appending(path: "Packs/\(entry.packId)")
            #expect(try PackValidator().validate(at: packRoot).isEmpty)
            #expect(try SHA256Digest.hex(fileAt: packRoot.appending(path: "theme/lyra.css")) == entry.sourceCSSSHA256)
        }
    }

    @Test func excludedThemesHaveNoPublishedPackDirectory() throws {
        let packsRoot = packageRoot().appending(path: "Registry/Packs")
        for entry in try loadReport().excluded {
            #expect(!FileManager.default.fileExists(atPath: packsRoot.appending(path: entry.packId).path))
            #expect(!entry.reason.isEmpty)
        }
    }

    private func loadReport() throws -> LicenseAuditReport {
        try CanonicalJSON.decode(
            LicenseAuditReport.self,
            from: Data(contentsOf: packageRoot().appending(path: "Registry/license-audit.json"))
        )
    }

    private func packageRoot() -> URL {
        URL(filePath: #filePath).deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
    }
}
