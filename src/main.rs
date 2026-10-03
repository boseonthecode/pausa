// Items used only in tests or future phases are expected to be dead code for now.
#![expect(dead_code)]

mod app;
mod calendar;
mod config;
mod db;
mod tui;

use std::io::Write;

const HELP: &str = concat!(
    "pausa — A full-screen terminal UI Pomodoro-style stopwatch\n",
    "\n",
    "USAGE:\n",
    "    pausa [FLAGS]\n",
    "\n",
    "FLAGS:\n",
    "    -h, --help       Print this help message and exit\n",
    "    -V, --version    Print version information and exit\n",
    "\n",
    "KEYBINDINGS:\n",
    "    Space    Start / pause / resume the stopwatch\n",
    "    b        Start a short break (countdown timer)\n",
    "    B        Start an endless break (no countdown)\n",
    "    h        Toggle between stopwatch and heatmap calendar\n",
    "    r        Reset the stopwatch (requires confirmation)\n",
    "    y        Confirm the reset prompt\n",
    "    n        Dismiss the reset prompt\n",
    "    q        Quit the application\n",
    "    Ctrl+C   Quit the application\n",
    "    Esc      Quit the application\n",
);

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    // Handle CLI flags before anything else.
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        match args[1].as_str() {
            "-h" | "--help" => {
                print!("{HELP}");
                std::process::exit(0);
            }
            "-V" | "--version" => {
                println!("pausa {VERSION}");
                std::process::exit(0);
            }
            _ => {
                eprintln!("pausa: unknown flag '{}'. Use --help for usage.", args[1]);
                std::process::exit(1);
            }
        }
    }

    // Ensure the terminal is restored if the application panics.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = tui::teardown_terminal();
        default_hook(info);
    }));
    let cfg = config::Config::load().expect("failed to load config");
    let conn = db::init(&db::db_path()).expect("failed to init database");

    // Show splash screen briefly
    let splash = tui::splash::render_splash();
    print!("{splash}");
    std::io::stdout().flush().ok();
    std::thread::sleep(std::time::Duration::from_secs(2));

    // Launch TUI
    let mut terminal = tui::setup_terminal().expect("failed to setup terminal");
    let mut app = app::App::new(conn, cfg);
    let result = app.run(&mut terminal);
    tui::teardown_terminal().expect("failed to teardown terminal");

    if let Err(e) = result {
        eprintln!("Error: {e:?}");
        std::process::exit(1);
    }
}
