import SwiftUI

// New project on one computer, from the ⋯ menu on its header in
// Workspaces. Menu: where the item sits. A: one form, empty or cloned.
// B: two doors first, then the clone list. C: describe it, the agent
// builds it. D: a name and nothing else.

/// Today's Workspaces tab, with studio's header first.
private struct WorkspacesBehind<Overlay: View>: View {
    @ViewBuilder var overlay: Overlay

    var body: some View {
        TabView(selection: .constant(1)) {
            Tab("Home", systemImage: "house", value: 0) { Text("Home") }
                .badge(3)
            Tab("Workspaces", systemImage: "square.stack.3d.up", value: 1) {
                NavigationStack {
                    List {
                        Section {} header: { header("studio", cpu: 71, mem: 58) }
                        Section {
                            row("main", detail: nil)
                            row("feat/share-extension", detail: "1 agent")
                        } header: { repo("sheprd", tint: .indigo) }
                        Section {
                            row("main", detail: "1 agent · 1 terminal")
                        } header: { repo("moltis", tint: .green) }
                        Section {} header: { header("build-01", cpu: 12, mem: 22) }
                    }
                    .listSectionSpacing(14)
                    .navigationTitle("Workspaces")
                    .navigationBarTitleDisplayMode(.inline)
                    .searchable(text: .constant(""), prompt: "Workspaces, chats, terminals")
                }
                .overlay(alignment: .topTrailing) { overlay }
            }
            Tab("PRs", systemImage: "arrow.triangle.pull", value: 2) { Text("PRs") }
            Tab("Settings", systemImage: "gearshape", value: 3) { Text("Settings") }
        }
    }

    private func header(_ name: String, cpu: Int, mem: Int) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack(spacing: 8) {
                Image(systemName: "chevron.right").font(.caption.weight(.bold)).foregroundStyle(.secondary)
                    .rotationEffect(.degrees(90))
                Image(systemName: "laptopcomputer")
                Text(name).font(.headline)
                Spacer()
                Circle().fill(.green).frame(width: 8, height: 8)
                Image(systemName: "ellipsis.circle").font(.title3)
            }
            .foregroundStyle(.primary)
            Text("CPU \(cpu)%   MEM \(mem)%").font(.caption2.monospaced()).foregroundStyle(.secondary)
        }
        .textCase(nil)
    }

    private func repo(_ name: String, tint: Color) -> some View {
        HStack(spacing: 8) {
            Text(String(name.prefix(1)).uppercased()).font(.caption.bold()).foregroundStyle(.white)
                .frame(width: 22, height: 22).background(tint.gradient, in: .rect(cornerRadius: 6))
            Text(name).font(.subheadline.weight(.semibold))
            Spacer()
            Image(systemName: "plus").foregroundStyle(.tint)
        }
        .textCase(nil)
    }

    private func row(_ name: String, detail: String?) -> some View {
        HStack {
            Circle().strokeBorder(detail == nil ? .gray : .green, lineWidth: 1.5).frame(width: 11, height: 11)
            VStack(alignment: .leading, spacing: 2) {
                Text(name).font(.body.monospaced()).lineLimit(1)
                if let detail { Text(detail).font(.caption).foregroundStyle(.secondary) }
            }
        }
    }
}

private struct OverWorkspaces<Sheet: View>: View {
    var detents: Set<PresentationDetent> = [.large]
    @ViewBuilder var sheet: Sheet

    var body: some View {
        WorkspacesBehind {}
            .sheet(isPresented: .constant(true)) { sheet.presentationDetents(detents) }
    }
}

// MARK: Menu, where it sits

struct NewProjectMenu: View {
    var body: some View {
        WorkspacesBehind {
            // Drawn open: a real Menu cannot be opened from a launch argument.
            VStack(alignment: .leading, spacing: 0) {
                item("New Project…", "folder.badge.plus", bold: true)
                Divider()
                item("Hold Alerts at the Desk", "bell.badge")
                item("Computer Details", "info.circle")
                Divider().padding(.vertical, 4)
                item("Forget studio…", "trash", destructive: true)
            }
            .padding(.vertical, 6)
            .frame(width: 270)
            .background(.regularMaterial, in: .rect(cornerRadius: 22))
            .shadow(color: .black.opacity(0.25), radius: 24)
            .padding(.top, 150).padding(.trailing, 18)
        }
    }

