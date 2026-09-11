import AppKit
import SwiftUI
import ReplayProtocol

@MainActor
final class ReplayModel: ObservableObject {
    @Published var root: URL
    @Published var replay: ReplayEnvelope?
    @Published var selection = 0
    @Published var busy = false
    @Published var error: String?
    @Published var page = "Session"
    @Published var loadedAt: Date?

    init() {
        let args = CommandLine.arguments
        let position = args.firstIndex(of: "--project")
        let path = position.flatMap { $0 + 1 < args.count ? args[$0 + 1] : nil }
        root = URL(fileURLWithPath: path ?? FileManager.default.currentDirectoryPath).standardizedFileURL
    }

    var executable: URL { root.appendingPathComponent("target/debug/fishing-core-cli") }
    var controllerBuilt: Bool { FileManager.default.isExecutableFile(atPath: executable.path) }
    var frame: ReplayFrame? {
        guard let frames = replay?.frames, frames.indices.contains(selection) else { return nil }
        return frames[selection]
    }

    func chooseProject() {
        let panel = NSOpenPanel()
        panel.canChooseDirectories = true
        panel.canChooseFiles = false
        panel.message = "Choose the Arcane Fishing Bot project folder."
        guard panel.runModal() == .OK, let url = panel.url else { return }
        root = url
        replay = nil
        error = nil
    }

    func chooseScenario() {
        let panel = NSOpenPanel()
        panel.allowedContentTypes = [.json]
        panel.directoryURL = root.appendingPathComponent("tests/replays")
        panel.message = "Choose a scenario for the Rust controller to replay. No input will be sent."
        guard panel.runModal() == .OK, let url = panel.url else { return }
        load(url)
    }

    func loadSample(_ name: String = "normal-cycle") {
        load(root.appendingPathComponent("tests/replays/\(name).json"))
    }

    func load(_ scenario: URL) {
        guard !busy else { return }
        busy = true
        error = nil
        let binary = executable
        Task {
            do {
                let result = try await Task.detached(priority: .userInitiated) {
                    try ReplayRunner.run(executable: binary, scenario: scenario)
                }.value
                replay = result
                selection = max(0, result.frames.count - 1)
                loadedAt = Date()
                page = "Session"
            } catch { self.error = error.localizedDescription }
            busy = false
        }
    }
}

@main
struct ArcaneNativeApp: App {
    @StateObject private var model = ReplayModel()
    var body: some Scene {
        WindowGroup("Arcane · Native comparison") {
            ContentView(model: model)
                .frame(minWidth: 960, minHeight: 660)
                .onAppear {
                    NSApplication.shared.setActivationPolicy(.regular)
                    NSApplication.shared.activate(ignoringOtherApps: false)
                    if model.replay == nil && model.controllerBuilt { model.loadSample() }
                }
        }
        .defaultSize(width: 1140, height: 780)
        .windowStyle(.titleBar)
        .commands {
            CommandGroup(replacing: .newItem) {
                Button("Open replay scenario…") { model.chooseScenario() }
                    .keyboardShortcut("o")
                    .disabled(model.busy)
            }
        }
    }
}

struct ContentView: View {
    @ObservedObject var model: ReplayModel
    var body: some View {
        NavigationSplitView {
            VStack(alignment: .leading, spacing: 22) {
                HStack(spacing: 12) {
                    Image(systemName: "water.waves").font(.title).foregroundStyle(.teal)
                    VStack(alignment: .leading, spacing: 2) {
                        Text("Arcane").font(.title2.bold())
                        Text("Native comparison").font(.caption).foregroundStyle(.secondary)
                    }
                }.padding(.horizontal, 12).padding(.top, 20)
                List(selection: $model.page) {
                    Label("Session", systemImage: "play.rectangle").tag("Session")
                    Label("Readiness", systemImage: "checklist").tag("Readiness")
                    Label("Native services", systemImage: "cpu").tag("Native services")
                }.listStyle(.sidebar)
                VStack(alignment: .leading, spacing: 8) {
                    Label("Recorded observations", systemImage: "film").font(.caption.bold())
                    Text("This comparison runs the shared Rust controller against replay files. Live capture and input are not connected.")
                        .font(.caption).foregroundStyle(.secondary)
                }.padding(14).background(.quaternary.opacity(0.4), in: RoundedRectangle(cornerRadius: 12))
            }.padding(14).navigationSplitViewColumnWidth(240)
        } detail: {
            ScrollView {
                VStack(alignment: .leading, spacing: 22) {
                    if let error = model.error {
                        Label(error, systemImage: "exclamationmark.triangle")
                            .foregroundStyle(.orange).padding(14)
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .background(.orange.opacity(0.08), in: RoundedRectangle(cornerRadius: 12))
                            .textSelection(.enabled)
                    }
                    switch model.page {
                    case "Readiness": readiness
                    case "Native services": services
                    default: session
                    }
                }.padding(28).frame(maxWidth: 1100)
            }.background(Color(nsColor: .windowBackgroundColor))
            .navigationTitle(model.page)
            .toolbar {
                ToolbarItemGroup {
                    Button { model.chooseProject() } label: { Label("Project", systemImage: "folder") }
                        .help(model.root.path)
                    Menu {
                        Button("Normal fishing cycle") { model.loadSample() }
                        Button("Focus loss") { model.loadSample("focus-loss") }
                        Divider()
                        Button("Open scenario…") { model.chooseScenario() }
                    } label: { Label("Load replay", systemImage: "arrow.down.doc") }
                    .disabled(model.busy)
                }
            }
        }
    }

