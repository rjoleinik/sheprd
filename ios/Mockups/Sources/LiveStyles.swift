import SwiftUI

// Live Activity styles, picked in Settings > Notifications. Each variant
// draws the same three chats on a stand-in lock screen (state 0) and in the
// Dynamic Island (state 1): one working, one waiting for an answer, one
// finished. Now: what ships, raw tool titles. A Minimal: status only.
// B Standard: the daemon's state in plain words. C Complete: B with the
// steps and counts. D Geek: the commands, cleaned, with their results.

private enum LivePhase { case working, question, finished }

private struct LiveSample {
    var agent: Agent
    var title: String
    var workspace: String
    var computer: String
    var phase: LivePhase
    var elapsed: String
    var minutes: String
    /// The tool call's title as the agent sends it.
    var raw: String
    /// The command alone, with the shell wrapper and quotes taken off.
    var command: String
    /// What the tool call does, in words, from its kind and arguments.
    var plain: String
    var symbol: String
    var previousRaw: [String]
    var previousCommands: [String]
    var previousPlain: [(String, String)]
    var plan: (Int, Int)?
    var question: String?
    var options: [String]
    var summary: String?
    var stats: String
    var tokens: String
}

private let samples: [LiveSample] = [
    LiveSample(
        agent: .codex, title: "Fix CI issues", workspace: "performance", computer: "m4max",
        phase: .working, elapsed: "17:06", minutes: "17 min",
        raw: "`/bin/zsh -lc \"gh pr checks 1548 --repo sheprd-labs/sheprd\"`",
        command: "gh pr checks 1548 --repo sheprd-labs/sheprd",
        plain: "Checking pull request #1548", symbol: "checkmark.seal",
        previousRaw: ["`/bin/zsh -lc 'git status --short; gh pr checks 1548 --repo…", "`/bin/zsh -lc \"gh api repos/sheprd-labs/sheprd/issues/com…"],
        previousCommands: ["git status --short", "gh api repos/sheprd-labs/sheprd/issues/1548/comments"],
        previousPlain: [("Looked at the changes", "doc.text.magnifyingglass"), ("Read the review comments", "text.bubble")],
        plan: (3, 5), question: nil, options: [], summary: nil,
        stats: "14 steps · 3 files", tokens: "41k in · 6k out"
    ),
    LiveSample(
        agent: .claude, title: "Video attachments", workspace: "sheprd", computer: "studio",
        phase: .question, elapsed: "2:09", minutes: "2 min",
        raw: "`/bin/zsh -lc 'git push origin feat/video-attachments'`",
        command: "git push origin feat/video-attachments",
        plain: "Wants to push feat/video-attachments", symbol: "arrow.up.circle",
        previousRaw: ["`/bin/zsh -lc 'git add crates/sheprd-acp/prompts/desktop.…", "`/bin/zsh -lc 'git diff --cached --check'`"],
        previousCommands: ["git add crates/sheprd-acp/prompts/desktop.md", "git diff --cached --check"],
        previousPlain: [("Staged 2 files", "plus.square.on.square"), ("Checked the diff", "checkmark.circle")],
        plan: (4, 4), question: "Allow git push origin feat/video-attachments?", options: ["Allow once", "Always", "Deny"],
        summary: nil, stats: "9 steps · 2 files", tokens: "18k in · 3k out"
    ),
    LiveSample(
        agent: .codex, title: "Larry feedback", workspace: "herdr", computer: "m4max",
        phase: .finished, elapsed: "22:53", minutes: "23 min",
        raw: "`/bin/zsh -lc 'herdr workspace create larry-feedback'`",
        command: "herdr workspace create larry-feedback",
        plain: "Opened a workspace for the video", symbol: "rectangle.stack.badge.plus",
        previousRaw: ["`/bin/zsh -lc 'ls -t ~/Downloads | head -1'`", "`/bin/zsh -lc 'herdr workspace create larry-feedback'`"],
        previousCommands: ["ls -t ~/Downloads | head -1", "cp ~/Downloads/feedback.mov .sheprd/attachments/"],
        previousPlain: [("Found the last video", "film"), ("Opened a workspace", "rectangle.stack.badge.plus")],
        plan: nil, question: nil, options: [],
        summary: "Attached the video and opened a herdr space for it.",
        stats: "21 steps · 4 files · +120 −8", tokens: "676k in · 346k out"
    ),
]

