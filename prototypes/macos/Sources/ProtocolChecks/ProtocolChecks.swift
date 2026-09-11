import Foundation
import ReplayProtocol

// Also works with Command Line Tools, which do not bundle XCTest.
@main struct ProtocolChecks {
    static func require(_ condition: Bool, _ message: String) throws {
        guard condition else { throw ReplayError.message(message) }
    }
    static func rejects(_ body: () throws -> Void) throws {
        var rejected = false
        do { try body() } catch { rejected = true }
        try require(rejected, "Expected invalid protocol input to be rejected")
    }
    static func main() throws {
        let data = Data(#"{"protocol_version":1,"kind":"replay","name":"Cycle","frames":[{"at_ms":200,"phase":"casting","actions":["click",{"select_slot":4}],"fish":0,"feeds":0,"reason":"Rod selected"}]}"#.utf8)
        let replay = try ReplayEnvelope.decode(data)
        try require(replay.frames[0].actions == [.click, .selectSlot(4)], "Rust actions did not decode")
        try require(replay.frames[0].phaseTitle == "Casting", "Phase label did not decode")
        for json in [
            #"{"protocol_version":2,"kind":"replay","name":"Cycle","frames":[]}"#,
            #"{"protocol_version":1,"kind":"replay","name":"Cycle","frames":[]}"#,
            String(decoding: data, as: UTF8.self).replacingOccurrences(of: "\"kind\":\"replay\"", with: "\"kind\":\"controller\"")
        ] { try rejects { _ = try ReplayEnvelope.decode(Data(json.utf8)) } }
        for json in [#""hold_mouse""#, #"{"select_slot":99}"#] {
            try rejects { _ = try JSONDecoder().decode(ReplayAction.self, from: Data(json.utf8)) }
        }
        let root = URL(fileURLWithPath: CommandLine.arguments.dropFirst().first ?? FileManager.default.currentDirectoryPath)
        let actual = try ReplayRunner.run(executable: root.appendingPathComponent("target/debug/fishing-core-cli"), scenario: root.appendingPathComponent("tests/replays/normal-cycle.json"))
        try require(actual.frames.last?.fish == 1, "Actual Rust normal-cycle replay did not confirm one catch")
        let failure = try ReplayRunner.run(executable: root.appendingPathComponent("target/debug/fishing-core-cli"), scenario: root.appendingPathComponent("tests/replays/focus-loss.json"))
        try require(failure.frames.contains { $0.phase == "paused" }, "Actual Rust focus-loss replay did not pause")
        print("Protocol checks passed: actions, version/kind/empty validation, unsupported actions, actual Rust normal-cycle and focus-loss replays.")
    }
}
