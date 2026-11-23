import ApplicationServices
import CoreGraphics
import Foundation

guard CommandLine.arguments.count == 2 else {
    fputs("usage: bench-terminal-scroll SECONDS|--until-eof\n", stderr)
    exit(2)
}
let untilEOF = CommandLine.arguments[1] == "--until-eof"
let duration = Double(CommandLine.arguments[1]) ?? 0
guard untilEOF || (duration.isFinite && duration > 0) else { exit(2) }
guard AXIsProcessTrusted() else {
    fputs("scroll helper lacks Accessibility permission\n", stderr)
    exit(1)
}

let finished = DispatchSemaphore(value: 0)
if untilEOF {
    Thread.detachNewThread {
        _ = FileHandle.standardInput.readDataToEndOfFile()
        finished.signal()
    }
}
let end = untilEOF ? Double.infinity : ProcessInfo.processInfo.systemUptime + duration
var step = 0
while ProcessInfo.processInfo.systemUptime < end {
    if finished.wait(timeout: .now()) == .success { break }
    let delta: Int32 = step % 200 < 100 ? 3 : -3
    guard let event = CGEvent(scrollWheelEvent2Source: nil, units: .line,
                              wheelCount: 1, wheel1: delta, wheel2: 0, wheel3: 0) else {
        fputs("could not create scroll event\n", stderr)
        exit(1)
    }
    event.location = CGPoint(x: 700, y: 450)
    event.post(tap: .cghidEventTap)
    step += 1
    Thread.sleep(forTimeInterval: 0.02)
}