private let fleet = "m4max: 2 working · studio: 1 waiting"

// MARK: - Shared pieces

private struct LiveWallpaper<Content: View>: View {
    @ViewBuilder var content: Content

    var body: some View {
        ZStack(alignment: .top) {
            LinearGradient(colors: [Color(red: 0.42, green: 0.25, blue: 0.16), Color(red: 0.62, green: 0.38, blue: 0.24), Color(red: 0.2, green: 0.16, blue: 0.24)],
                           startPoint: .top, endPoint: .bottom)
                .ignoresSafeArea()
            VStack(spacing: 10) {
                Text("Sunday, November 22").font(.title3.weight(.semibold)).padding(.top, 54)
                Text("9:41").font(.system(size: 84, weight: .semibold, design: .rounded))
                    .padding(.bottom, 8)
                content.padding(.horizontal, 10)
                Spacer(minLength: 0)
            }
            .foregroundStyle(.white)
        }
        .environment(\.colorScheme, .dark)
    }
}

private struct Card<Content: View>: View {
    var tint: Color = .clear
    @ViewBuilder var content: Content

    var body: some View {
        content
            .padding(14)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(tint.opacity(0.16))
            .background(.black.opacity(0.62), in: .rect(cornerRadius: 24))
    }
}

private func phaseColor(_ phase: LivePhase) -> Color {
    switch phase {
    case .working: .green
    case .question: .orange
    case .finished: .green
    }
}

private struct PhaseDot: View {
    var phase: LivePhase
    var size: CGFloat = 9

    var body: some View {
        switch phase {
        case .finished:
            Image(systemName: "checkmark.circle.fill").font(.system(size: size * 1.8)).foregroundStyle(.green)
        default:
            Circle().fill(phaseColor(phase)).frame(width: size, height: size)
        }
    }
}

private struct Pill: View {
    var text: String
    var color: Color
    var filled = false

    var body: some View {
        Text(text)
            .font(.subheadline.weight(.semibold))
            .padding(.horizontal, 12).padding(.vertical, 5)
            .background(filled ? color : .white.opacity(0.16), in: Capsule())
            .foregroundStyle(filled ? .black : .white)
    }
}

private struct Segments: View {
    var done: Int
    var total: Int
    var color: Color = .green

    var body: some View {
        HStack(spacing: 3) {
            ForEach(0..<total, id: \.self) { index in
                Capsule().fill(index < done ? color : .white.opacity(0.2)).frame(height: 4)
            }
        }
    }
}

private struct AnswerRow: View {
    var options: [String]

    var body: some View {
        HStack(spacing: 6) {
            ForEach(Array(options.enumerated()), id: \.offset) { index, option in
                Text(option).font(.footnote.weight(.semibold)).lineLimit(1)
                    .frame(maxWidth: .infinity).padding(.vertical, 7)
                    .background(index == 0 ? Color.orange : .white.opacity(0.16), in: Capsule())
                    .foregroundStyle(index == 0 ? .black : .white)
            }
        }
    }
}

// MARK: - Now: what ships

private struct NowCard: View {
    var sample: LiveSample

    var body: some View {
        Card {
            VStack(alignment: .leading, spacing: 8) {
                HStack(spacing: 8) {
                    AgentIcon(agent: sample.agent, size: 22)
                    Text("\(sample.workspace) · \(sample.title)").font(.subheadline.weight(.semibold)).lineLimit(1)
                    Spacer(minLength: 4)
                    Text(sample.agent.name).font(.caption).foregroundStyle(.secondary)
                }
                ForEach(sample.previousRaw, id: \.self) { step in
                    HStack(spacing: 6) {
                        Image(systemName: "checkmark").font(.caption2).foregroundStyle(.white.opacity(0.35))
                        Text(step).font(.caption).foregroundStyle(.white.opacity(0.35)).lineLimit(1)
                    }
                }
                HStack(spacing: 10) {
                    PhaseDot(phase: sample.phase)
                    VStack(alignment: .leading, spacing: 1) {
                        Text(sample.phase == .working ? sample.raw : sample.phase == .question ? "Needs an answer" : "Finished")
                            .font(.headline).lineLimit(1)
                        Text(sample.phase == .working ? sample.elapsed : sample.question ?? sample.summary ?? "")
                            .font(.caption).foregroundStyle(.secondary).lineLimit(1)
                    }
                }
                if sample.phase != .question { Segments(done: sample.phase == .finished ? 1 : 0, total: 1) }
            }
        }
    }
}

