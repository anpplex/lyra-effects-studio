import Testing
@testable import LyraPackKit

@Suite("Semantic version")
struct SemanticVersionTests {
    @Test func parsesAndOrdersSemVer() throws {
        let prerelease = try SemanticVersion("1.2.3-alpha.1+build.7")
        let release = try SemanticVersion("1.2.3")

        #expect(prerelease.description == "1.2.3-alpha.1+build.7")
        #expect(prerelease < release)
        #expect(try SemanticVersion("2.0.0") > release)
    }

    @Test(arguments: ["1", "1.2", "01.2.3", "1.02.3", "v1.2.3", "1.2.3-"])
    func rejectsNonSemVer(_ source: String) {
        #expect(throws: SemanticVersionError.self) {
            try SemanticVersion(source)
        }
    }

    @Test func rangeAcceptsSchemaAndRuntimeForms() throws {
        let schema = try VersionRange(">=1 <2")
        let runtime = try VersionRange(">=1.0.0 <2.0.0")

        #expect(schema.contains(try SemanticVersion("1.9.0")))
        #expect(!schema.contains(try SemanticVersion("2.0.0")))
        #expect(runtime.contains(try SemanticVersion("1.0.0")))
        #expect(!runtime.contains(try SemanticVersion("0.9.9")))
    }
}
