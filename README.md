<p align="center">
  <img src="logo.png" alt="Trailcache" width="200">
</p>

<h1 align="center">Trailcache</h1>

<p align="center"><strong>BE PREPARED... EVEN WITHOUT A SIGNAL.</strong></p>

<p align="center">
  <a href="https://crates.io/crates/trailcache-tui"><img src="https://img.shields.io/crates/v/trailcache-tui.svg" alt="crates.io"></a>
  <a href="https://github.com/dlaporte/trailcache/releases/latest"><img src="https://img.shields.io/github/v/release/dlaporte/trailcache" alt="GitHub Release"></a>
  <a href="https://github.com/dlaporte/trailcache/blob/master/LICENSE"><img src="https://img.shields.io/crates/l/trailcache-tui" alt="License: MIT"></a>
</p>

---

**BECAUSE CELL SERVICE DOESN'T REACH THE CAMPSITE.**

You're deep in the backcountry. A scout asks if they've completed all their Second Class requirements. Another needs to know which merit badges they're registered for. The sun is setting, the campfire is calling, and your phone shows exactly zero bars.

Trailcache keeps your troop's Scoutbook data cached locally, ready whenever you need it — whether you're in a basement meeting room or on a mountaintop.

---

## Why Trailcache?

### Take Your Data Into the Field

Your troop data goes where you go. Cache it before the campout, access it anywhere. No signal? No problem. Check advancement status by lantern light.

### Blazing Fast

Trailcache loads your data in under a second. Navigate instantly across scouts, ranks, badges, events, and leaders. Your answers are ready before you finish sitting down.

### Cross-Platform

Runs everywhere your troop goes:
- **Desktop** — macOS (signed .dmg) and Windows (.msi / .exe)
- **Terminal** — macOS, Windows, Linux (keyboard-driven TUI)
- **Mobile** — iOS and Android (coming soon)

---

## Features

### Scouts
Your complete youth roster — names, ranks, patrols, leadership positions, and advancement status. Drill into any scout to see their full profile including rank progress, merit badges, and awards. Sort and search across the entire troop.

### Ranks
Track rank advancement across every scout in the troop. See at a glance who's close to their next rank, what requirements they've completed, and who's ready for a Board of Review. Pivot tables show the full picture.

### Merit Badges
Merit badge progress for every scout, all in one place. See who's working on what, how many requirements are complete, and which badges have been awarded. Track Eagle-required badges and overall progress toward Eagle.

### Events
Campouts, meetings, service projects, and more. See RSVP status for every event — who's going, who's not, and who hasn't responded. Adult and scout counts at a glance.

### Adults
Leaders, committee members, and parents. View positions, training status (YPT), membership expiration, and contact information. Quickly identify who needs to renew training.

### Unit
The big picture — troop-level statistics, awards ready to present, and a summary of your unit's overall advancement status.

---

## On the Trail

Picture this: You're at summer camp. It's merit badge midway. Scouts are asking what they're signed up for, parents are texting questions you can't answer without data, and the camp WiFi is... well, it's camp WiFi.

**With Trailcache:**
- Synced your data before leaving home
- Pull up any scout's info instantly
- Answer questions, be the hero, get back to the s'mores

---

## Installation

Download the latest release for your platform from the [GitHub Releases](https://github.com/dlaporte/trailcache/releases) page:

| Platform | Download |
|----------|----------|
| macOS (Desktop) | `.dmg` — signed and notarized |
| Windows (Desktop) | `.msi` or `.exe` installer |
| macOS / Linux (Terminal) | Pre-built binaries or install script |
| Windows (Terminal) | Pre-built binary or PowerShell install script |

**Or install the terminal app from crates.io:**
```bash
cargo install trailcache-tui
```

---

## Requirements

- Your Scouting.org credentials (same login as Scoutbook)
- An internet connection (just once, to sync your data)

---

## Technical Details

Built with Rust for maximum performance and safety. Three crates, one codebase:

- **trailcache-core** — Shared library handling Scoutbook API communication, data caching, encryption, and domain models
- **trailcache-tui** — Terminal interface built with [ratatui](https://ratatui.rs), featuring keyboard-driven navigation and vim-style keybindings
- **trailcache-gui** — Desktop application built with [Tauri](https://tauri.app) and [Svelte](https://svelte.dev), providing a modern native UI on macOS and Windows

Key dependencies:
- **tokio** — Async runtime for parallel data fetching
- **reqwest** — HTTP client with rustls for cross-platform TLS
- **keyring** — Secure credential storage via the OS keychain
- **chacha20poly1305 + argon2** — Encryption at rest for cached data

Data is cached locally and refreshed in the background when connected, so you always have something to work with — online or off.

---

## Security

Trailcache is designed with security as a priority:

- **Direct Authentication** — Your credentials are sent directly to Scoutbook's servers and are never transmitted to any third-party service.
- **Public APIs Only** — All data is retrieved using the same publicly available APIs that the Scoutbook website uses.
- **Encrypted in Transit** — All communication with Scoutbook is encrypted over HTTPS.
- **Encrypted at Rest** — Cached data is encrypted on your device using modern, standards-based encryption (ChaCha20-Poly1305 with Argon2 key derivation).
- **Open Source** — The complete source code is available for inspection. No hidden functionality, no telemetry, no surprises.

---

## License

MIT