// MARK: - A Minimal: status only

private struct MinimalCard: View {
    var sample: LiveSample

    var body: some View {
        Card(tint: sample.phase == .question ? .orange : .clear) {
            HStack(spacing: 10) {
                AgentIcon(agent: sample.agent, size: 30)
                VStack(alignment: .leading, spacing: 1) {
                    Text(sample.title).font(.headline).lineLimit(1)
                    Text("\(sample.computer) · \(sample.workspace)").font(.caption).foregroundStyle(.secondary)
                }
                Spacer(minLength: 6)
                switch sample.phase {
                case .working:
                    HStack(spacing: 6) {
                        PhaseDot(phase: .working, size: 7)
                        Text(sample.elapsed).font(.subheadline.monospacedDigit().weight(.semibold))
                    }
                case .question: Pill(text: "Needs you", color: .orange, filled: true)
                case .finished: Pill(text: "Done", color: .green)
                }
            }
        }
    }
}

// MARK: - B Standard: the daemon's state in words

private func headline(_ sample: LiveSample) -> String {
    switch sample.phase {
    case .working: "Working for \(sample.minutes)"
    case .question: "Waiting for you"
    case .finished: "Finished in \(sample.minutes)"
    }
}

private struct StandardCard: View {
    var sample: LiveSample
    var complete = false

    var body: some View {
        Card(tint: sample.phase == .question ? .orange : .clear) {
            VStack(alignment: .leading, spacing: 9) {
                HStack(spacing: 8) {
                    AgentIcon(agent: sample.agent, size: 24)
                    Text(sample.title).font(.subheadline.weight(.semibold)).lineLimit(1)
                    Spacer(minLength: 4)
                    Text(sample.computer).font(.caption).foregroundStyle(.secondary)
                }
                if complete {
                    ForEach(Array(sample.previousPlain.enumerated()), id: \.offset) { _, step in
                        Label(step.0, systemImage: step.1)
                            .font(.caption).foregroundStyle(.white.opacity(0.45)).labelStyle(.titleAndIcon)
                    }
                }
                HStack(alignment: .center, spacing: 10) {
                    PhaseDot(phase: sample.phase)
                    VStack(alignment: .leading, spacing: 2) {
                        Text(headline(sample)).font(.headline)
                        Group {
                            switch sample.phase {
                            case .working: Label(sample.plain, systemImage: sample.symbol)
                            case .question: Text(sample.question ?? "")
                            case .finished: Text(sample.summary ?? "")
                            }
                        }
                        .font(.subheadline).foregroundStyle(.white.opacity(0.8)).lineLimit(2)
                    }
                }
                if sample.phase == .question {
                    AnswerRow(options: sample.options)
                } else if let plan = sample.plan, sample.phase == .working {
                    Segments(done: plan.0 - 1, total: plan.1)
                    HStack {
                        Text("Step \(plan.0) of \(plan.1)")
                        Spacer()
                        if complete { Text(sample.stats) }
                    }
                    .font(.caption).foregroundStyle(.secondary)
                } else if complete, sample.phase == .finished {
                    Text(sample.stats).font(.caption).foregroundStyle(.secondary)
                }
                if complete, sample.phase == .working {
                    Text(fleet).font(.caption2).foregroundStyle(.white.opacity(0.5))
                }
            }
        }
    }
}

// MARK: - D Geek: the commands, cleaned

private struct GeekCard: View {
    var sample: LiveSample

