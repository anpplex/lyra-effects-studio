import Testing
@testable import LyraProjectKit

@Test func projectContractStartsAtVersionOne() {
    #expect(LyraProjectKit.contractVersion == 1)
}
