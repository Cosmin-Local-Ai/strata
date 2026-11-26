# Changelog

## 0.3.1 (26 Nov 2026)

### Improved
- Notifications are tinted for your dock style (Liquid Glass, Black, White…) and stack at the right.

### Fixed
- Two notifications arriving close together could leave none showing.

## 0.3.0 (26 Nov 2026)

### New
- **Strata notifications** (Settings → Notifications): notifications from your apps appear next to the dock, at the right, in Strata's style instead of Windows' pop-ups, and the Notifications widget opens them (plus Strata's own) in a Strata list. Windows' pop-ups are switched off per app with the same switch Windows Settings uses; turn Strata notifications off and every app's own settings, and the sounds, are put back. Optionally silence Windows' notification sounds.
- **Smart notifications**: a heads-up once per event: a calendar event starts in 10 minutes, rain is likely around an event in the next 3 hours (from your Weather widget's place), a to-do's time has come, an AI usage limit passed 80 % or 95 %, a drive is almost full.
- **Storage** widget: free space up front, a colored bar of what fills the drive and a list of its biggest folders and files (scanned when you ask, kept 5 minutes; click one to open it), and your other drives one click away.
- **Drop to pin**: drag a file, folder, shortcut or program onto your pinned apps to pin it there.
- **To-dos with a time**: add one with the clock button or by typing it at the end ("Call the bank 16:00"); the time shows as a tag and turns red when it has passed.
- **Project Launcher**: your projects are kept on their own, so removing the widget no longer loses them; hold a project for a moment, then drag it up or down to reorder.
- **komorebi arrange rules**: keep apps in fixed places of a workspace (e.g. browser in the middle, Discord left, Spotify right); applied when one of them opens.

### Improved
- **Vault ID documents**: shown in full (only passwords, card numbers, CVV and PIN are masked), with series, number, personal number (CNP), date and place of birth, sex, nationality, address, issued by, issue and expiry dates; every field has its own copy button. Country and notes fields removed (anything already saved in them still shows).
- **Show Desktop** catches every window (charts, tool palettes with a hidden owner), redraws windows as they come back (no more grey boxes) and keeps komorebi's tile order.
- **Currency converter**: the currency search looks like the rest of the card, with longer names.
- The komorebi rule that keeps Strata out of tiling is also added to komorebi.json (with a backup), so the dock no longer jumps into a tile for a moment at sign-in.
- Strata logs what it starts (apps, terminal commands, project commands), to trace a stray window.

### Fixed
- The dock could be treated as hidden by its web view on some PCs: animations stopped (Flip Clock stuck), the dock got laggy and cards opened slowly or cut off.
- Tray icons: clicks did nothing for many icons (Windows Security, Bluetooth, launchers); icons of apps that were closed or ended stayed listed.
- Microphone mute is logged per microphone, to find what still records when it shouldn't.

## 0.2.3 (23 Nov 2026)

### Fixed
- Cloud Storage: when Google sign-in works but the Google Drive API is turned off in your Google Cloud project, the card now says so, with a button to the page that turns it on (it used to ask you to sign in again, which didn't help). The full message is shown instead of being cut off.
- Cloud Storage: a sign-in done while the card was closed shows up as soon as you open it again (accounts that aren't connected are re-checked on every open).
- Cloud Storage square tile: the usage bars no longer run into the cloud icon.
- Services in general: when a service refuses a key, its own reason is shown along with the message.

## 0.2.2 (23 Nov 2026)

### New
- **Middle-click an app for a new window**: works for browsers (Chrome, Edge, Brave, Vivaldi, Opera, Firefox), VS Code / Cursor, Windows Terminal and Store apps too, on running apps, pinned apps and the "New window" menu item. Pinned apps with your own middle-click arguments keep them.

### Fixed
- Terminal: "Run as administrator" opened an empty admin terminal without your command, and afterwards "Run again" (and normal runs) could open a bare terminal too. Admin commands now run in an elevated PowerShell / CMD in the chosen folder; normal runs open as tabs in Strata's own Terminal window.
- Cloud Storage: the Digi Storage sign-in boxes are labelled, show where you're typing, aren't repainted by Windows' autofill, and Enter connects.

## 0.2.1 (23 Nov 2026)

### New
- **Updates without opening Settings**: Strata checks for a new version when it starts and every 6 hours and shows a message on the dock; click it to see what changed and install. Settings → About → Update automatically: Tell me (default), Install (downloads, installs and restarts by itself) or Off.
- **Vault: forgot your master password?** Confirm it's you with Windows Hello (face, fingerprint or PIN) and the master password is removed; your items stay (they are encrypted with your Windows account), and you can set a new one.
- **Start with Windows** stays listed in Task Manager → Startup apps and Settings → Apps → Startup even when it's off, like other programs; switching it there or in Strata is the same switch.

### Fixed
- Clicking an app that Windows couldn't open froze the whole dock until Strata was ended in Task Manager. The error now shows on the dock for a few seconds instead of in a blocking dialog.
- Store apps that update themselves into a new folder (the Claude app, for example) showed a letter instead of their icon and wouldn't open ("code 3"). They now open through their Start menu entry and keep their icon.
- Mute microphone now mutes every microphone, so apps that record from their own device (Discord, Teams, games) go silent too. Unmuting leaves microphones you had muted yourself alone.
- Hidden taskbar: Windows kept bringing it back (after the dock changed size, settings changes, at sign-in). Strata now hides it again within a fraction of a second, and switches auto-hide back on if something turns it off.
- Times follow the Windows clock setting (24-hour or AM/PM) instead of the display language: power menu, calendar, weather, meeting notes and clocks.
- Flip Clock could show only the hours, or no digits at all.
- Cards that fill in after opening (Screenshots, Downloads, Recently opened) could open cut off at the top part until opened a second time.
- Updates: if installing fails, Settings shows why and offers the download page.
- The log keeps up to 3 MB instead of 40 KB and no longer fills with music visualiser lines, so a log sent with a problem report reaches back to startup.

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