    private func item(_ title: String, _ symbol: String, bold: Bool = false, destructive: Bool = false) -> some View {
        HStack {
            Text(title).fontWeight(bold ? .semibold : .regular)
            Spacer()
            Image(systemName: symbol)
        }
        .foregroundStyle(destructive ? .red : .primary)
        .padding(.horizontal, 16).padding(.vertical, 11)
    }
}

// MARK: A, one form

struct NewProjectForm: View {
    var state: Int
    private var cloning: Bool { state == 1 }

    var body: some View {
        OverWorkspaces {
            NavigationStack {
                Form {
                    Section {
                        Picker("Start from", selection: .constant(cloning)) {
                            Text("Empty").tag(false)
                            Text("Clone").tag(true)
                        }
                        .pickerStyle(.segmented)
                        .listRowBackground(Color.clear)
                        .listRowInsets(EdgeInsets())
                    }
                    if cloning {
                        Section {
                            TextField("Repository", text: .constant("github.com/sheprd-labs/weather"))
                                .font(.body.monospaced())
                        } header: { Text("Repository") } footer: {
                            Text("Cloned on studio with its own Git credentials. owner/name or any URL.")
                        }
                    }
                    Section {
                        TextField("Name", text: .constant("weather")).font(.body.monospaced())
                        LabeledContent("In") {
                            HStack(spacing: 4) { Text("~/code"); Image(systemName: "chevron.up.chevron.down").font(.caption) }
                        }
                        if !cloning {
                            Toggle("Initialize Git", isOn: .constant(true))
                        }
                    } header: { Text("Folder on studio") } footer: {
                        Text(cloning ? "Clones into ~/code/weather." : "Creates ~/code/weather with a main branch.")
                    }
                    Section("Then open") {
                        Picker("Agent", selection: .constant("Claude")) {
                            Text("Claude").tag("Claude"); Text("Codex").tag("Codex"); Text("Terminal").tag("Terminal")
                        }
                        Text("A small CLI that prints tomorrow's weather.")
                            .foregroundStyle(cloning ? .tertiary : .primary)
                            .frame(minHeight: 56, alignment: .topLeading)
                    }
                }
                .navigationTitle("New Project")
                .navigationBarTitleDisplayMode(.inline)
                .toolbar {
                    ToolbarItem(placement: .cancellationAction) { Button(role: .cancel) {} }
                    ToolbarItem(placement: .confirmationAction) {
                        Button(cloning ? "Clone" : "Create") {}.buttonStyle(.glassProminent)
                    }
                }
            }
        }
    }
}

// MARK: B, two doors

struct NewProjectDoors: View {
    var state: Int

    var body: some View {
        OverWorkspaces(detents: state == 0 ? [.medium] : [.large]) {
            NavigationStack {
                if state == 0 { doors } else { cloneList }
            }
        }
    }

    private var doors: some View {
        VStack(spacing: 12) {
            door("Empty Project", "A new folder with Git, then an agent in it.", "folder.badge.plus", .blue)
            door("Clone a Repository", "From GitHub, or any Git URL.", "arrow.down.doc", .purple)
            door("From the Clipboard", "github.com/sheprd-labs/weather", "doc.on.clipboard", .orange)
            Spacer()
        }
        .padding(.horizontal)
        .padding(.top, 8)
        .navigationTitle("New Project on studio")
        .navigationBarTitleDisplayMode(.inline)
        .toolbar { ToolbarItem(placement: .cancellationAction) { Button(role: .cancel) {} } }
    }

    private func door(_ title: String, _ detail: String, _ symbol: String, _ tint: Color) -> some View {
        HStack(spacing: 14) {
            Image(systemName: symbol).font(.title2).foregroundStyle(.white)
                .frame(width: 48, height: 48).background(tint.gradient, in: .rect(cornerRadius: 13))
            VStack(alignment: .leading, spacing: 2) {
                Text(title).font(.headline)
                Text(detail).font(.subheadline).foregroundStyle(.secondary).lineLimit(1)
            }
            Spacer()
            Image(systemName: "chevron.right").foregroundStyle(.tertiary)
        }
        .padding(14)
        .background(.fill.quinary, in: .rect(cornerRadius: 18))
    }

