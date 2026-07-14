import Foundation
import LyraPackKit

public struct ScenarioTrack: Codable, Equatable, Sendable {
    public var title: String
    public var artist: String
    public var album: String?
    public var artwork: String?

    public init(title: String, artist: String, album: String? = nil, artwork: String? = nil) {
        self.title = title; self.artist = artist; self.album = album; self.artwork = artwork
    }
}

public struct ScenarioLyric: Codable, Equatable, Sendable {
    public var startMilliseconds: Int
    public var endMilliseconds: Int
    public var text: String
    public var translation: String?
    public var romanization: String?

    public init(startMilliseconds: Int, endMilliseconds: Int, text: String, translation: String? = nil, romanization: String? = nil) {
        self.startMilliseconds = startMilliseconds; self.endMilliseconds = endMilliseconds
        self.text = text; self.translation = translation; self.romanization = romanization
    }
}

public struct ScenarioEvent: Codable, Equatable, Sendable {
    public var atMilliseconds: Int
    public var type: String
    public var value: JSONValue?
}

public struct PreviewScenario: Codable, Equatable, Sendable {
    public var schemaVersion: Int
    public var id: String
    public var track: ScenarioTrack
    public var lyrics: [ScenarioLyric]
    public var events: [ScenarioEvent]
    public var expectedDiagnostics: [String]

    public init(
        schemaVersion: Int,
        id: String,
        track: ScenarioTrack,
        lyrics: [ScenarioLyric],
        events: [ScenarioEvent],
        expectedDiagnostics: [String] = []
    ) {
        self.schemaVersion = schemaVersion; self.id = id; self.track = track
        self.lyrics = lyrics; self.events = events; self.expectedDiagnostics = expectedDiagnostics
    }

    public static func defaultSong() throws -> Self {
        guard let url = Bundle.module.url(forResource: "default-song", withExtension: "json") else {
            throw PreviewScenarioError.resourceMissing
        }
        let scenario = try CanonicalJSON.decode(Self.self, from: Data(contentsOf: url))
        guard scenario.schemaVersion == 1 else { throw PreviewScenarioError.unsupportedSchema(scenario.schemaVersion) }
        return scenario
    }
}

public enum PreviewScenarioError: Error, Equatable {
    case resourceMissing
    case unsupportedSchema(Int)
}

public struct PreviewScenarioValidator {
    public init() {}

    public func validate(_ scenario: PreviewScenario) -> [ProjectDiagnostic] {
        var result: [ProjectDiagnostic] = []
        if let artwork = scenario.track.artwork,
           artwork.lowercased().hasPrefix("http://") || artwork.lowercased().hasPrefix("https://") {
            result.append(.init(code: "scenario.remoteAssetForbidden", path: artwork, message: "Scenario assets must be local fixtures"))
        }
        for (index, lyric) in scenario.lyrics.enumerated()
        where lyric.startMilliseconds < 0 || lyric.endMilliseconds < lyric.startMilliseconds {
            result.append(.init(code: "scenario.lyricTimingInvalid", path: "lyrics[\(index)]", message: "Lyric timing must be non-negative and ordered"))
        }
        for (index, event) in scenario.events.enumerated() where event.atMilliseconds < 0 {
            result.append(.init(code: "scenario.eventTimingInvalid", path: "events[\(index)]", message: "Event timing must be non-negative"))
        }
        return result
    }
}
