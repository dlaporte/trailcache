```
 _____         _ _  ____           _
|_   _| __ __ _(_) |/ ___|__ _  ___| |__   ___
  | || '__/ _` | | | |   / _` |/ __| '_ \ / _ \
  | || | | (_| | | | |__| (_| | (__| | | |  __/
  |_||_|  \__,_|_|_|\____\__,_|\___|_| |_|\___|

       <<<  YOUR SCOUTBOOK DATA, TRAIL-READY  >>>
```

---

**BECAUSE CELL SERVICE DOESN'T REACH THE CAMPSITE.**

You're deep in the backcountry. A scout asks if they've completed all their Second Class requirements. Another needs to know which merit badges they're registered for. The sun is setting, the campfire is calling, and your phone shows exactly zero bars.

Trailcache keeps your troop's Scoutbook data cached locally, ready whenever you need it—whether you're in a basement meeting room or on a mountaintop.

---

## WHY TRAILCACHE?

### Take Your Data Into the Field

Your troop data goes where you go. Cache it before the campout, access it anywhere. No signal? No problem. Check advancement status by lantern light.

### Blazing Fast

Trailcache loads your data in under a second. Keyboard-driven navigation means you get answers as fast as you can type. Your terminal is ready before you finish sitting down.

---

## FEATURES

```
[1] SCOUTS      Your complete youth roster. Names, ranks, patrols,
                advancement status. Sorted any way you want it.

[2] RANKS       Track rank advancement across the troop. See who's
                close to their next rank and what they still need.

[3] BADGES      Merit badge progress at a glance. Who's working on
                what, and how far along they are.

[4] EVENTS      Campouts, meetings, service projects. See who's RSVP'd.
                Plan your next adventure.

[5] ADULTS      Leaders, committee members, parents. All the grown-ups
                who make it happen.

[6] UNIT        The big picture. Troop stats, awards ready to present,
                everything at a glance.
```

## ON THE TRAIL

Picture this: You're at summer camp. It's merit badge midway. Scouts are asking what they're signed up for, parents are texting questions you can't answer without data, and the camp WiFi is... well, it's camp WiFi.

**With Trailcache:**
- Synced your data before leaving home
- Pull up any scout's info instantly
- Answer questions, be the hero, get back to the s'mores

---

## INSTALLATION

### The Quick Way (Recommended)

**macOS / Linux:**
```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/dlaporte/trailcache/releases/latest/download/trailcache-installer.sh | sh
```

**Windows (PowerShell):**
```powershell
irm https://github.com/dlaporte/trailcache/releases/latest/download/trailcache-installer.ps1 | iex
```

### From Source

```bash
git clone https://github.com/dlaporte/trailcache.git
cd trailcache
cargo build --release
./target/release/trailcache
```

---

## KEYBOARD CONTROLS

```
  NAVIGATION                    ACTIONS
  ──────────────────────────    ──────────────────────────
  1-6              Jump to tab  /         Search
  Tab / Shift+Tab  Next/prev    Esc       Cancel / Back
  j / Down         Move down    Enter     Select / Confirm
  k / Up           Move up      r         Refresh data
  h / Left         Previous     ?         Help
  l / Right        Next         q         Quit
  PgUp / PgDn      Fast scroll
```

---

## REQUIREMENTS

- Your Scouting.org credentials (same login as Scoutbook)
- A terminal that supports Unicode (most do)
- An internet connection (just once, to sync your data)

---

## TECHNICAL DETAILS

Built with Rust for maximum performance:
- **ratatui** - Terminal UI framework
- **tokio** - Async runtime for parallel data fetching
- **reqwest** - HTTP client
- **keyring** - Secure credential storage (OS keychain)

Data is cached locally and refreshed in the background when connected, so you always have something to work with—online or off.

---

## LICENSE

MIT

---

```
      ╔═══════════════════════════════════════════════════╗
      ║                                                   ║
      ║   BE PREPARED... EVEN WITHOUT A SIGNAL.           ║
      ║                                                   ║
      ╚═══════════════════════════════════════════════════╝
```
