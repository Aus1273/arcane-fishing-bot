import AppKit
import Foundation
import Darwin
import ImageIO
import ScreenCaptureKit
import UniformTypeIdentifiers
import Vision

enum ServiceError: LocalizedError {
    case message(String)
    var errorDescription: String? { if case .message(let text) = self { return text }; return nil }
}

struct OCRSample: Codable {
    let text: String
    let confidence: Float?
    let usable: UInt32?
    let capacity: UInt32?
    let elapsed_ms: Double
}

@main
struct NativeServices {
    static func main() async {
        do {
            let args = Array(CommandLine.arguments.dropFirst())
            guard let command = args.first else { printHelp(); return }
            switch command {
            case "ocr": try ocr(Array(args.dropFirst()))
            case "capture": try await capture(Array(args.dropFirst()))
            case "--help", "help": printHelp()
            default: throw ServiceError.message("Unknown command. Run native-services --help.")
            }
        } catch {
            FileHandle.standardError.write(Data((error.localizedDescription + "\n").utf8))
            exit(1)
        }
    }

    static func printHelp() {
        print("""
        Read-only native service evaluation. No mouse/keyboard input is implemented.

        native-services ocr FILE.png [--engine vision|tesseract] [--region x,y,width,height]
          [--prepare raw|threshold] [--iterations 5] [--warmup 1] [--tesseract /absolute/path]
        native-services capture --list-windows
        native-services capture --window-id ID --output /absolute/new-file.png

        OCR uses local PNG files only. Capture commands explicitly request ScreenCaptureKit
        access; no capture happens on launch, --help, or OCR. Capture excludes audio and cursor.
        Region coordinates are source-image pixels from the top-left. Default Energy region:
        1988,1707,101,22, matching the supplied 3024x1964 MacBook screenshots.
        """)
    }

    static func parseOptions(_ args: [String], allowed: Set<String>) throws -> [String: String] {
        var options: [String: String] = [:]
        var index = 0
        while index < args.count {
            let key = args[index]
            guard allowed.contains(key), index + 1 < args.count, options[key] == nil else {
                throw ServiceError.message("Invalid or duplicated option: \(key)")
            }
            options[key] = args[index + 1]
            index += 2
        }
        return options
    }

    static func ocr(_ args: [String]) throws {
        guard let path = args.first, !path.hasPrefix("--") else { throw ServiceError.message("Provide a PNG path.") }
        let options = try parseOptions(Array(args.dropFirst()), allowed: ["--engine", "--region", "--prepare", "--iterations", "--warmup", "--tesseract"])
        let engine = options["--engine"] ?? "vision"
        let prepare = options["--prepare"] ?? "threshold"
        guard ["vision", "tesseract"].contains(engine), ["raw", "threshold"].contains(prepare),
              let count = Int(options["--iterations"] ?? "5"), (1...100).contains(count),
              let warmup = Int(options["--warmup"] ?? "1"), (0...10).contains(warmup) else {
            throw ServiceError.message("Invalid engine, preparation, iteration count or warmup count.")
        }
        let coordinates = (options["--region"] ?? "1988,1707,101,22").split(separator: ",").compactMap { Int($0) }
        guard coordinates.count == 4, coordinates[0] >= 0, coordinates[1] >= 0,
              coordinates[2] > 0, coordinates[3] > 0,
              coordinates[2] <= 1000, coordinates[3] <= 1000 else {
            throw ServiceError.message("Expected x,y,width,height with a positive crop no larger than 1000×1000.")
        }
        let url = URL(fileURLWithPath: path)
        guard let source = CGImageSourceCreateWithURL(url as CFURL, nil),
              let properties = CGImageSourceCopyPropertiesAtIndex(source, 0, nil) as? [CFString: Any],
              let width = properties[kCGImagePropertyPixelWidth] as? Int,
              let height = properties[kCGImagePropertyPixelHeight] as? Int,
              width > 0, height > 0, width <= 16384, height <= 16384, width * height <= 16_000_000,
              coordinates[0] <= width - coordinates[2], coordinates[1] <= height - coordinates[3],
              let original = CGImageSourceCreateImageAtIndex(source, 0, nil),
              let cropped = original.cropping(to: CGRect(x: coordinates[0], y: coordinates[1], width: coordinates[2], height: coordinates[3])) else {
            throw ServiceError.message("Cannot decode the image or the crop is outside its dimensions.")
        }
        let image = prepare == "threshold" ? try threshold(cropped) : cropped
        let executable = options["--tesseract"] ?? resolveTesseract()
        var samples: [OCRSample] = []
        var warmups: [OCRSample] = []
        for index in 0..<(warmup + count) {
            let start = DispatchTime.now().uptimeNanoseconds
            let result: (String, Float?)
            if engine == "vision" { result = try recognize(image) }
            else { result = (try tesseract(image, executable: executable), nil) }
            let elapsed = Double(DispatchTime.now().uptimeNanoseconds - start) / 1_000_000
            let values = parseEnergy(result.0)
            let sample = OCRSample(text: result.0, confidence: result.1, usable: values?.0, capacity: values?.1, elapsed_ms: elapsed)
            if index < warmup { warmups.append(sample) } else { samples.append(sample) }
        }
        let encoder = JSONEncoder()
        let sampleObjects = try JSONSerialization.jsonObject(with: encoder.encode(samples))
        let warmupObjects = try JSONSerialization.jsonObject(with: encoder.encode(warmups))
        try printJSON([
            "schema_version": 1, "engine": engine, "preparation": prepare,
            "source_file": url.lastPathComponent, "source_width": width, "source_height": height,
            "region": coordinates, "ocr_width": image.width, "ocr_height": image.height,
            "recognition_level": "accurate", "language": "en-US", "samples": sampleObjects,
            "warmup_samples": warmupObjects,
            "timing_scope": engine == "vision" ? "request construction + synchronous Vision recognition" : "temporary PNG creation + Tesseract process launch, recognition and wait"
        ])
    }

