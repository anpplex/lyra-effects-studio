import Foundation
import Testing
@testable import LyraProjectKit

@Suite("Preview scenario")
struct PreviewScenarioTests {
    @Test func loadsDefaultScenarioWithoutRemoteAssets() throws {
        let scenario = try PreviewScenario.defaultSong()

        #expect(scenario.track.title == "Lyra Sample")
        #expect(scenario.lyrics.count >= 2)
        #expect(PreviewScenarioValidator().validate(scenario).isEmpty)
    }

    @Test func rejectsInvalidTimingAndRemoteAssets() {
        let scenario = PreviewScenario(
            schemaVersion: 1,
            id: "invalid",
            track: .init(title: "Invalid", artist: "Test", artwork: "https://example.com/art.jpg"),
            lyrics: [.init(startMilliseconds: 2000, endMilliseconds: 1000, text: "bad")],
            events: []
        )

        let codes = Set(PreviewScenarioValidator().validate(scenario).map(\.code))
        #expect(codes.contains("scenario.remoteAssetForbidden"))
        #expect(codes.contains("scenario.lyricTimingInvalid"))
    }
}
