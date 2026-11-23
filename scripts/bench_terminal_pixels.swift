import CoreMedia
import CoreVideo
import Foundation
import ScreenCaptureKit

final class Pixels: NSObject, SCStreamOutput {
    private var previous: [UInt8]?

    func stream(_ stream: SCStream, didOutputSampleBuffer sample: CMSampleBuffer,
                of outputType: SCStreamOutputType) {
        guard outputType == .screen,
              let attachments = CMSampleBufferGetSampleAttachmentsArray(sample, createIfNecessary: false)
                as? [[SCStreamFrameInfo: Any]],
              let raw = attachments.first?[.status] as? Int,
              SCFrameStatus(rawValue: raw) == .complete,
              let buffer = CMSampleBufferGetImageBuffer(sample) else { return }
        CVPixelBufferLockBaseAddress(buffer, .readOnly)
        defer { CVPixelBufferUnlockBaseAddress(buffer, .readOnly) }
        guard let address = CVPixelBufferGetBaseAddress(buffer) else { return }
        let bytes = address.assumingMemoryBound(to: UInt8.self)
        let width = CVPixelBufferGetWidth(buffer)
        let height = CVPixelBufferGetHeight(buffer)
        let stride = CVPixelBufferGetBytesPerRow(buffer)
        var current = [UInt8](repeating: 0, count: width * height * 4)
        var changed = 0
        for row in 0..<height {
            for column in 0..<width {
                var differs = false
                for channel in 0..<4 {
                    let index = (row * width + column) * 4 + channel
                    let value = bytes[row * stride + column * 4 + channel]
                    current[index] = value
                    if channel < 3, let previous,
                       abs(Int(value) - Int(previous[index])) > 8 {
                        differs = true
                    }
                }
                if differs { changed += 1 }
            }
        }
        if previous == nil {
            do {
                try Data(current).write(to: URL(fileURLWithPath: CommandLine.arguments[2]))
            } catch {
                fputs("cannot save the probe region: \(error)\n", stderr)
                exit(1)
            }
        }
        if previous == nil || changed > width * height / 200 {
            print("{\"time\":\(ProcessInfo.processInfo.systemUptime),\"changed_pixels\":\(changed)}")
            fflush(stdout)
            previous = current
        }
    }
}

@main
struct Probe {
    static func main() async throws {
        guard CommandLine.arguments.count == 3 else {
            fputs("usage: bench-terminal-pixels SECONDS|--until-eof FIRST_FRAME_BGRA\n", stderr)
            exit(2)
        }
        let untilEOF = CommandLine.arguments[1] == "--until-eof"
        let seconds = Double(CommandLine.arguments[1]) ?? 0
        guard untilEOF || (seconds.isFinite && seconds > 0 && seconds <= 3600) else { exit(2) }
        let content = try await SCShareableContent.excludingDesktopWindows(false, onScreenWindowsOnly: true)
        guard let display = content.displays.first else { throw CocoaError(.fileNoSuchFile) }
        let filter = SCContentFilter(display: display, excludingWindows: [])
        let config = SCStreamConfiguration()
        config.sourceRect = CGRect(x: 560, y: 220, width: 100, height: 300)
        config.width = 100
        config.height = 300
        config.pixelFormat = kCVPixelFormatType_32BGRA
        config.minimumFrameInterval = CMTime(value: 1, timescale: 60)
        config.showsCursor = false
        let output = Pixels()
        let queue = DispatchQueue(label: "benchmark.pixels")
        let stream = SCStream(filter: filter, configuration: config, delegate: nil)
        try stream.addStreamOutput(output, type: .screen, sampleHandlerQueue: queue)
        try await stream.startCapture()
        print("{\"start\":\(ProcessInfo.processInfo.systemUptime)}")
        fflush(stdout)
        if untilEOF {
            await withCheckedContinuation { (continuation: CheckedContinuation<Void, Never>) in
                DispatchQueue.global().async {
                    _ = FileHandle.standardInput.readDataToEndOfFile()
                    continuation.resume()
                }
            }
        } else {
            try await Task.sleep(nanoseconds: UInt64(seconds * 1_000_000_000))
        }
        try await stream.stopCapture()
        queue.sync {}
        print("{\"end\":\(ProcessInfo.processInfo.systemUptime)}")
    }
}