    var body: some View {
        Card(tint: sample.phase == .question ? .orange : .clear) {
            VStack(alignment: .leading, spacing: 6) {
                HStack(spacing: 8) {
                    AgentIcon(agent: sample.agent, size: 20)
                    Text("\(sample.computer):\(sample.workspace)").font(.system(.caption, design: .monospaced).weight(.semibold))
                    Text(sample.title).font(.caption).foregroundStyle(.secondary).lineLimit(1)
                    Spacer(minLength: 4)
                    Text(sample.elapsed).font(.system(.caption, design: .monospaced))
                }
                VStack(alignment: .leading, spacing: 3) {
                    ForEach(sample.previousCommands, id: \.self) { command in
                        HStack(spacing: 6) {
                            Text("✓").foregroundStyle(.green)
                            Text(command).foregroundStyle(.white.opacity(0.45)).lineLimit(1)
                        }
                    }
                    HStack(spacing: 6) {
                        Text(sample.phase == .question ? "?" : sample.phase == .finished ? "✓" : "$")
                            .foregroundStyle(phaseColor(sample.phase))
                        Text(sample.command).lineLimit(1)
                        if sample.phase == .working { Text("▍").foregroundStyle(.green) }
                    }
                }
                .font(.system(.footnote, design: .monospaced))
                .padding(8)
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(.black.opacity(0.55), in: .rect(cornerRadius: 10))
                if sample.phase == .question {
                    AnswerRow(options: sample.options)
                }
                HStack {
                    Text(sample.tokens)
                    Spacer()
                    Text(sample.plan.map { "plan \($0.0)/\($0.1)" } ?? sample.stats)
                }
                .font(.system(.caption2, design: .monospaced)).foregroundStyle(.secondary)
            }
        }
    }
}

// MARK: - The Dynamic Island

private struct Island<Expanded: View>: View {
    var leading: AnyView
    var trailing: AnyView
    @ViewBuilder var expanded: Expanded

    var body: some View {
        VStack(spacing: 22) {
            HStack {
                leading
                Spacer()
                trailing
            }
            .padding(.horizontal, 14)
            .frame(width: 250, height: 37)
            .background(.black, in: Capsule())
            .padding(.top, 11)
            expanded
                .padding(18)
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(.black, in: .rect(cornerRadius: 44))
                .padding(.horizontal, 8)
        }
    }
}

private struct IslandScreen: View {
    var variant: String

    var body: some View {
        ZStack(alignment: .top) {
            LinearGradient(colors: [Color(red: 0.2, green: 0.3, blue: 0.45), Color(red: 0.08, green: 0.1, blue: 0.16)],
                           startPoint: .top, endPoint: .bottom)
                .ignoresSafeArea()
            content
                .foregroundStyle(.white)
                .ignoresSafeArea(edges: .top)
        }
        .statusBarHidden()
        .environment(\.colorScheme, .dark)
    }

    private var working: LiveSample { samples[0] }
    private var asking: LiveSample { samples[1] }

    @ViewBuilder
    private var content: some View {
        switch variant {
        case "Now":
            Island(
                leading: AnyView(HStack(spacing: 4) { PhaseDot(phase: .working, size: 8); Text("2").font(.caption.bold()) }),
                trailing: AnyView(HStack(spacing: 4) { PhaseDot(phase: .question, size: 8); Text("1").font(.caption.bold()) })
            ) {
                VStack(alignment: .leading, spacing: 8) {
                    HStack { AgentIcon(agent: .codex, size: 28); Text("Codex is working").font(.subheadline.weight(.semibold)); Spacer(); Text("17:06").font(.caption) }
                    Text(working.raw).font(.footnote).lineLimit(2).padding(8)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .background(.white.opacity(0.08), in: .rect(cornerRadius: 10))
                }
            }
        case "A":
            Island(
                leading: AnyView(AgentIcon(agent: .codex, size: 22)),
                trailing: AnyView(Text("17:06").font(.caption.monospacedDigit().bold()).foregroundStyle(.green))
            ) {
                HStack(spacing: 10) {
                    AgentIcon(agent: .codex, size: 34)
                    VStack(alignment: .leading) {
                        Text(working.title).font(.headline)
                        Text("Working · \(working.computer)").font(.caption).foregroundStyle(.secondary)
                    }
                    Spacer()
                    Pill(text: "Open", color: .white)
                }
            }
        case "B", "C":
            Island(
                leading: AnyView(HStack(spacing: 5) { Image(systemName: working.symbol).foregroundStyle(.green); Text("2").font(.caption.bold()) }),
                trailing: AnyView(HStack(spacing: 4) { Circle().fill(.orange).frame(width: 8, height: 8); Text("1 waiting").font(.caption2.bold()).foregroundStyle(.orange) })
            ) {
                VStack(alignment: .leading, spacing: 9) {
                    HStack {
                        AgentIcon(agent: .codex, size: 28)
                        VStack(alignment: .leading, spacing: 0) {
                            Text(headline(working)).font(.subheadline.weight(.semibold))
                            Text("\(working.title) · \(working.computer)").font(.caption).foregroundStyle(.secondary)
                        }
                        Spacer()
                    }
                    Label(working.plain, systemImage: working.symbol).font(.subheadline)
                    if variant == "C", let plan = working.plan {
                        Segments(done: plan.0 - 1, total: plan.1)
                        HStack { Text("Step \(plan.0) of \(plan.1)"); Spacer(); Text(working.stats) }
                            .font(.caption).foregroundStyle(.secondary)
                    }
                    HStack(spacing: 6) {
                        Circle().fill(.orange).frame(width: 8, height: 8)
                        Text("Claude on studio is waiting for you").font(.caption).foregroundStyle(.orange)
                    }
                }
            }
        default:
            Island(
                leading: AnyView(Text("$").font(.system(.caption, design: .monospaced).bold()).foregroundStyle(.green)),
                trailing: AnyView(Text("17:06").font(.system(.caption, design: .monospaced)))
            ) {
                VStack(alignment: .leading, spacing: 6) {
                    Text("m4max:performance · codex").font(.system(.caption, design: .monospaced)).foregroundStyle(.secondary)
                    Text("$ \(working.command)").font(.system(.footnote, design: .monospaced)).lineLimit(2)
                    HStack { Text(working.tokens); Spacer(); Text("plan 3/5") }
                        .font(.system(.caption2, design: .monospaced)).foregroundStyle(.secondary)
                }
            }
        }
    }
}

