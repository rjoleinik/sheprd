import Foundation
import Testing
@testable import SheprdRemoteCore

/// The phone's Live Activity styles, and the daemon's step in words they
/// draw (`Fixtures/update.json`, written by `apns/tests/payload.rs`).
@Suite struct LiveActivityStyleTests {
    @Test func standardIsTheDefaultAndKeepGenericBecomesPrivate() {
        #expect(LiveActivityStyle.stored(nil) == .standard)
        #expect(LiveActivityStyle.stored(nil, legacyGeneric: true) == .private)
        // A choice made since wins over the old switch.
        #expect(LiveActivityStyle.stored("geek", legacyGeneric: true) == .geek)
        #expect(LiveActivityStyle.stored("fancy") == .standard)
        #expect(LiveActivityStyle.allCases.filter(\.asksGeneric) == [.private])
    }

    @Test func theDaemonsStepInWordsDecodes() throws {
        let url = try #require(Bundle.module.url(forResource: "update", withExtension: "json", subdirectory: "Fixtures"))
        let root = try #require(JSONSerialization.jsonObject(with: Data(contentsOf: url)) as? [String: Any])
        let aps = try #require(root["aps"] as? [String: Any])
        let data = try JSONSerialization.data(withJSONObject: try #require(aps["content-state"]))
        let state = try JSONDecoder().decode(ChatActivityState.self, from: data)
        #expect(state.activity == "Running tests")
        #expect(state.icon == .test)
        #expect(state.previousActivities == ["Reading lib.rs", "Editing lib.rs"])
        #expect(state.plainPrevious == ["Reading lib.rs", "Editing lib.rs"])
        #expect(state.counts == "3 steps \u{b7} 1 file")
        #expect(state.plainStep == "Running tests")
    }

    @Test func anOlderComputersStateFallsBackToItsSteps() throws {
        let json = #"{"phase":"working","step":"Edit lib.rs","startedAt":1,"previous":["Read lib.rs"],"icon":"hologram"}"#
        let state = try JSONDecoder().decode(ChatActivityState.self, from: Data(json.utf8))
        #expect(state.plainStep == "Edit lib.rs")
        #expect(state.plainPrevious == ["Read lib.rs"])
        #expect(state.icon == nil, "an unknown kind draws no symbol")
        #expect(state.counts == nil)
    }

    @Test func theNewFieldsRoundTrip() throws {
        let state = LiveActivitySample.state(at: Date(timeIntervalSince1970: 1_800_000_000))
        let decoded = try JSONDecoder().decode(ChatActivityState.self, from: JSONEncoder().encode(state))
        #expect(decoded == state)
    }
}
