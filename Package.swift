// swift-tools-version: 6.2

import PackageDescription

let package = Package(
    name: "LyraEffectsStudio",
    platforms: [.macOS(.v14)],
    products: [
        .executable(name: "LyraEffectsStudio", targets: ["LyraEffectsStudio"]),
        .executable(name: "lyra-effects", targets: ["LyraEffectsCLIExecutable"]),
        .library(name: "LyraEffectsCLI", targets: ["LyraEffectsCLI"]),
        .library(name: "LyraPackKit", targets: ["LyraPackKit"]),
        .library(name: "LyraRegistryKit", targets: ["LyraRegistryKit"]),
        .library(name: "LyraProjectKit", targets: ["LyraProjectKit"]),
    ],
    targets: [
        .executableTarget(
            name: "LyraEffectsStudio",
            dependencies: ["LyraPackKit", "LyraRegistryKit", "LyraProjectKit"]
        ),
        .executableTarget(
            name: "LyraEffectsCLIExecutable",
            dependencies: ["LyraEffectsCLI"],
            path: "Sources/LyraEffectsCLIExecutable"
        ),
        .target(
            name: "LyraEffectsCLI",
            dependencies: ["LyraPackKit", "LyraRegistryKit", "LyraProjectKit"]
        ),
        .target(name: "LyraPackKit"),
        .target(name: "LyraRegistryKit", dependencies: ["LyraPackKit"]),
        .target(
            name: "LyraProjectKit",
            dependencies: ["LyraPackKit"],
            resources: [.process("Resources")]
        ),
        .testTarget(name: "LyraEffectsCLITests", dependencies: ["LyraEffectsCLI"]),
        .testTarget(name: "LyraPackKitTests", dependencies: ["LyraPackKit"]),
        .testTarget(name: "LyraRegistryKitTests", dependencies: ["LyraRegistryKit"]),
        .testTarget(name: "LyraProjectKitTests", dependencies: ["LyraProjectKit"]),
    ],
    swiftLanguageModes: [.v6]
)