    private var session: some View {
        VStack(alignment: .leading, spacing: 22) {
            HStack {
                VStack(alignment: .leading, spacing: 7) {
                    Text(model.replay?.name ?? "Inspect a fishing cycle").font(.largeTitle.bold())
                    Text("A native view of the same controller used by the desktop app.").foregroundStyle(.secondary)
                }
                Spacer()
                Label("REPLAY", systemImage: "film").font(.caption.weight(.semibold))
                    .padding(.horizontal, 14).padding(.vertical, 10).modifier(GlassBadge())
            }
            if model.busy { ProgressView("Running the Rust replay…").padding() }
            if let frame = model.frame, let replay = model.replay {
                HStack(alignment: .top, spacing: 16) {
                    VStack(alignment: .leading, spacing: 13) {
                        Label("CONTROLLER STATE", systemImage: "circle.inset.filled")
                            .font(.caption.bold()).foregroundStyle(.secondary)
                        Text(frame.phaseTitle).font(.system(size: 32, weight: .semibold, design: .rounded))
                        Text(frame.reason).foregroundStyle(.secondary).fixedSize(horizontal: false, vertical: true)
                        Text(String(format: "At %.3f seconds in the recording", Double(frame.at_ms) / 1000))
                            .font(.caption.monospacedDigit()).foregroundStyle(.secondary)
                    }.frame(maxWidth: .infinity, alignment: .leading).padding(22).card()
                    VStack(spacing: 12) {
                        metric("Confirmed catches", value: frame.fish, symbol: "fish")
                        Divider()
                        metric("Verified feeds", value: frame.feeds, symbol: "leaf")
                    }.padding(22).frame(width: 170).card()
                }
                HStack(alignment: .top, spacing: 18) {
                    VStack(alignment: .leading, spacing: 14) {
                        HStack {
                            Text("Decision timeline").font(.title3.bold())
                            Spacer()
                            Text("\(replay.frames.count) observations").font(.caption).foregroundStyle(.secondary)
                        }
                        ForEach(Array(replay.frames.enumerated()), id: \.offset) { index, event in
                            Button { model.selection = index } label: {
                                HStack(spacing: 12) {
                                    Image(systemName: event.actions.isEmpty ? "circle" : "cursorarrow.click")
                                        .foregroundStyle(index == model.selection ? .teal : .secondary)
                                        .frame(width: 20)
                                    VStack(alignment: .leading, spacing: 4) {
                                        Text(event.phaseTitle).fontWeight(.medium)
                                        Text(event.actions.isEmpty ? "Observe and verify" : event.actions.map(\.label).joined(separator: ", "))
                                            .font(.caption).foregroundStyle(.secondary)
                                    }
                                    Spacer()
                                    Text(String(format: "%.3fs", Double(event.at_ms) / 1000))
                                        .font(.caption.monospacedDigit()).foregroundStyle(.secondary)
                                }.padding(12)
                                    .background(index == model.selection ? Color.teal.opacity(0.10) : Color.clear,
                                                in: RoundedRectangle(cornerRadius: 10))
                                    .contentShape(Rectangle())
                            }.buttonStyle(.plain)
                                .accessibilityLabel("\(event.phaseTitle), at \(event.at_ms) milliseconds")
                                .accessibilityAddTraits(index == model.selection ? .isSelected : [])
                        }
                    }.padding(20).frame(maxWidth: .infinity, alignment: .leading).card()
                    VStack(alignment: .leading, spacing: 16) {
                        Text("Decision evidence").font(.title3.bold())
                        Text(frame.reason).textSelection(.enabled)
                        Divider()
                        Text("Proposed input").font(.headline)
                        if frame.actions.isEmpty {
                            Label("No action", systemImage: "hand.raised").foregroundStyle(.secondary)
                        } else {
                            ForEach(Array(frame.actions.enumerated()), id: \.offset) { _, action in
                                Label(action.label, systemImage: "cursorarrow.click").foregroundStyle(.teal)
                            }
                        }
                        Divider()
                        Text("Inputs are displayed only. The Rust replay does not create an input device or capture the screen.")
                            .font(.caption).foregroundStyle(.secondary)
                        Label("Protocol v\(replay.protocol_version)", systemImage: "checkmark.seal")
                            .font(.caption).foregroundStyle(.secondary)
                    }.padding(20).frame(width: 225, alignment: .leading).card()
                }
            } else if !model.busy {
                VStack(alignment: .leading, spacing: 16) {
                    Image(systemName: "play.rectangle.on.rectangle").font(.system(size: 40)).foregroundStyle(.teal)
                    Text("Load a recorded scenario").font(.title2.bold())
                    Text("Build the shared controller, choose this project folder, then open a replay. Results come from Rust; this interface contains no fishing logic.")
                        .foregroundStyle(.secondary)
                    Button("Load normal cycle") { model.loadSample() }.buttonStyle(.borderedProminent)
                }.padding(32).frame(maxWidth: .infinity, alignment: .leading).card()
            }
        }
    }

