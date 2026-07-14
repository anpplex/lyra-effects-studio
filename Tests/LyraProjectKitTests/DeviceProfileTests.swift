import Testing
@testable import LyraProjectKit

@Suite("Device profile")
struct DeviceProfileTests {
    @Test func loadsBuiltInAvatrStarRingProfile() throws {
        let profile = try DeviceProfile.builtInAvatrStarRing()

        #expect(profile.id == "com.avatr.cluster.4032x284")
        #expect(profile.canvas.physicalWidth == 4032)
        #expect(profile.canvas.physicalHeight == 284)
        #expect(profile.safeArea.left == 64)
        #expect(profile.capabilities.contains("devBridge"))
        #expect(DeviceProfileValidator().validate(profile).isEmpty)
    }
}
