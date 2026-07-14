import SwiftUI

@main
struct LyraEffectsStudioApp: App {
    var body: some Scene {
        WindowGroup {
            ContentView()
        }
    }
}

private struct ContentView: View {
    var body: some View {
        VStack(spacing: 12) {
            Image(systemName: "waveform.path")
                .font(.system(size: 42))
            Text("Lyra Effects Studio")
                .font(.title2)
            Text("Theme Pack tooling is ready.")
                .foregroundStyle(.secondary)
        }
        .frame(minWidth: 720, minHeight: 480)
    }
}
