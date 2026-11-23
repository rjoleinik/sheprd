import CryptoKit
import Foundation
import Testing
@testable import SheprdRemoteCore

/// A Live Activity the phone starts on itself through the push relay: the
/// push-to-start it posts, and what the widget opens of it.
@Suite struct SelfTestActivityTests {
    typealias Facts = ActivitySealable<ChatActivityFacts>
    typealias State = ActivitySealable<ChatActivityState>

    static let id = "6F1D2C3B-0A4E-4F5D-9C8B-7A6E5D4C3B2A"
    static let own = P256.KeyAgreement.PrivateKey()
    static let now = Date(timeIntervalSince1970: 1_800_000_000)

    static func aps(_ request: SelfTestPush.Request) throws -> [String: Any] {
        let payload = try #require(JSONSerialization.jsonObject(with: Data(request.payload.utf8)) as? [String: Any])
        return try #require(payload["aps"] as? [String: Any])
    }

    static func decode<T: Decodable>(_ type: T.Type, _ value: Any?) throws -> T {
        let value = try #require(value)
        return try JSONDecoder().decode(T.self, from: JSONSerialization.data(withJSONObject: value))
    }

    static func open(_ facts: Facts, _ state: State, with key: P256.KeyAgreement.PrivateKey = own) -> SealedChatActivity.Opened? {
        SelfTestActivity.open(attributes: facts, state: state, device: key.publicKey, agree: key.sharedSecretFromKeyAgreement)
    }

    @Test func itIsAPushToStartTheRelayTakesForAPushToStartHandle() throws {
        let request = try SelfTestActivity.request(id: Self.id, handle: "h", device: Self.own.publicKey, now: Self.now)
        #expect(request.pushType == "liveactivity")
        #expect(request.priority == "high")
        #expect(request.collapseId == nil)
        #expect(request.handle == "h")
        // The relay's request has no `collapse_id` key at all then.
        let body = try #require(JSONSerialization.jsonObject(with: JSONEncoder().encode(request)) as? [String: Any])
        #expect(Set(body.keys) == ["handle", "payload", "priority", "push_type"])
        let aps = try Self.aps(request)
        #expect(aps["event"] as? String == "start")
        #expect(aps["attributes-type"] as? String == "ChatActivityAttributes")
        #expect(aps["timestamp"] as? Int == 1_800_000_000)
        // ActivityKit starts one only with an alert; it is the cover.
        #expect(aps["alert"] as? [String: String] == ["title": "Sheprd", "body": "New activity"])
        // Nothing but the cover is readable.
        #expect((aps["attributes"] as? [String: Any])?.keys.sorted() == ["sealed"])
        #expect((aps["content-state"] as? [String: Any])?.keys.sorted() == ["sealed"])
    }

    @Test func theOwnKeyOpensItAsATestAndNoComputersKeyDoes() throws {
        let aps = try Self.aps(try SelfTestActivity.request(id: Self.id, handle: "h", device: Self.own.publicKey, now: Self.now))
        let facts = try Self.decode(Facts.self, aps["attributes"])
        let state = try Self.decode(State.self, aps["content-state"])
        let opened = try #require(Self.open(facts, state))
        #expect(opened.facts == SelfTestActivity.facts(id: Self.id))
        #expect(opened.state == SelfTestActivity.started(at: Self.now))
        #expect(SelfTestActivity.id(of: opened.facts) == Self.id)
        // The widget offers no Reply or Open on it, only on a chat's.
        #expect(SelfTestActivity.isTest(opened.facts))
        var chat = opened.facts
        chat.computerId = String(repeating: "AB", count: 32)
        #expect(!SelfTestActivity.isTest(chat))
        // It names no computer, so its token and link go nowhere.
        #expect(opened.facts.fingerprint == nil)
        #expect(Self.open(facts, state, with: P256.KeyAgreement.PrivateKey()) == nil)
        // Sealed to the own key, it is no computer's activity either.
        let key = SealedChatActivity.Key(
            computer: Fingerprint(String(repeating: "AB", count: 32))!,
            device: Self.own.publicKey,
            agree: Self.own.sharedSecretFromKeyAgreement
        )
        #expect(SealedChatActivity.open(attributes: facts, state: state, keys: [key], allowsPlain: false) == nil)
    }

    @Test func itsEndOpensToo() throws {
        let facts = Facts.sealed(try SelfTestActivity.seal(SelfTestActivity.facts(id: Self.id), to: Self.own.publicKey))
        let end = SelfTestActivity.finished(from: SelfTestActivity.started(at: Self.now))
        let state = State.sealed(try SelfTestActivity.seal(end, to: Self.own.publicKey))
        #expect(Self.open(facts, state)?.state.phase == .finished)
    }

    /// The own key opens only a test: not plain content, not one naming a
    /// computer or another pane, and not one carrying a question or buttons.
    @Test func onlyAnExactTestOpens() throws {
        let start = SelfTestActivity.started(at: Self.now)
        let sealedStart = State.sealed(try SelfTestActivity.seal(start, to: Self.own.publicKey))
        let test = SelfTestActivity.facts(id: Self.id)
        #expect(Self.open(.plain(test), sealedStart) == nil)
        #expect(Self.open(.sealed(try SelfTestActivity.seal(test, to: Self.own.publicKey)), .plain(start)) == nil)

        var computer = test
        computer.computerId = String(repeating: "AB", count: 32)
        var pane = test
        pane.pane = "w1-p3"
        var badId = test
        badId.pane = SelfTestActivity.panePrefix + "../x"
        for facts in [computer, pane, badId] {
            #expect(Self.open(.sealed(try SelfTestActivity.seal(facts, to: Self.own.publicKey)), sealedStart) == nil)
        }

        let sealedTest = Facts.sealed(try SelfTestActivity.seal(test, to: Self.own.publicKey))
        var question = start
        question.phase = .question
        question.question = "Allow?"
        var buttons = start
        buttons.permission = 7
        buttons.options = [.init(id: "a", name: "Allow", kind: .allowOnce)]
        for state in [question, buttons] {
            #expect(Self.open(sealedTest, .sealed(try SelfTestActivity.seal(state, to: Self.own.publicKey))) == nil)
        }
    }

    @Test func anInvalidIdIsRefused() {
        #expect(throws: SelfTestPush.Failure.id) {
            try SelfTestActivity.request(id: "../x", handle: "h", device: Self.own.publicKey, now: Self.now)
        }
    }
}
