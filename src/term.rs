use std::env;
use std::fmt::Display;
use std::io::IsTerminal;

use owo_colors::OwoColorize;

/// Returns true if we should use colors on stdout (TTY and NO_COLOR not set).
pub fn color_stdout() -> bool {
    !env::var("NO_COLOR").is_ok() && std::io::stdout().is_terminal()
}

/// Returns true if we should use colors on stderr (TTY and NO_COLOR not set).
pub fn color_stderr() -> bool {
    !env::var("NO_COLOR").is_ok() && std::io::stderr().is_terminal()
}

/// Prints a warning to stderr, in yellow when stderr is a TTY.
pub fn warn(msg: impl Display) {
    let s = msg.to_string();
    if color_stderr() {
        eprintln!("{}", s.yellow());
    } else {
        eprintln!("{}", s);
    }
}
