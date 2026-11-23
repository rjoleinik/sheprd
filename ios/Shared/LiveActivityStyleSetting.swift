import Foundation
import SheprdRemoteCore

/// The Live Activity style chosen in Settings, kept in the App Group's
/// defaults so the widget extension draws with it. It replaced Keep Generic
/// (`legacyKey`, in the app's own defaults): a phone that had it on starts
/// in Private.
enum LiveActivityStyleSetting {
    static let key = "liveActivityStyle"
    /// Keep Generic's key, read once to carry its choice over.
    static let legacyKey = "liveActivitiesGeneric"
    static var store: UserDefaults? { UserDefaults(suiteName: NotifyKey.group) }

    static var current: LiveActivityStyle {
        LiveActivityStyle.stored(store?.string(forKey: key))
    }

    /// Stores the style Keep Generic implies when none is stored yet. The
    /// app calls it at launch, before registering with the computers; only
    /// the app can read its own defaults, where Keep Generic lived.
    static func migrate() {
        guard let store, store.string(forKey: key) == nil else { return }
        let style = LiveActivityStyle.stored(nil, legacyGeneric: UserDefaults.standard.bool(forKey: legacyKey))
        store.set(style.rawValue, forKey: key)
    }
}
