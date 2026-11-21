# Changelog

## 0.2.0 (21 Nov 2026)

### New widgets
- **Start Search**: search apps, Settings pages, documents and the web, like the search in Start. Arrow keys and Enter; Ctrl+Enter opens a file's folder or runs an app as administrator. Replaces the old Commands widget (docks that had Commands get it automatically).
- **Terminal**: type a command and it opens in Windows Terminal already running. Choose PowerShell 7, Windows PowerShell or CMD and a start folder; Ctrl+Enter runs it as administrator; recent commands are one click away.
- **Project Launcher**: start, open and stop the apps you run yourself, all in one card. Add an app installed on your PC, a project from a GitHub link (downloaded with git, nothing is run) or a project folder; Strata suggests how to start it (its own start/stop script, Docker Compose, launch.bat, npm or Python) and where it listens. Nothing is added unless you add it. Without a stop command it stops whatever listens on the project's port, or closes the app.
- **Cloud Storage** in three sizes (square, wide, large): Google Drive and Digi Storage together. The card shows space used and free and the latest files; click an account to open it on the web. Separate **Google Drive** and **Digi Storage** widgets too.
- **Microsoft Office** and **Google Workspace**: one tile that opens every app, quick create (Google), pinned and recent documents. Right-click → Show app icons for one-click app buttons on the dock instead.
- **Meeting Notes**: agenda, notes, action items with owners and a timer; copy or save as Markdown.
- New looks: **Weather Card**, **Time, Date & Weather**, **Now Playing Art**.

### Improved
- **Vault**: secure notes, cards, Wi-Fi, ID documents and encrypted files besides logins and API keys, each with only the fields it needs. Import passwords from Google, Apple (iCloud Keychain) or Vivaldi / Chrome / Edge (CSV), export, a health report (weak, reused, old) with an optional breach check that never sends your passwords, and an optional master password that asks again after the time you choose.
- **Now Playing** remembers the last song after you close the player or restart; its play button reopens the player and resumes. "Click to open Spotify" only appears when Spotify is installed.
- **Show Desktop** minimises everything again (also with komorebi), so desktop widgets and apps komorebi doesn't tile are visible; press again to bring everything back.
- **AI usage** widgets refresh right after a limit resets (also early or trial resets) instead of showing old usage.
- **Updates** now show what changed before installing.
- The old **Search** widget is now **Web Search** with a globe icon, so it can't be confused with Start Search.
- Errors inside Strata are written to its log (%LOCALAPPDATA%\local.strata.dock\logs\Strata.log), handy when reporting a problem.

### Fixed
- Clicking an app could do nothing and freeze the whole dock while Windows was slow to start it.
- Apps that update themselves into a new folder (Discord, Slack, GitHub Desktop) lost their icon (showed a letter) and wouldn't open from a pinned tile.
- Padding and Spacing in Settings changed nothing.
- Magnified icons were cut off at the top of the dock.
- The glass could stay at an old size or position (icons outside the bar, a stretched bar) after changing size or magnification while a game or AI tool was running.
- The Minimal size let two-line tiles spill out of the bar.
- The power menu's Dim background did nothing.
- The dock could be picked up and moved by komorebi if both started at the same time at sign-in.
- The Mini and Bar Now Playing play buttons did nothing.

## 0.1.0 (17 Nov 2026)

First release.
