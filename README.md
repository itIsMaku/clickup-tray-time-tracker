<div align="center">

# ClickUp Tray Time Tracker

**Your next task is one click away.**

A small native ClickUp timer for the macOS menu bar and Windows system tray.
Built with Rust and GPUI Kit — no browser engine, webview, or companion server.

[![Build](https://github.com/itIsMaku/clickup-tray-time-tracker/actions/workflows/build.yml/badge.svg)](https://github.com/itIsMaku/clickup-tray-time-tracker/actions/workflows/build.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/built_with-Rust-dea584.svg)](https://www.rust-lang.org/)
[![GPUI Kit](https://img.shields.io/badge/UI-GPUI_Kit-1b9c77.svg)](https://gpui-kit.com/)

[Getting started](#getting-started) · [Build](#build-from-source) · [Contributing](CONTRIBUTING.md) · [Report a bug](https://github.com/itIsMaku/clickup-tray-time-tracker/issues/new/choose)

</div>

## What it does

- **Track from the tray.** Start, stop, or switch tasks without opening ClickUp.
- **Find your work.** Search assigned open tasks, including subtasks, and resume recent tasks. Load another accessible task by its ClickUp link or original ID.
- **Create and track.** Give a new task a name, pick its list, and start tracking immediately. The app assigns it to you and remembers your last list.
- **See your progress.** Weekly and monthly summaries show tracked hours, goal progress, remaining time, and overtime. Goals are configurable per workspace; defaults are 20 hours per week and 80 per month.
- **Choose your language.** English is the default. Switch to Czech in **Settings → Language** without restarting.
- **Keep credentials local.** Your personal API token goes into macOS Keychain or Windows Credential Manager. Requests go directly to ClickUp.

This is an independent personal-token client, not an official ClickUp application.

## Getting started

1. Build and launch the app using the instructions below, or download a build artifact from a successful [Actions run](https://github.com/itIsMaku/clickup-tray-time-tracker/actions/workflows/build.yml).
2. Open [ClickUp Settings → Apps](https://app.clickup.com/settings/apps) and generate or copy your **personal API token**.
3. Paste the token into the app. Select your workspace, then click a task to start tracking.

Clicking another task stops the previous timer and starts the new one. Selecting the already running task leaves its timer intact. **Stop** ends tracking. Use **New task → Create and track** to start something new.

Click the tray icon to open or close the popup. Escape or clicking outside closes the popup while the app stays in the tray. On Windows, the icon may be inside the hidden-icons menu.

**Quitting the app, disconnecting your account, or putting the computer to sleep does not stop ClickUp's server-side timer. Use Stop when you finish work.**

### Summaries and goals

**Summary**, next to Settings, displays your time for the selected workspace. Weeks start on Monday; month and week boundaries follow your computer's local time zone. The running timer is included once, and unverified data is not extrapolated during a connection failure.

Set monthly and weekly goals in hours and choose **Save goals**. Decimal values such as `20.5` or `20,5` are accepted. Goals are local preferences; the app does not change ClickUp's workspace settings.

## Build from source

Install [Rust through rustup](https://rustup.rs/). The checked-in `rust-toolchain.toml` selects the compiler and development tools automatically. The first build downloads and compiles the native UI dependencies and can take several minutes.

```sh
git clone https://github.com/itIsMaku/clickup-tray-time-tracker.git
cd clickup-tray-time-tracker
cargo run --locked
```

### macOS

Requires **macOS 15 or later** and Xcode Command Line Tools:

```sh
xcode-select --install
sh scripts/package-macos.sh
open "dist/ClickUp Tracker.app"
```

Move the app to Applications if you want to keep it there. To launch it at login, add it under **System Settings → General → Login Items**.

The script builds for the host architecture and signs the app ad hoc for local use. Distributed builds are not Developer ID signed or notarized. CI builds Apple Silicon and Intel variants separately.

### Windows

Requires **Windows 10 or later**, the **MSVC Rust toolchain**, Visual Studio 2022 Build Tools with **Desktop development with C++**, a Windows SDK, and CMake on `PATH`.

In PowerShell:

```powershell
./scripts/package-windows.ps1
```

Run `dist\ClickUp Tracker.exe`. To launch it at login, place a shortcut in the folder opened by **Win+R → `shell:startup`**.

### Development checks

```sh
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

The API, localization, and summary calculations can also be tested without the desktop dependencies:

```sh
cargo test --locked --no-default-features
```

Tests use a local HTTP server and controlled fixtures. They do not need a ClickUp account, access the credential store, or create live tasks/time entries.

## Data and synchronization

| Data | Location |
| --- | --- |
| API token | macOS Keychain / Windows Credential Manager, service `com.clickuptracker.desktop` |
| Language, workspace, goals, recent tasks | `preferences.json` in the app data directory |
| macOS app data | `~/Library/Application Support/ClickUpTracker/` |
| Windows app data | `%LOCALAPPDATA%\ClickUpTracker\` |

Preferences retain up to eight recent tasks per workspace. There is no telemetry or companion backend.

A single worker handles network and credential operations away from the UI thread. ClickUp is the source of truth: the app refreshes running state every 30 seconds, checks before changing timers, and confirms a start by reading the running task back. It does not blindly retry writes or silently queue offline mutations. When task creation succeeds but tracking cannot start, the created task remains available in recents.

See [Architecture](docs/architecture.md) for the module layout and failure-handling rules.

## Troubleshooting

| Problem | What to check |
| --- | --- |
| Invalid token | Generate a new personal token in ClickUp, disconnect, and reconnect. |
| Access denied or tracking rejected | Check access to the task and that Time Tracking is available/enabled in the workspace. |
| Missing task | Assigned open tasks are shown by default. Paste an accessible task's link or original ID to load another task. |
| HTTP 429 | Wait before refreshing; ClickUp has rate-limited the requests. |
| Unknown result after a connection failure | Refresh to verify ClickUp's current state before trying again. For task creation, check ClickUp first to avoid a duplicate. |
| App is already running | Look for the timer icon in the menu bar or Windows hidden-icons menu. |

Custom task IDs such as `PROJ-123` are not supported yet; use the original ID or a normal `app.clickup.com/t/…` link. The popup currently opens on the primary display. Linux desktop packaging, automatic updates, and offline time-entry editing are outside the current scope.

## Contributing

Bug reports and focused pull requests are welcome. Read [CONTRIBUTING.md](CONTRIBUTING.md) for setup, tests, and translation guidance. Report sensitive issues through the process in [SECURITY.md](SECURITY.md).

## License and acknowledgements

[MIT](LICENSE) © 2026 itIsMaku. Dependencies retain their own licenses.

Built with [GPUI Kit](https://gpui-kit.com/), [GPUI](https://www.gpui.rs/), [gpui-tray](https://github.com/domenkozar/gpui-tray), [reqwest](https://github.com/seanmonstar/reqwest), and [keyring](https://github.com/open-source-cooperative/keyring-rs). Integration behavior follows the [ClickUp API documentation](https://developer.clickup.com/docs/authentication).
