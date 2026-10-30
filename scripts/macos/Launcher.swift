import AppKit
import Foundation
#if canImport(Sparkle)
import Sparkle
#endif

// The menu-bar app owns only its child engine. Quit sends SIGINT for a durable session flush.
final class AppDelegate: NSObject, NSApplicationDelegate {
    #if canImport(Sparkle)
    var updater: SPUStandardUpdaterController?
    #endif
    var engine: Process?
    var statusItem: NSStatusItem!
    var openItem: NSMenuItem!
    var quitting = false
    var ready = false
    var console = ""
    var log: FileHandle?
    var activity: NSObjectProtocol?
    let studio = URL(string: "http://127.0.0.1:8797/?view=studio")!

    func applicationDidFinishLaunching(_ notification: Notification) {
        statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
        statusItem.button?.title = "schéma"
        let menu = NSMenu()
        openItem = NSMenuItem(title: "Open Studio", action: #selector(openStudio), keyEquivalent: "o")
        openItem.target = self; openItem.isEnabled = false; menu.addItem(openItem)
        #if canImport(Sparkle)
        updater = SPUStandardUpdaterController(startingUpdater: true, updaterDelegate: nil, userDriverDelegate: nil)
        let downloads = NSMenuItem(title: "Check for Updates…", action: #selector(SPUStandardUpdaterController.checkForUpdates(_:)), keyEquivalent: "")
        downloads.target = updater
        #else
        let downloads = NSMenuItem(title: "Updates / Downloads…", action: #selector(openDownloads), keyEquivalent: "")
        downloads.target = self
        #endif
        menu.addItem(downloads)
        let login = NSMenuItem(title: "Start at Login…", action: #selector(loginHelp), keyEquivalent: "")
        login.target = self; menu.addItem(login)
        menu.addItem(.separator())
        let quit = NSMenuItem(title: "Quit Schéma-s", action: #selector(quitApp), keyEquivalent: "q")
        quit.target = self; menu.addItem(quit); statusItem.menu = menu
        NSApp.setActivationPolicy(.accessory)
        activity = ProcessInfo.processInfo.beginActivity(options: [.userInitiated, .latencyCritical], reason: "Run Schéma lighting engine")
        do {
            let logs = FileManager.default.urls(for: .libraryDirectory, in: .userDomainMask)[0].appendingPathComponent("Logs/Schema-s")
            try FileManager.default.createDirectory(at: logs, withIntermediateDirectories: true)
            let file = logs.appendingPathComponent("engine.log")
            if !FileManager.default.fileExists(atPath: file.path) { FileManager.default.createFile(atPath: file.path, contents: nil) }
            log = try FileHandle(forWritingTo: file); log?.seekToEndOfFile()
            let child = Process(), pipe = Pipe()
            child.executableURL = Bundle.main.bundleURL.appendingPathComponent("Contents/MacOS/schema-rs")
            child.arguments = ["desktop", "--no-open", "--bind", "127.0.0.1:8797"]
            child.standardOutput = pipe; child.standardError = pipe
            pipe.fileHandleForReading.readabilityHandler = { [weak self] handle in
                let data = handle.availableData
                if data.isEmpty { handle.readabilityHandler = nil; return }
                DispatchQueue.main.async {
                    guard let self = self else { return }
                    self.log?.write(data); self.console += String(decoding: data, as: UTF8.self)
                    if !self.ready && self.console.contains("engine: http://127.0.0.1:8797") {
                        self.ready = true; self.openItem.isEnabled = true; self.openStudio()
                    }
                    self.console = String(self.console.suffix(4096))
                }
            }
            child.terminationHandler = { [weak self] process in
                DispatchQueue.main.async {
                    guard let self = self else { return }
                    if self.quitting { NSApp.reply(toApplicationShouldTerminate: true) }
                    else { self.fail("Engine stopped (status \(process.terminationStatus)). Check ~/Library/Logs/Schema-s/engine.log. Another app or login service may already use port8797.") }
                }
            }
            engine = child; try child.run()
        } catch { fail(error.localizedDescription) }
    }
    @objc func openStudio() { if ready { NSWorkspace.shared.open(studio) } }
    @objc func openDownloads() { NSWorkspace.shared.open(URL(string: "https://schema-s.vercel.app/#download")!) }
    @objc func loginHelp() {
        let alert = NSAlert(); alert.messageText = "Start Schéma-s at login"
        alert.informativeText = "Add Schéma-s in System Settings → General → Login Items. The app remains in the menu bar; Quit stops its engine and saves the session. For a headless service, see the included macOS guide."
        alert.runModal()
    }
    @objc func quitApp() { NSApp.terminate(nil) }
    func applicationShouldTerminate(_ sender: NSApplication) -> NSApplication.TerminateReply {
        if let child = engine, child.isRunning { quitting = true; child.interrupt(); return .terminateLater }
        return .terminateNow
    }
    func applicationWillTerminate(_ notification: Notification) {
        if let token = activity { ProcessInfo.processInfo.endActivity(token) }
        try? log?.close()
    }
    func applicationShouldHandleReopen(_ sender: NSApplication, hasVisibleWindows flag: Bool) -> Bool { openStudio(); return false }
    func fail(_ message: String) {
        let alert = NSAlert(); alert.messageText = "Schéma-s could not run"; alert.informativeText = message; alert.runModal()
        quitting = false; NSApp.terminate(nil)
    }
}
let application = NSApplication.shared
let delegate = AppDelegate()
application.delegate = delegate
withExtendedLifetime(delegate) { application.run() }
