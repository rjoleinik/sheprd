import ActivityKit
import SheprdRemoteCore
import SwiftUI
import WidgetKit

/// One running chat on the lock screen and in the Dynamic Island, in the
/// style chosen in Settings (`LiveActivityStyleSetting`). Tapping anywhere
/// opens `herdr-remote://chat?...`, which the app turns into the chat's
/// page. Content a computer sealed for the push relay is opened here with
/// this phone's notify key; when it does not open, the activity is the
/// fixed `SealedCover`, with nothing from the push.
struct ChatLiveActivity: Widget {
    var body: some WidgetConfiguration {
        ActivityConfiguration(for: ChatActivityAttributes.self) { context in
            if let opened = context.attributes.opened(context.state) {
                LockScreenView(facts: opened.facts, state: opened.state)
                    .activityBackgroundTint(Color.black.opacity(0.55))
                    .activitySystemActionForegroundColor(.white)
                    .widgetURL(opened.facts.link.url)
            } else {
                SealedCover()
                    .activityBackgroundTint(Color.black.opacity(0.55))
                    .activitySystemActionForegroundColor(.white)
            }
        } dynamicIsland: { context in
            guard let opened = context.attributes.opened(context.state) else {
                return SealedCover.island
            }
            let state = opened.state
            let facts = opened.facts
            let style = LiveActivityStyleSetting.current
            return DynamicIsland {
                DynamicIslandExpandedRegion(.leading) {
                    AgentBadge(agent: facts.agent, size: 28)
                        .padding(.leading, 4)
                }
                DynamicIslandExpandedRegion(.trailing) {
                    if state.phase == .working {
                        Elapsed(since: state.startedAt)
                            .font(.caption.weight(.semibold))
                            .frame(maxWidth: 56, alignment: .trailing)
                    } else {
                        PhaseMark(phase: state.phase)
                    }
                }
                DynamicIslandExpandedRegion(.center) {
                    VStack(alignment: .leading, spacing: 2) {
                        Text(IslandParts.title(state, facts: facts, style: style))
                            .font(.subheadline.weight(.semibold))
                            .lineLimit(1)
                        Text(IslandParts.subtitle(facts: facts, style: style))
                            .font(.caption)
                            .foregroundStyle(.secondary)
                            .lineLimit(1)
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                }
                DynamicIslandExpandedRegion(.bottom) {
                    IslandParts.bottom(state, facts: facts, style: style)
                }
            } compactLeading: {
                IslandParts.compactLeading(state, facts: facts, style: style)
            } compactTrailing: {
                IslandParts.compactTrailing(state)
            } minimal: {
                PhaseMark(phase: state.phase, size: 9)
            }
            .widgetURL(facts.link.url)
            .keylineTint(Palette.phase(state.phase))
        }
    }
}
