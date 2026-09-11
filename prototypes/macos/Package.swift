// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "ArcaneNativeComparison",
    platforms: [.macOS(.v14)],
    products: [
        .executable(name: "ArcaneNative", targets: ["ArcaneNative"]),
        .executable(name: "native-services", targets: ["NativeServices"]),
        .executable(name: "protocol-checks", targets: ["ProtocolChecks"])
    ],
    targets: [
        .target(name: "ReplayProtocol"),
        .executableTarget(name: "ArcaneNative", dependencies: ["ReplayProtocol"]),
        .executableTarget(name: "NativeServices"),
        .executableTarget(name: "ProtocolChecks", dependencies: ["ReplayProtocol"])
    ]
)
