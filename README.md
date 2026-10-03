# pausa

A full-screen terminal UI Pomodoro-style stopwatch CLI tool written in Rust.

Tracks daily study time with a zero-starting counter, supports short and endless
breaks, and renders a GitHub-style heatmap calendar of your study history. All
data lives locally in SQLite — no cloud, no tracking, no accounts.

## Installation

```bash
cargo install pausa
```

Requires Rust 1.95.0 or later.

## Usage

Run from your terminal:

```bash
pausa
```

### Keybindings

| Key       | Action                                  |
|-----------|-----------------------------------------|
| `Space`   | Start / pause / resume the stopwatch    |
| `b`       | Start a short break (countdown timer)   |
| `B`       | Start an endless break (no countdown)   |
| `h`       | Toggle between stopwatch and heatmap    |
| `r`       | Reset the stopwatch (requires confirm)  |
| `y`       | Confirm the reset prompt                |
| `n`       | Dismiss the reset prompt                |
| `q`       | Quit                                    |
| `Ctrl+C`  | Quit                                    |
| `Esc`     | Quit                                    |

### Options

| Flag          | Description            |
|---------------|------------------------|
| `-h`, `--help`  | Print help and exit  |
| `-V`, `--version` | Print version and exit |

## Configuration

Pausa reads a TOML config file at `~/.config/pausa/config.toml`. If the file
does not exist, defaults are written on first run.

```toml
short_break_minutes = 10
reset_time = "00:00"
```

- **`short_break_minutes`** — Duration of the short break countdown (must be >0).
- **`reset_time`** — Daily reset time in HH:MM format (24-hour). When the clock
  passes this time, the in-flight session is saved and the stopwatch resets.

Customize with your own file:

```toml
short_break_minutes = 5
reset_time = "04:00"
```

## Data

All data lives in `~/.local/share/pausa/pausa.db` (SQLite, WAL mode).

Two tables:

- **`sessions`** — Individual study and break sessions with start/end times,
  duration, and type.
- **`daily_sessions`** — Rolled-up daily totals aggregated from sessions.

## License

MIT