    static func parseEnergy(_ text: String) -> (UInt32, UInt32)? {
        let cleaned = text.filter { !$0.isWhitespace }
        let parts = cleaned.split(separator: "/", omittingEmptySubsequences: false)
        guard parts.count == 2, let usable = UInt32(parts[0]), let capacity = UInt32(parts[1]),
              capacity > 0, usable <= capacity, capacity <= 100_000 else { return nil }
        return (usable, capacity)
    }

    static func recognize(_ image: CGImage) throws -> (String, Float?) {
        let request = VNRecognizeTextRequest()
        request.recognitionLevel = .accurate
        request.usesLanguageCorrection = false
        request.recognitionLanguages = ["en-US"]
        request.minimumTextHeight = 0
        try VNImageRequestHandler(cgImage: image, options: [:]).perform([request])
        let results = (request.results ?? []).compactMap { $0.topCandidates(1).first }
        return (results.map(\.string).joined(separator: " "), results.map(\.confidence).min())
    }

    /// Matches Rust energy_ocr_image: R/G > 165 becomes black; 4× nearest scale and 20 px border.
    static func threshold(_ image: CGImage) throws -> CGImage {
        let width = image.width, height = image.height
        var rgba = [UInt8](repeating: 0, count: width * height * 4)
        let okay = rgba.withUnsafeMutableBytes { bytes -> Bool in
            guard let context = CGContext(data: bytes.baseAddress, width: width, height: height,
                                          bitsPerComponent: 8, bytesPerRow: width * 4,
                                          space: CGColorSpaceCreateDeviceRGB(),
                                          bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue | CGBitmapInfo.byteOrder32Big.rawValue) else { return false }
            context.draw(image, in: CGRect(x: 0, y: 0, width: width, height: height))
            return true
        }
        guard okay else { throw ServiceError.message("Could not create preprocessing context.") }
        let targetWidth = width * 4 + 40, targetHeight = height * 4 + 40
        var gray = [UInt8](repeating: 255, count: targetWidth * targetHeight)
        for y in 0..<height {
            for x in 0..<width {
                let value: UInt8 = rgba[(y * width + x) * 4] > 165 && rgba[(y * width + x) * 4 + 1] > 165 ? 0 : 255
                for dy in 0..<4 { for dx in 0..<4 { gray[(y * 4 + 20 + dy) * targetWidth + x * 4 + 20 + dx] = value } }
            }
        }
        guard let provider = CGDataProvider(data: Data(gray) as CFData),
              let result = CGImage(width: targetWidth, height: targetHeight, bitsPerComponent: 8, bitsPerPixel: 8,
                                   bytesPerRow: targetWidth, space: CGColorSpaceCreateDeviceGray(),
                                   bitmapInfo: CGBitmapInfo(rawValue: 0), provider: provider, decode: nil,
                                   shouldInterpolate: false, intent: .defaultIntent) else {
            throw ServiceError.message("Could not create the prepared image.")
        }
        return result
    }

