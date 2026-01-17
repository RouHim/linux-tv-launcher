# Process Monitoring & Focus Management

## Overview

The `FocusManager` (`src/focus_manager.rs`) is responsible for tracking game processes and detecting when they exit to
restore the launcher window. This is critical for the "Console-like" experience where the UI disappears during gameplay
and reappears instantly upon exit.

## The Challenge: "The Handover Gap"

When launching Linux games (especially via Proton, Wine, or Heroic), the process tree often behaves chaotically:

1. **Wrapper Exits:** The initial command (`xdg-open`, `steam.sh`) exits almost immediately.
2. **Gap:** There is a 1-5 second delay where *no* game process is visible while Wine initializes.
3. **Game Appears:** The actual game binary (e.g., `cyberpunk.exe`) finally starts.

If we simply monitored the initial PID, the launcher would reappear instantly during step 1, disrupting the game launch.

## Strategy: "Paranoid Start, Snappy End"

To handle this, we use a dual-phase monitoring strategy defined by three key constants:

1. **Launch Phase (First 15s):**
    * **Behavior:** We assume the process state is unstable.
    * **Grace Period (`10s`):** If the tracked process disappears, we wait 10 seconds before assuming the game has quit.
      This bridges the "Handover Gap".
    * **Trade-off:** If the user quits the game immediately (within 15s), there is a 10s delay before the launcher
      returns.

2. **Stable Phase (After 15s):**
    * **Behavior:** We assume the game is fully running and stable.
    * **Grace Period (`0.5s`):** If the process disappears, we assume the user quit intentionaly. The launcher reappears
      instantly.

## Performance Optimizations

To minimize background CPU usage while maintaining responsiveness:

* **PID Locking:** Once the game executable is identified, we "lock" onto its PID. Subsequent checks are O(1) (checking
  if PID exists) rather than scanning the entire `/proc` tree.
* **Adaptive Polling:**
    * **Searching:** Poll every `1s` (Low resource usage).
    * **Running:** Poll every `250ms` (High responsiveness for exit detection).
* **Helper Filtering:** We explicitly ignore known helper processes (`steam`, `steamwebhelper`, `gameoverlayui`,
  `pressure-vessel`) to prevent false positives where the game has quit but Steam remains open.

## Key Constants (`src/focus_manager.rs`)

| Constant                       | Value    | Description                                        |
|:-------------------------------|:---------|:---------------------------------------------------|
| `POLL_INTERVAL_FAST`           | `250ms`  | Polling rate when game is confirmed running.       |
| `POLL_INTERVAL_SLOW`           | `1000ms` | Polling rate when searching for game.              |
| `GAME_EXIT_GRACE_PERIOD_LONG`  | `10s`    | Safety buffer during Launch Phase.                 |
| `GAME_EXIT_GRACE_PERIOD_SHORT` | `0.5s`   | Instant exit buffer during Stable Phase.           |
| `STABLE_RUN_THRESHOLD`         | `15s`    | Time required to transition from Launch -> Stable. |