    private var cloneList: some View {
        List {
            Section("Your repositories") {
                repo("sheprd-labs/weather", "Swift · updated today", cloned: false)
                repo("sheprd-labs/site", "TypeScript · 2 days ago", cloned: false)
                repo("penso/dotfiles", "Shell · last week", cloned: false)
            }
            Section("Already on studio") {
                repo("sheprd-labs/sheprd", "~/code/sheprd", cloned: true)
                repo("moltis-org/moltis", "~/code/moltis", cloned: true)
            }
        }
        .searchable(text: .constant(""), placement: .navigationBarDrawer(displayMode: .always), prompt: "Search or paste a URL")
        .navigationTitle("Clone on studio")
        .navigationBarTitleDisplayMode(.inline)
        .toolbar { ToolbarItem(placement: .cancellationAction) { Button(role: .cancel) {} } }
    }

    private func repo(_ name: String, _ detail: String, cloned: Bool) -> some View {
        HStack {
            VStack(alignment: .leading, spacing: 2) {
                Text(name).font(.body.monospaced()).lineLimit(1)
                Text(detail).font(.caption).foregroundStyle(.secondary)
            }
            Spacer()
            if cloned {
                Text("Open").font(.subheadline).foregroundStyle(.secondary)
            } else {
                Button("Clone") {}.buttonStyle(.bordered).controlSize(.small)
            }
        }
    }
}

// MARK: C, describe it

struct NewProjectPrompt: View {
    var body: some View {
        OverWorkspaces {
            NavigationStack {
                VStack(alignment: .leading, spacing: 14) {
                    Text("What are you starting?").font(.title2.bold())
                    Text("A small CLI that prints tomorrow's weather for a city, with a --json flag.")
                        .frame(maxWidth: .infinity, minHeight: 140, alignment: .topLeading)
                        .padding(14)
                        .background(.fill.quinary, in: .rect(cornerRadius: 18))
                    ScrollView(.horizontal, showsIndicators: false) {
                        HStack(spacing: 8) {
                            chip("laptopcomputer", "studio")
                            chip("folder", "~/code/weather-cli")
                            chip("sparkle", "Claude")
                            chip("arrow.triangle.branch", "Git")
                        }
                    }
                    Text("The name comes from your words; tap a chip to change it. The agent starts in the new folder with this as its first prompt.")
                        .font(.footnote).foregroundStyle(.secondary)
                    Spacer()
                }
                .padding(.horizontal)
                .navigationTitle("New Project")
                .navigationBarTitleDisplayMode(.inline)
                .toolbar {
                    ToolbarItem(placement: .cancellationAction) { Button(role: .cancel) {} }
                    ToolbarItem(placement: .confirmationAction) {
                        Button("Start") {}.buttonStyle(.glassProminent)
                    }
                }
            }
        }
    }

    private func chip(_ symbol: String, _ title: String) -> some View {
        HStack(spacing: 5) {
            Image(systemName: symbol).font(.caption)
            Text(title).font(.subheadline.monospaced())
            Image(systemName: "chevron.down").font(.caption2).foregroundStyle(.secondary)
        }
        .padding(.horizontal, 12).padding(.vertical, 8)
        .background(.fill.tertiary, in: .capsule)
    }
}

// MARK: D, a name and nothing else

struct NewProjectQuick: View {
    var body: some View {
        OverWorkspaces(detents: [.height(300)]) {
            NavigationStack {
                VStack(alignment: .leading, spacing: 12) {
                    HStack {
                        Image(systemName: "folder").foregroundStyle(.secondary)
                        Text("weather").font(.title3.monospaced())
                        Spacer()
                    }
                    .padding(14)
                    .background(.fill.quinary, in: .rect(cornerRadius: 16))
                    HStack(spacing: 4) {
                        Text("studio:").foregroundStyle(.secondary)
                        Text("~/code/").foregroundStyle(.tint)
                        Text("weather")
                        Image(systemName: "chevron.up.chevron.down").font(.caption2).foregroundStyle(.tint)
                    }
                    .font(.footnote.monospaced())
                    Text("A Git repository on main, opened in a new workspace. Start an agent from there.")
                        .font(.footnote).foregroundStyle(.secondary)
                    Button("Clone a repository instead") {}.font(.footnote)
                }
                .padding(.horizontal)
                .navigationTitle("New Project")
                .navigationBarTitleDisplayMode(.inline)
                .toolbar {
                    ToolbarItem(placement: .cancellationAction) { Button(role: .cancel) {} }
                    ToolbarItem(placement: .confirmationAction) {
                        Button("Create") {}.buttonStyle(.glassProminent)
                    }
                }
            }
        }
    }
}
