import Testing
@testable import LyraEffectsCLI

@Test func versionCommandIsStable() {
    var output: [String] = []
    let code = CLI.run(arguments: ["--version"], write: { output.append($0) })

    #expect(code == 0)
    #expect(output == ["0.1.0-dev"])
}
