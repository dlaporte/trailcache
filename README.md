```
 ____                  _   _                 _      ___   ___
/ ___|  ___ ___  _   _| |_| |__   ___   ___ | | __ ( _ ) ( _ )
\___ \ / __/ _ \| | | | __| '_ \ / _ \ / _ \| |/ / / _ \ / _ \
 ___) | (_| (_) | |_| | |_| |_) | (_) | (_) |   < | (_) | (_) |
|____/ \___\___/ \__,_|\__|_.__/ \___/ \___/|_|\_\ \___/ \___/

        <<<  TERMINAL INTERFACE FOR SCOUTBOOK DATA  >>>
```

---

**YOU'RE A SCOUTMASTER. YOU DON'T HAVE TIME TO WAIT.**

Between campouts, merit badge classes, and trying to remember which scout needs their Totin' Chip signed off, the *last* thing you need is a slow web interface. Scoutbook88 is a **blazing-fast terminal application** that puts your troop data at your fingertips in milliseconds, not minutes.

---

## FEATURES

```
[F1] SCOUTS     Your complete youth roster. Names, ranks, patrols,
                advancement status. Sorted any way you want it.

[F2] ADULTS     Leaders, committee members, parents. All the grown-ups
                who make it happen.

[F3] EVENTS     Campouts, meetings, service projects. See who's RSVP'd.
                Plan your next adventure.

[F4] DASHBOARD  The big picture. Rank advancement stats. Awards ready
                to present. Everything at a glance.
```

## SPEED IS THE NAME OF THE GAME

- **Instant startup** - cached data loads while you blink
- **Background refresh** - data updates happen behind the scenes
- **Keyboard-driven** - your hands never leave home row
- **Zero bloat** - no browser, no JavaScript, no waiting

```
  ╔══════════════════════════════════════════════════════════════╗
  ║  LOAD TIME COMPARISON                                        ║
  ╠══════════════════════════════════════════════════════════════╣
  ║  Scoutbook Website    ████████████████████████████  12+ sec  ║
  ║  Scoutbook88          █                              <1 sec  ║
  ╚══════════════════════════════════════════════════════════════╝
```

## INSTALLATION

### The Quick Way (Recommended)

**macOS / Linux:**
```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/dlaporte/scoutbook88/releases/latest/download/scoutbook88-installer.sh | sh
```

**Windows (PowerShell):**
```powershell
irm https://github.com/dlaporte/scoutbook88/releases/latest/download/scoutbook88-installer.ps1 | iex
```

### From Source

```bash
git clone https://github.com/dlaporte/scoutbook88.git
cd scoutbook88
cargo build --release
./target/release/scoutbook88
```

## KEYBOARD CONTROLS

```
  NAVIGATION                    ACTIONS
  ──────────────────────────    ──────────────────────────
  Tab / Shift+Tab  Switch tabs  /         Search
  j / Down         Move down    Esc       Cancel / Back
  k / Up           Move up      Enter     Select / Confirm
  h / Left         Previous     r         Refresh data
  l / Right        Next         ?         Help
  PgUp / PgDn      Fast scroll  q         Quit
```

## REQUIREMENTS

- Your Scouting.org credentials (same login as Scoutbook)
- A terminal that supports Unicode (most do)
- The burning desire to stop waiting for web pages to load

## FOR THE BUSY SCOUTMASTER

You've got 15 minutes before the troop meeting. A parent asks which scouts are close to Star rank. Another wants to know if their kid RSVP'd for the campout. The Advancement Chair needs to know what awards to order.

**With Scoutbook88:**
1. Open terminal
2. Get answers in seconds
3. Look like a hero

**With the website:**
1. Open browser
2. Wait for login page
3. Log in
4. Wait for dashboard
5. Click through menus
6. Wait for each page
7. Meeting has started

---

```
  ╭────────────────────────────────────────────────────────────╮
  │                                                            │
  │    "I used to dread checking Scoutbook before meetings.    │
  │     Now I just pop open the terminal and I'm done."        │
  │                                                            │
  │                              - Every Scoutmaster, probably │
  │                                                            │
  ╰────────────────────────────────────────────────────────────╯
```

---

## TECHNICAL DETAILS

Built with Rust for maximum performance:
- **ratatui** - Terminal UI framework
- **tokio** - Async runtime for parallel data fetching
- **reqwest** - HTTP client
- **keyring** - Secure credential storage (OS keychain)

Data is cached locally and refreshed in the background, so you always see something instantly while fresh data loads.

## LICENSE

MIT

---

```
        ╔═══════════════════════════════════════════╗
        ║                                           ║
        ║   BE PREPARED... TO BE FAST.              ║
        ║                                           ║
        ╚═══════════════════════════════════════════╝
```
