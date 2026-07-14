import Testing
@testable import LyraPackKit

@Test func packContractStartsAtVersionOne() {
    #expect(LyraPackKit.contractVersion == 1)
}
