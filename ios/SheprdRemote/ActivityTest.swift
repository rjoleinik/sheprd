import ActivityKit
import Foundation
import SheprdRemoteCore
import UIKit

/// Settings' "Start Test Activity": this phone starts a Live Activity on
/// itself the way a computer does, with a sealed push-to-start through the
/// push relay and APNs (`SelfTestActivity`), and waits for ActivityKit to
/// start it. The activity then runs for `shown`, so it can be seen on the
/// Lock Screen and in the Dynamic Island, and ends itself. Says where it
/// stopped: this phone's settings, the push relay, APNs, or ActivityKit.
@MainActor
@Observable
final class ActivityTest {
    enum Status: Equatable {
        case sending
        /// APNs took it; no activity has started yet.
        case waiting
        /// It started, `seconds` after it was sent, and is still running.
        case running(seconds: Double)
        /// It started, ran its time, and ended.
        case ended(seconds: Double)
        case failed(String)
    }

    /// How long an accepted push may take before it counts as lost.
    static let patience: Duration = .seconds(30)
    /// How long iOS may take to hand over the push-to-start token.
    static let tokenPatience: Duration = .seconds(10)
    /// How long the test activity runs once it started. The app needs
    /// background time to end it, which iOS gives for about 30 seconds.
    static let shown: Duration = .seconds(20)

    private(set) var status: Status?
    private let tokens: ActivityTokens

    init(tokens: ActivityTokens) {
        self.tokens = tokens
    }

    var busy: Bool {
        switch status {
        case .sending, .waiting, .running: true
        default: false
        }
    }

    func start() async {
        guard !busy else { return }
        status = .sending
        guard PushRelaySetting.isOn else {
            status = .failed("This phone has no APNs key of its own, so it can only start an activity on itself through the push relay. Turn on Use the Push Relay above.")
            return
        }
        guard ActivityAuthorizationInfo().areActivitiesEnabled else {
            status = .failed("Live Activities are off for Sheprd. Turn them on in the iPhone's Settings > Sheprd, then try again.")
            return
        }
        // One test at a time on the Lock Screen.
        await Self.endTests()
        guard let token = await pushToStartToken() else { return }
        let relay = tokens.pushRelay
        guard let credential = await relay.usableCredential() else {
            status = .failed("The push relay has not let this phone in: \(relay.lastError ?? "turn the push relay off and on again.")")
            return
        }
        guard let handle = relay.handle(for: token, kind: .pushToStart, credential: credential) else {
            status = .failed("Could not make a push relay handle for this phone's push-to-start token.")
            return
        }
        let id = UUID().uuidString
        let sent = Date()
        let request: SelfTestPush.Request
        do {
            request = try SelfTestActivity.request(id: id, handle: handle, device: NotifyKey.own().publicKey, now: sent)
        } catch {
            status = .failed("Could not seal the test to this phone's own key: \(error.localizedDescription)")
            return
        }
        let outcome: PushRelayOutcome
        do {
            outcome = try await relay.push(request)
        } catch {
            status = .failed("Could not reach the push relay: \(error.localizedDescription)")
            return
        }
        if let failure = outcome.failure {
            if outcome.needsNewCredential { relay.rejected() }
            status = .failed(failure)
            return
        }
        status = .waiting
        guard let activity = await arrival(of: id) else {
            status = .failed("APNs took it, but no Live Activity started within 30 seconds. iOS limits how many a phone starts by push; check that Live Activities are on for Sheprd and try again later.")
            return
        }
        let seconds = Date().timeIntervalSince(sent)
        status = .running(seconds: seconds)
        // The activity shows once the app leaves the screen, when iOS
        // suspends it soon after; ending it needs the background time.
        let background = UIApplication.shared.beginBackgroundTask(withName: "Test activity")
        try? await Task.sleep(for: Self.shown)
        await Self.end(activity, from: SelfTestActivity.started(at: sent))
        status = .ended(seconds: seconds)
        UIApplication.shared.endBackgroundTask(background)
    }

    /// The push-to-start token, which iOS hands over after launch. `nil`,
    /// with the reason in `status`, when there is none.
    private func pushToStartToken() async -> String? {
        let deadline = ContinuousClock.now + Self.tokenPatience
        while ContinuousClock.now < deadline {
            if let token = tokens.pushToStart { return token }
            try? await Task.sleep(for: .milliseconds(100))
        }
        status = .failed("iOS has not given Sheprd a push-to-start token yet. Check that the phone is online, reopen the app, and try again.")
        return nil
    }

    /// Waits up to `patience` for the activity of test `id` to start, and
    /// answers its ActivityKit id.
    private func arrival(of id: String) async -> String? {
        let deadline = ContinuousClock.now + Self.patience
        while ContinuousClock.now < deadline {
            if let activity = Self.find(id) { return activity }
            try? await Task.sleep(for: .milliseconds(250))
        }
        return nil
    }

    /// The ActivityKit id of the running activity of test `id`.
    private nonisolated static func find(_ id: String) -> String? {
        Activity<ChatActivityAttributes>.activities.first { activity in
            testId(of: activity) == id
        }?.id
    }

    private nonisolated static func testId(of activity: Activity<ChatActivityAttributes>) -> String? {
        activity.attributes.opened(activity.content.state).flatMap { SelfTestActivity.id(of: $0.facts) }
    }

    /// Ends the test activity `id` showing that it worked, and leaves it on
    /// the Lock Screen for a minute. By id and off this actor, since an
    /// activity it holds cannot be sent to ActivityKit.
    private nonisolated static func end(_ id: String, from start: ChatActivityState) async {
        guard let activity = Activity<ChatActivityAttributes>.activities.first(where: { $0.id == id }) else { return }
        let finished = SelfTestActivity.finished(from: start)
        guard let own = try? NotifyKey.own(),
              let sealed = try? SelfTestActivity.seal(finished, to: own.publicKey)
        else {
            await activity.end(nil, dismissalPolicy: .immediate)
            return
        }
        await activity.end(.init(state: .sealed(sealed), staleDate: nil), dismissalPolicy: .after(.now + 60))
    }

    /// Ends every test activity still running, such as one the app had no
    /// time left to end.
    private nonisolated static func endTests() async {
        for activity in Activity<ChatActivityAttributes>.activities where testId(of: activity) != nil {
            await activity.end(nil, dismissalPolicy: .immediate)
        }
    }

    /// The line Settings shows under the button.
    static func text(_ status: Status) -> String {
        switch status {
        case .sending: "Sending…"
        case .waiting: "Sent. Waiting for the activity to start…"
        case let .running(seconds):
            "Started in \(format(seconds)) s. Go to the Home Screen or lock the phone to see it; it finishes in \(Int(shown.components.seconds)) seconds."
        case let .ended(seconds):
            "Started in \(format(seconds)) s, and finished. Live Activities work on this phone."
        case let .failed(why): why
        }
    }

    private static func format(_ seconds: Double) -> String {
        seconds.formatted(.number.precision(.fractionLength(1)))
    }
}
