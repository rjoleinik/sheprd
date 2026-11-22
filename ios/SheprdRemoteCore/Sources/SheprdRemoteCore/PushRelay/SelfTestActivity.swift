import CryptoKit
import Foundation

/// A Live Activity this phone starts on itself through the push relay, so
/// push-to-start, APNs, and the widget can be checked with no computer
/// online. It is the push a computer sends to start a chat's activity
/// (`crates/sheprd-remote-control/src/apns/relayed.rs`), sealed under
/// `SealedChatActivity.info`, but to this phone's own key (`NotifyKey`'s
/// own key) and naming `SelfTestPush.sender` as the computer, which is no
/// fingerprint: its token goes to no computer, its link opens no chat, and
/// the widget opens it only as a test, which has no answer buttons.
public enum SelfTestActivity {
    /// What every test activity's pane starts with; the test's id follows.
    public static let panePrefix = "test-"

    /// The activity's fixed part for test `id`.
    public static func facts(id: String) -> ChatActivityFacts {
        ChatActivityFacts(
            computer: "This iPhone",
            computerId: SelfTestPush.sender,
            workspace: "Live Activity test",
            chatTitle: "",
            agent: "Sheprd",
            pane: panePrefix + id
        )
    }

    /// What the activity says when it starts at `now`.
    public static func started(at now: Date) -> ChatActivityState {
        ChatActivityState(phase: .working, step: "Started through the push relay", startedAt: now, working: 1)
    }

    /// What it says when the test ends it.
    public static func finished(from start: ChatActivityState) -> ChatActivityState {
        var state = start
        state.phase = .finished
        state.step = "Live Activities work on this phone."
        state.summary = "Live Activities work on this phone."
        state.working = 0
        return state
    }

    /// The push relay request that starts test `id` through `handle` (one
    /// made for this phone's push-to-start token) at `now`, sealed to
    /// `device`.
    public static func request(id: String, handle: String, device: P256.KeyAgreement.PublicKey, now: Date) throws -> SelfTestPush.Request {
        guard SelfTestPush.validId(id) else { throw SelfTestPush.Failure.id }
        let aps = Aps(
            timestamp: Int(now.timeIntervalSince1970),
            attributes: .sealed(try seal(facts(id: id), to: device)),
            contentState: .sealed(try seal(started(at: now), to: device))
        )
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.sortedKeys, .withoutEscapingSlashes]
        let payload = try encoder.encode(Outer(aps: aps))
        return SelfTestPush.Request(
            collapseId: nil,
            handle: handle,
            payload: String(decoding: payload, as: UTF8.self),
            // A test is waited for; a computer's start of a question is
            // high too.
            priority: "high",
            pushType: "liveactivity"
        )
    }

    /// `value` sealed to `device` as a computer seals Live Activity content.
    public static func seal(_ value: some Encodable, to device: P256.KeyAgreement.PublicKey) throws -> String {
        let sealed = try NotificationSeal.seal(JSONEncoder().encode(value), to: device, info: SealedChatActivity.info)
        return Base64URL.encode(sealed)
    }

    /// The test's id, when `facts` are a test activity's.
    public static func id(of facts: ChatActivityFacts) -> String? {
        guard facts.computerId == SelfTestPush.sender, facts.pane.hasPrefix(panePrefix) else { return nil }
        let id = String(facts.pane.dropFirst(panePrefix.count))
        return SelfTestPush.validId(id) ? id : nil
    }

    /// Whether `facts` are a test activity's, which opens no chat, so the
    /// widget offers no Reply or Open.
    public static func isTest(_ facts: ChatActivityFacts) -> Bool {
        id(of: facts) != nil
    }

    /// Opens a test activity with this phone's own key: both parts must be
    /// sealed to it, name no computer, and carry no question or buttons.
    /// `nil` for anything else, which then shows the cover.
    public static func open(
        attributes: ActivitySealable<ChatActivityFacts>,
        state: ActivitySealable<ChatActivityState>,
        device: P256.KeyAgreement.PublicKey,
        agree: (P256.KeyAgreement.PublicKey) throws -> SharedSecret
    ) -> SealedChatActivity.Opened? {
        guard case let .sealed(facts) = attributes, case let .sealed(state) = state,
              let facts: ChatActivityFacts = open(facts, device: device, agree: agree),
              id(of: facts) != nil,
              let state: ChatActivityState = open(state, device: device, agree: agree),
              state.question == nil, state.permission == nil, state.options.isEmpty
        else { return nil }
        return SealedChatActivity.Opened(facts: facts, state: state)
    }

    private static func open<Value: Decodable>(
        _ text: String,
        device: P256.KeyAgreement.PublicKey,
        agree: (P256.KeyAgreement.PublicKey) throws -> SharedSecret
    ) -> Value? {
        guard let sealed = Base64URL.decode(text),
              let plaintext = try? NotificationSeal.open(sealed, device: device, info: SealedChatActivity.info, agree: agree)
        else { return nil }
        return try? JSONDecoder().decode(Value.self, from: plaintext)
    }

    /// The push-to-start payload, as `relayed.rs` writes a start, with the
    /// cover alert ActivityKit requires to start one.
    struct Outer: Encodable {
        let aps: Aps
    }

    struct Aps: Encodable {
        let timestamp: Int
        let event = "start"
        let attributesType = "ChatActivityAttributes"
        let attributes: ActivitySealable<ChatActivityFacts>
        let contentState: ActivitySealable<ChatActivityState>
        let alert = SelfTestPush.Cover()

        enum CodingKeys: String, CodingKey {
            case timestamp, event, attributes, alert
            case attributesType = "attributes-type"
            case contentState = "content-state"
        }
    }
}