// MARK: - The screens

struct LiveStyle: View {
    var variant: String
    var state: Int

    var body: some View {
        if state == 1 {
            IslandScreen(variant: variant)
        } else {
            LiveWallpaper {
                VStack(spacing: 8) {
                    ForEach(Array(samples.enumerated()), id: \.offset) { _, sample in
                        card(sample)
                    }
                }
            }
        }
    }

    @ViewBuilder
    private func card(_ sample: LiveSample) -> some View {
        switch variant {
        case "A": MinimalCard(sample: sample)
        case "B": StandardCard(sample: sample)
        case "C": StandardCard(sample: sample, complete: true)
        case "D": GeekCard(sample: sample)
        default: NowCard(sample: sample)
        }
    }
}

/// Settings > Notifications > Live Activities with the style picker, which
/// takes over Keep Generic: Private is today's generic.
struct LiveStylePicker: View {
    private let styles: [(String, String)] = [
        ("Private", "The agent and whether it works or waits. No chat names, steps, or questions."),
        ("Minimal", "The chat, its computer, and working, waiting, or done."),
        ("Standard", "What the agent is doing, in words, and questions you can answer from the Lock Screen."),
        ("Complete", "Standard with the last steps, plan progress, and counts across your computers."),
        ("Geek", "The commands themselves, with tokens and plan steps."),
    ]

    var body: some View {
        NavigationStack {
            List {
                Section {
                    ForEach(Array(styles.enumerated()), id: \.offset) { index, style in
                        HStack(alignment: .top, spacing: 12) {
                            VStack(alignment: .leading, spacing: 3) {
                                Text(style.0)
                                Text(style.1).font(.footnote).foregroundStyle(.secondary)
                            }
                            Spacer()
                            if index == 2 { Image(systemName: "checkmark").foregroundStyle(.tint).fontWeight(.semibold) }
                        }
                    }
                } header: {
                    Text("Live Activity Style")
                } footer: {
                    Text("Every computer sends the same activity; this phone chooses how much of it to show. Private also tells each computer to leave the chat's name and steps out.")
                }
                Section {
                    StandardCard(sample: samples[0])
                        .listRowInsets(EdgeInsets())
                        .listRowBackground(Color(red: 0.42, green: 0.25, blue: 0.16))
                        .environment(\.colorScheme, .dark)
                        .foregroundStyle(.white)
                } header: {
                    Text("Preview")
                }
                Section {
                    Label("Start Test Activity", systemImage: "platter.filled.top.iphone").foregroundStyle(.tint)
                }
            }
            .navigationTitle("Live Activities")
            .navigationBarTitleDisplayMode(.inline)
        }
    }
}
