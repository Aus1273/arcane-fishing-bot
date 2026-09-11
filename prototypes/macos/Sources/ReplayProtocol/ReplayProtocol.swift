import Foundation
import Darwin

public enum ReplayAction: Decodable, Equatable, Sendable {
    case click
    case selectSlot(Int)

    public init(from decoder: Decoder) throws {
        let value = try decoder.singleValueContainer()
        if let string = try? value.decode(String.self), string == "click" {
            self = .click
        } else if let object = try? value.decode([String: Int].self),
                  object.count == 1, let slot = object["select_slot"], (0...9).contains(slot) {
            self = .selectSlot(slot)
        } else {
            throw DecodingError.dataCorruptedError(in: value, debugDescription: "Unsupported controller action")
        }
    }

    public var label: String {
        switch self {
        case .click: return "Would click once"
        case .selectSlot(let slot): return "Would select slot \(slot)"
        }
    }
}

public struct ReplayFrame: Decodable, Sendable {
    public let at_ms: UInt64
    public let phase: String
    public let actions: [ReplayAction]
    public let fish: UInt64
    public let feeds: UInt64
    public let reason: String

    public var phaseTitle: String {
        phase.replacingOccurrences(of: "_", with: " ").capitalized
    }
}

public struct ReplayEnvelope: Decodable, Sendable {
    public let protocol_version: Int
    public let kind: String
    public let name: String
    public let frames: [ReplayFrame]

    public static func decode(_ data: Data) throws -> ReplayEnvelope {
        let result = try JSONDecoder().decode(Self.self, from: data)
        guard result.protocol_version == 1, result.kind == "replay" else {
            throw ReplayError.message("Unsupported controller protocol. Expected version 1 replay output.")
        }
        guard !result.frames.isEmpty else { throw ReplayError.message("The replay contains no frames.") }
        guard zip(result.frames, result.frames.dropFirst()).allSatisfy({ $0.at_ms <= $1.at_ms }) else {
            throw ReplayError.message("The replay timeline moves backwards.")
        }
        return result
    }
}

public enum ReplayError: LocalizedError {
    case message(String)
    public var errorDescription: String? {
        switch self { case .message(let text): return text }
    }
}

/// Runs the same pure Rust controller used by Tauri. File arguments never pass through a shell.
public enum ReplayRunner {
    public static func run(executable: URL, scenario: URL) throws -> ReplayEnvelope {
        guard FileManager.default.isExecutableFile(atPath: executable.path) else {
            throw ReplayError.message("Build the shared controller first: cargo build -p fishing-core --bin fishing-core-cli")
        }
        let folder = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: folder) }
        let output = folder.appendingPathComponent("stdout")
        let errors = folder.appendingPathComponent("stderr")
        FileManager.default.createFile(atPath: output.path, contents: nil)
        FileManager.default.createFile(atPath: errors.path, contents: nil)
        let stdout = try FileHandle(forWritingTo: output)
        let stderr = try FileHandle(forWritingTo: errors)
        defer { try? stdout.close(); try? stderr.close() }
        let process = Process()
        process.executableURL = executable
        process.arguments = ["replay", scenario.path]
        process.standardOutput = stdout
        process.standardError = stderr
        try process.run()
        let deadline = Date().addingTimeInterval(15)
        while process.isRunning && Date() < deadline { Thread.sleep(forTimeInterval: 0.02) }
        if process.isRunning {
            kill(process.processIdentifier, SIGKILL)
            process.waitUntilExit()
            throw ReplayError.message("The controller replay exceeded 15 seconds.")
        }
        process.waitUntilExit()
        guard process.terminationStatus == 0 else {
            let text = (try? String(contentsOf: errors, encoding: .utf8)) ?? "No error output"
            throw ReplayError.message(String(text.prefix(4000)))
        }
        let attributes = try FileManager.default.attributesOfItem(atPath: output.path)
        guard (attributes[.size] as? NSNumber)?.intValue ?? 0 <= 16_000_000 else {
            throw ReplayError.message("Replay output exceeds 16 MB.")
        }
        return try ReplayEnvelope.decode(Data(contentsOf: output))
    }
}