    static func resolveTesseract() -> String {
        let folders = ["/opt/homebrew/bin", "/usr/local/bin"] + (ProcessInfo.processInfo.environment["PATH"] ?? "").split(separator: ":").map(String.init)
        return folders.map { $0 + "/tesseract" }.first { FileManager.default.isExecutableFile(atPath: $0) } ?? "/usr/bin/tesseract"
    }

    static func tesseract(_ image: CGImage, executable: String) throws -> String {
        let folder = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: folder) }
        let input = folder.appendingPathComponent("energy.png")
        try savePNG(image, to: input)
        let process = Process()
        process.executableURL = URL(fileURLWithPath: executable)
        process.arguments = [input.path, "stdout", "-l", "eng", "--psm", "7", "--dpi", "150", "-c", "tessedit_char_whitelist=0123456789/"]
        let pipe = Pipe()
        process.standardOutput = pipe
        process.standardError = FileHandle.nullDevice
        try process.run()
        let deadline = Date().addingTimeInterval(2.5)
        while process.isRunning && Date() < deadline { Thread.sleep(forTimeInterval: 0.005) }
        if process.isRunning {
            kill(process.processIdentifier, SIGKILL)
            process.waitUntilExit()
            throw ServiceError.message("Tesseract exceeded 2.5 seconds.")
        }
        process.waitUntilExit()
        guard process.terminationStatus == 0 else { throw ServiceError.message("Tesseract failed.") }
        return String(decoding: pipe.fileHandleForReading.readDataToEndOfFile(), as: UTF8.self).trimmingCharacters(in: .whitespacesAndNewlines)
    }

    static func capture(_ args: [String]) async throws {
        guard args == ["--list-windows"] || args.contains("--window-id") else {
            throw ServiceError.message("Capture requires --list-windows or --window-id ID --output NEWFILE.")
        }
        // Permission acquisition only occurs after an explicit capture subcommand.
        let options = args == ["--list-windows"] ? [:] : try parseOptions(args, allowed: ["--window-id", "--output"])
        var captureID: UInt32?
        var captureOutput: URL?
        if !options.isEmpty {
            guard let value = options["--window-id"], let id = UInt32(value), let output = options["--output"],
                  output.hasPrefix("/"), !FileManager.default.fileExists(atPath: output) else {
                throw ServiceError.message("Provide a numeric window ID and a new absolute output path. Existing files are not overwritten.")
            }
            captureID = id
            captureOutput = URL(fileURLWithPath: output)
        }
        let content = try await SCShareableContent.excludingDesktopWindows(true, onScreenWindowsOnly: true)
        if args == ["--list-windows"] {
            try printJSON(["schema_version": 1, "windows": content.windows.map {
                ["id": $0.windowID, "application": $0.owningApplication?.applicationName ?? "Unknown", "title": $0.title ?? "", "width_points": $0.frame.width, "height_points": $0.frame.height] as [String: Any]
            }])
            return
        }
        guard let id = captureID, let output = captureOutput, let window = content.windows.first(where: { $0.windowID == id }) else {
            throw ServiceError.message("The selected window is no longer available.")
        }
        let filter = SCContentFilter(desktopIndependentWindow: window)
        let configuration = SCStreamConfiguration()
        configuration.width = max(1, Int(filter.contentRect.width * CGFloat(filter.pointPixelScale)))
        configuration.height = max(1, Int(filter.contentRect.height * CGFloat(filter.pointPixelScale)))
        configuration.showsCursor = false
        configuration.capturesAudio = false
        let started = DispatchTime.now().uptimeNanoseconds
        let image = try await SCScreenshotManager.captureImage(contentFilter: filter, configuration: configuration)
        let elapsed = Double(DispatchTime.now().uptimeNanoseconds - started) / 1_000_000
        try savePNG(image, to: output)
        try printJSON(["schema_version": 1, "window_id": id, "width": image.width, "height": image.height, "capture_ms": elapsed, "output": output.path])
    }

    static func savePNG(_ image: CGImage, to output: URL) throws {
        guard let destination = CGImageDestinationCreateWithURL(output as CFURL, UTType.png.identifier as CFString, 1, nil) else {
            throw ServiceError.message("Cannot create the PNG destination.")
        }
        CGImageDestinationAddImage(destination, image, nil)
        guard CGImageDestinationFinalize(destination) else { throw ServiceError.message("Could not write PNG.") }
    }

    static func printJSON(_ object: [String: Any]) throws {
        let data = try JSONSerialization.data(withJSONObject: object, options: [.prettyPrinted, .sortedKeys])
        print(String(decoding: data, as: UTF8.self))
    }
}