    private var readiness: some View {
        VStack(alignment: .leading, spacing: 18) {
            Text("Ready to compare").font(.largeTitle.bold())
            Text("These checks concern the prototype. They are not permission checks for live automation.")
                .foregroundStyle(.secondary)
            statusRow("Shared controller", detail: model.controllerBuilt ? model.executable.path : "Build fishing-core-cli from the project root.", ready: model.controllerBuilt)
            statusRow("Replay scenario", detail: model.replay?.name ?? "No validated replay loaded.", ready: model.replay != nil)
            statusRow("Protocol", detail: model.replay == nil ? "Checked when a replay loads." : "Version 1 decoded successfully.", ready: model.replay != nil)
            HStack(alignment: .top, spacing: 12) {
                Image(systemName: "minus.circle").foregroundStyle(.secondary)
                VStack(alignment: .leading, spacing: 5) {
                    Text("Live capture and input").font(.headline)
                    Text("Not implemented in this comparison app. Screen Recording and Accessibility access are not requested.")
                        .foregroundStyle(.secondary)
                }
            }.padding(20).frame(maxWidth: .infinity, alignment: .leading).card()
            Text("cargo build -p fishing-core --bin fishing-core-cli")
                .font(.body.monospaced()).textSelection(.enabled).padding(16).card()
        }
    }

    private var services: some View {
        VStack(alignment: .leading, spacing: 20) {
            Text("Native services evaluation").font(.largeTitle.bold())
            Text("Separate command-line experiments are included with this prototype. This page does not start them.")
                .foregroundStyle(.secondary)
            serviceCard("Apple Vision", symbol: "text.viewfinder", description: "Measures Energy OCR on supplied PNGs, with the same crop and optional threshold treatment as the current detector. Results are stored in the repository evaluation report.")
            serviceCard("ScreenCaptureKit", symbol: "macwindow", description: "Can list capturable windows and capture one explicitly selected window through the native-services command. Live capture remains unmeasured; it is never started automatically.")
            serviceCard("Liquid Glass", symbol: "rectangle.on.rectangle", description: "The replay badge uses the native glass effect on macOS 26 and later, with material fallback. Diagnostic surfaces stay solid and readable. Reduced Transparency is respected.")
            Button("Open evaluation report") {
                NSWorkspace.shared.open(model.root.appendingPathComponent("docs/native-evaluation.md"))
            }
        }
    }

    private func metric(_ title: String, value: UInt64, symbol: String) -> some View {
        HStack { Image(systemName: symbol).foregroundStyle(.teal); Spacer(); Text(value.formatted()).font(.title.bold()).monospacedDigit() }
            .overlay(alignment: .bottomLeading) { EmptyView() }
            .padding(.bottom, 24)
            .overlay(alignment: .bottomLeading) { Text(title).font(.caption).foregroundStyle(.secondary) }
    }
    private func statusRow(_ title: String, detail: String, ready: Bool) -> some View {
        HStack(alignment: .top, spacing: 12) {
            Image(systemName: ready ? "checkmark.circle.fill" : "exclamationmark.circle")
                .foregroundStyle(ready ? Color.teal : .orange)
            VStack(alignment: .leading, spacing: 5) {
                Text(title).font(.headline)
                Text(detail).font(.callout).foregroundStyle(.secondary).textSelection(.enabled)
            }
        }.padding(20).frame(maxWidth: .infinity, alignment: .leading).card()
    }
    private func serviceCard(_ title: String, symbol: String, description: String) -> some View {
        HStack(alignment: .top, spacing: 16) {
            Image(systemName: symbol).font(.title2).foregroundStyle(.teal).frame(width: 30)
            VStack(alignment: .leading, spacing: 7) {
                Text(title).font(.headline)
                Text(description).foregroundStyle(.secondary)
            }
        }.padding(22).frame(maxWidth: .infinity, alignment: .leading).card()
    }
}

struct GlassBadge: ViewModifier {
    @Environment(\.accessibilityReduceTransparency) var reduceTransparency
    @ViewBuilder func body(content: Content) -> some View {
        if reduceTransparency {
            content.background(Color(nsColor: .controlBackgroundColor), in: Capsule())
        } else if #available(macOS 26.0, *) {
            content.glassEffect(.regular.tint(.teal.opacity(0.12)), in: Capsule())
        } else {
            content.background(.regularMaterial, in: Capsule())
        }
    }
}

extension View {
    func card() -> some View {
        self.background(Color(nsColor: .controlBackgroundColor), in: RoundedRectangle(cornerRadius: 16))
            .overlay { RoundedRectangle(cornerRadius: 16).strokeBorder(.primary.opacity(0.06), lineWidth: 1) }
    }
}
