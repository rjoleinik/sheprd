import Foundation

/// How much a Live Activity shows, chosen on the phone in Settings >
/// Notifications. Every computer sends the same activity; the widget draws
/// it in the chosen style. Private also asks each computer to leave the
/// chat's name and steps out (`genericActivities`), as Keep Generic did.
public enum LiveActivityStyle: String, CaseIterable, Identifiable, Sendable {
    case `private`
    case minimal
    case standard
    case complete
    case geek

    public static let standardChoice: LiveActivityStyle = .standard

    public var id: String { rawValue }

    public var title: String {
        switch self {
        case .private: "Private"
        case .minimal: "Minimal"
        case .standard: "Standard"
        case .complete: "Complete"
        case .geek: "Geek"
        }
    }

    public var summary: String {
        switch self {
        case .private: "The agent and whether it works or waits. Computers leave out the chat's name and steps."
        case .minimal: "The chat, its computer, and working, waiting, or done."
        case .standard: "What the agent is doing, in words, and questions you can answer from the Lock Screen."
        case .complete: "Standard with the last steps, plan progress, and counts."
        case .geek: "The commands themselves, with the steps before them and the turn's counts."
        }
    }

    /// Whether computers should send this phone generic activities.
    public var asksGeneric: Bool { self == .private }

    /// The stored style, Standard when unset or unknown. `legacyGeneric`
    /// is the Keep Generic switch this setting replaced: on, an unset
    /// style reads as Private.
    public static func stored(_ raw: String?, legacyGeneric: Bool = false) -> LiveActivityStyle {
        if let raw { return LiveActivityStyle(rawValue: raw) ?? standardChoice }
        return legacyGeneric ? .private : standardChoice
    }
}

/// The kind of step a Live Activity shows, as the daemon names it
/// (`crates/sheprd-remote-control/src/apns/payload.rs`, `Icon`), mapped to
/// an SF Symbol.
public enum ActivityIcon: String, Codable, Hashable, Sendable {
    case agent, build, edit, git, github, plan, read, run, search, test, think, web

    public var symbol: String {
        switch self {
        case .agent: "person.2"
        case .build: "hammer"
        case .edit: "pencil"
        case .git: "arrow.triangle.branch"
        case .github: "checkmark.seal"
        case .plan: "list.bullet"
        case .read: "doc.text.magnifyingglass"
        case .run: "terminal"
        case .search: "magnifyingglass"
        case .test: "checkmark.circle"
        case .think: "brain"
        case .web: "globe"
        }
    }
}

/// What Settings previews each style with: a turn three steps into a
/// five-step plan, as a computer sends it.
public enum LiveActivitySample {
    public static func facts() -> ChatActivityFacts {
        ChatActivityFacts(
            computer: "studio",
            computerId: String(repeating: "0", count: 64),
            workspace: "sheprd",
            chatTitle: "Fix CI issues",
            agent: "Codex",
            pane: "preview"
        )
    }

    public static func state(at now: Date = Date()) -> ChatActivityState {
        ChatActivityState(
            phase: .working,
            step: "gh pr checks 1548 --repo sheprd-labs/sheprd",
            startedAt: now.addingTimeInterval(-17 * 60),
            planDone: 2,
            planTotal: 5,
            working: 2,
            waiting: 1,
            previous: ["git status --short", "gh api repos/sheprd-labs/sheprd/issues/1548/comments"],
            activity: "Checking pull request #1548",
            icon: .github,
            previousActivities: ["Looking at the changes", "Asking GitHub"],
            steps: 14,
            files: 3
        )
    }

    /// The same turn in `phase`: asking to push, or done.
    public static func state(_ phase: ChatActivityState.Phase, at now: Date = Date()) -> ChatActivityState {
        var state = state(at: now)
        state.phase = phase
        switch phase {
        case .working:
            break
        case .question:
            state.step = "git push origin fix/ci"
            state.activity = "Wants to push fix/ci"
            state.icon = .git
            state.question = "Allow git push origin fix/ci?"
            state.permission = 7
            state.options = [
                .init(id: "allow", name: "Allow once", kind: .allowOnce),
                .init(id: "reject", name: "Deny", kind: .rejectOnce),
            ]
        case .finished:
            state.summary = "Fixed the lint failure and pushed. Worked 23m"
            state.planDone = 5
            state.working = 1
            state.waiting = 0
        case .failed:
            state.summary = "Codex stopped"
        }
        return state
    }
}
