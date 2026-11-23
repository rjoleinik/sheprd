import SheprdRemoteCore
import SwiftUI

/// Settings > Notifications > Live Activities > Style: each style with a
/// preview drawn by the widget's own views, a phase to preview them in, the
/// Dynamic Island in the chosen style, and the test, which plays in it.
struct LiveActivityStyleView: View {
    @Environment(AppModel.self) private var model
    @AppStorage(LiveActivityStyleSetting.key, store: LiveActivityStyleSetting.store)
    private var stored = LiveActivityStyle.standardChoice.rawValue
    @State private var phase: ChatActivityState.Phase = .working

    private var style: LiveActivityStyle { LiveActivityStyle.stored(stored) }

    var body: some View {
        List {
            Section {
                Picker("Preview", selection: $phase) {
                    Text("Working").tag(ChatActivityState.Phase.working)
                    Text("Waiting").tag(ChatActivityState.Phase.question)
                    Text("Finished").tag(ChatActivityState.Phase.finished)
                }
                .pickerStyle(.segmented)
                .listRowBackground(Color.clear)
                .listRowInsets(EdgeInsets())
            }
            ForEach(LiveActivityStyle.allCases) { option in
                Section {
                    Button {
                        choose(option)
                    } label: {
                        StyleRow(style: option, chosen: option == style, phase: phase)
                    }
                    .buttonStyle(.plain)
                    .accessibilityAddTraits(option == style ? .isSelected : [])
                }
            }
            Section {
                IslandPreview(style: style, phase: phase)
                    .listRowBackground(Color.clear)
                    .listRowInsets(EdgeInsets())
            } header: {
                Text("Dynamic Island")
            }
            Section {
                TestActivityRow()
            } footer: {
                Text("The test activity plays in the \(style.title) style: it starts, moves on a step, and finishes, so you can watch it on the Lock Screen and in the Dynamic Island.")
            }
        }
        .navigationTitle("Live Activity Style")
        .navigationBarTitleDisplayMode(.inline)
    }

    private func choose(_ new: LiveActivityStyle) {
        let old = style
        guard new != old else { return }
        stored = new.rawValue
        Task { await model.tokens.styleChanged(from: old, to: new) }
    }
}

/// One style: its name and what it shows, then the preview.
private struct StyleRow: View {
    let style: LiveActivityStyle
    let chosen: Bool
    let phase: ChatActivityState.Phase

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(alignment: .top) {
                VStack(alignment: .leading, spacing: 3) {
                    Text(style.title).font(.body.weight(.semibold))
                    Text(style.summary).font(.footnote).foregroundStyle(.secondary)
                }
                Spacer(minLength: 8)
                Image(systemName: chosen ? "checkmark.circle.fill" : "circle")
                    .font(.title3)
                    .foregroundStyle(chosen ? Color.accentColor : Color.secondary)
            }
            LockScreenPreview(style: style, phase: phase)
        }
        .padding(.vertical, 4)
        .contentShape(Rectangle())
    }
}

/// The lock screen card on a lock-screen-like backdrop.
struct LockScreenPreview: View {
    let style: LiveActivityStyle
    let phase: ChatActivityState.Phase

    var body: some View {
        let state = LiveActivitySample.state(phase)
        var facts = LiveActivitySample.facts()
        if style.asksGeneric {
            // What a computer sends for Private: no title, the computer as
            // the place, and no steps.
            facts.chatTitle = ""
            facts.workspace = facts.computer
        }
        return LockScreenView(facts: facts, state: style.asksGeneric ? Self.generic(state) : state, style: style)
            .background(Color.black.opacity(0.55))
            .clipShape(RoundedRectangle(cornerRadius: 22, style: .continuous))
            .padding(8)
            .background(
                LinearGradient(colors: [Color(red: 0.45, green: 0.27, blue: 0.17), Color(red: 0.24, green: 0.17, blue: 0.22)], startPoint: .top, endPoint: .bottom),
                in: RoundedRectangle(cornerRadius: 28, style: .continuous)
            )
            .foregroundStyle(.white)
            .environment(\.colorScheme, .dark)
            // The answer buttons run real intents; a preview answers nothing.
            .allowsHitTesting(false)
    }

    /// `state` as a computer redacts it for Private.
    static func generic(_ state: ChatActivityState) -> ChatActivityState {
        var state = state
        state.step = state.phase == .working ? "Working" : state.phase == .question ? "Needs an answer" : "Done"
        state.activity = nil
        state.icon = nil
        state.previous = []
        state.previousActivities = []
        state.steps = nil
        state.files = nil
        state.question = nil
        state.permission = nil
        state.options = []
        state.summary = nil
        return state
    }
}

/// The compact island above the expanded one, in `style`.
private struct IslandPreview: View {
    let style: LiveActivityStyle
    let phase: ChatActivityState.Phase

    var body: some View {
        let state = style.asksGeneric
            ? LockScreenPreview.generic(LiveActivitySample.state(phase))
            : LiveActivitySample.state(phase)
        var facts = LiveActivitySample.facts()
        if style.asksGeneric {
            facts.chatTitle = ""
            facts.workspace = facts.computer
        }
        return VStack(spacing: 14) {
            HStack {
                IslandParts.compactLeading(state, facts: facts, style: style)
                Spacer()
                IslandParts.compactTrailing(state)
            }
            .padding(.horizontal, 14)
            .frame(width: 230, height: 36)
            .background(.black, in: Capsule())
            VStack(alignment: .leading, spacing: 10) {
                HStack(alignment: .top, spacing: 10) {
                    AgentBadge(agent: facts.agent, size: 28)
                    VStack(alignment: .leading, spacing: 2) {
                        Text(IslandParts.title(state, facts: facts, style: style))
                            .font(.subheadline.weight(.semibold))
                            .lineLimit(1)
                        Text(IslandParts.subtitle(facts: facts, style: style))
                            .font(.caption)
                            .foregroundStyle(.secondary)
                            .lineLimit(1)
                    }
                    Spacer()
                    if state.phase == .working {
                        Elapsed(since: state.startedAt).font(.caption.weight(.semibold))
                    } else {
                        PhaseMark(phase: state.phase)
                    }
                }
                IslandParts.bottom(state, facts: facts, style: style)
            }
            .padding(18)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(.black, in: RoundedRectangle(cornerRadius: 40, style: .continuous))
        }
        .padding(.vertical, 14)
        .padding(.horizontal, 8)
        // A wallpaper behind it, as on the phone: black on the list's black
        // would not show.
        .background(
            LinearGradient(colors: [Color(red: 0.2, green: 0.3, blue: 0.45), Color(red: 0.1, green: 0.13, blue: 0.2)], startPoint: .top, endPoint: .bottom),
            in: RoundedRectangle(cornerRadius: 28, style: .continuous)
        )
        .foregroundStyle(.white)
        .environment(\.colorScheme, .dark)
        .allowsHitTesting(false)
    }
}
