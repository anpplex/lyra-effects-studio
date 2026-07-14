import Testing
@testable import LyraRegistryKit

@Test func registryContractStartsAtVersionOne() {
    #expect(LyraRegistryKit.contractVersion == 1)
}
