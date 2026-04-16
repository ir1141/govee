//! Terminal UI helpers: banner, colored output, brightness bars, color swatches,
//! segment blocks, and theme list formatting.

use colored::Colorize;
use std::sync::atomic::{AtomicBool, Ordering};

const DIAMOND: &str = "◆";
const ERROR_X: &str = "✖";
const FILLED: char = '█';
const EMPTY: char = '░';

/// Global flag to suppress all informational UI output (set once at startup via `--quiet`).
static QUIET: AtomicBool = AtomicBool::new(false);

/// Enable or disable quiet mode (suppresses informational output, not errors).
pub fn set_quiet(quiet: bool) {
    QUIET.store(quiet, Ordering::Relaxed);
}

/// Returns `true` if quiet mode is active.
fn is_quiet() -> bool {
    QUIET.load(Ordering::Relaxed)
}

/// Print the application banner with version.
pub fn banner() {
    if is_quiet() { return; }
    println!(
        "{} {} {}",
        "░▒▓".purple(),
        "govee".purple().bold(),
        format!("v{}", env!("CARGO_PKG_VERSION")).dimmed()
    );
    println!("{}", "LAN control · no cloud · no keys".dimmed());
    println!("{}", "─────────────────────────────────".dimmed());
}

/// Print a labeled info line with a purple diamond prefix.
pub fn info(label: &str, value: &str) {
    if is_quiet() { return; }
    println!("{} {} {}", DIAMOND.purple(), label, value);
}

/// Print the standard "Press Ctrl+C to stop" hint.
pub fn ctrlc_hint() {
    if is_quiet() { return; }
    println!("  {}", "Press Ctrl+C to stop".dimmed());
}

/// Print a pre-formatted detail line (suppressed in quiet mode).
pub fn detail(msg: &str) {
    if is_quiet() { return; }
    println!("{msg}");
}

/// Print an error message to stderr.
pub fn error(msg: &str) {
    eprintln!("{} {}", ERROR_X.red(), msg.red());
}

/// Print an error with a hint on the next line.
pub fn error_hint(msg: &str, hint: &str) {
    eprintln!("{} {}", ERROR_X.red(), msg.red());
    eprintln!("  {}", hint.dimmed());
}

/// Render a 10-block brightness bar with percentage.
pub fn brightness_bar(percent: u8) -> String {
    let filled = (percent as usize).div_ceil(10);
    let filled = filled.min(10);
    let empty = 10 - filled;
    format!(
        "{}{} {}",
        FILLED.to_string().repeat(filled).yellow(),
        EMPTY.to_string().repeat(empty).dimmed(),
        format!("{percent}%").yellow()
    )
}

/// Render a colored swatch block with hex code.
pub fn color_swatch(r: u8, g: u8, b: u8) -> String {
    format!(
        "{} {}",
        "██".truecolor(r, g, b),
        format!("#{r:02X}{g:02X}{b:02X}").dimmed()
    )
}

/// Render a colored swatch with both hex and decimal RGB.
pub fn color_swatch_full(r: u8, g: u8, b: u8) -> String {
    format!("{} {}", color_swatch(r, g, b), format!("({r}, {g}, {b})").dimmed())
}

/// Render colored block characters for each segment.
pub fn segment_blocks(colors: &[(u8, u8, u8)]) -> String {
    colors
        .iter()
        .map(|&(r, g, b)| format!("{}", "██".truecolor(r, g, b)))
        .collect::<String>()
}

/// Overwrite the current terminal line with segment colors and metadata.
pub fn status_line(segments: &[(u8, u8, u8)], meta: &str) {
    if is_quiet() { return; }
    let blocks = segment_blocks(segments);
    print!("\r{} {}", blocks, meta.dimmed());
    use std::io::Write;
    std::io::stdout().flush().ok();
}

/// End the live status line with a newline.
pub fn status_line_finish() {
    if is_quiet() { return; }
    println!();
}

/// Print "Scanning for devices..." status.
pub fn discovery_scanning() {
    if is_quiet() { return; }
    eprintln!("{} {}", DIAMOND.purple(), "Scanning for devices...".dimmed());
}

/// Print a discovered device name and IP.
pub fn discovery_found(name: &str, ip: &str) {
    if is_quiet() { return; }
    eprintln!(
        "{} Found {} {} {}",
        DIAMOND.cyan(),
        name.white().bold(),
        "at".dimmed(),
        ip.cyan()
    );
}

fn category_color(category: &str) -> colored::Color {
    match category {
        "static" => colored::Color::Magenta,
        "nature" => colored::Color::Cyan,
        "vibes" => colored::Color::Yellow,
        "functional" => colored::Color::Green,
        "seasonal" => colored::Color::Red,
        _ => colored::Color::White,
    }
}

fn category_label(category: &str) -> &str {
    match category {
        "static" => "STATIC",
        "nature" => "NATURE",
        "vibes" => "VIBES",
        "functional" => "FUNCTIONAL",
        "seasonal" => "SEASONAL",
        _ => category,
    }
}

/// Returns theme list as a string for clap help text.
/// Uses ANSI colors — `colored` auto-disables when not a TTY.
pub fn theme_list_help(themes: &[(&str, &str)]) -> String {
    let mut categories: Vec<&str> = govee_themes::BUILTIN_CATEGORIES.to_vec();
    for &(_, cat) in themes {
        if !categories.contains(&cat) {
            categories.push(cat);
        }
    }
    let mut out = format!("{}\n", "THEMES".purple().bold());
    for cat in &categories {
        let names: Vec<&str> = themes
            .iter()
            .filter(|(_, c)| c == cat)
            .map(|(n, _)| *n)
            .collect();
        if names.is_empty() {
            continue;
        }
        let color = category_color(cat);
        let border = "│".color(color);
        let label = category_label(cat).color(color).bold();
        let joined = names.join(&format!(" {} ", "·".dimmed()));
        out.push_str(&format!("{border} {label}\n"));
        out.push_str(&format!("{border} {joined}\n"));
    }
    out
}

/// Print DreamView deactivation message.
pub fn deactivating() {
    if is_quiet() { return; }
    println!("{}", "Deactivating DreamView mode...".dimmed());
}

/// Print "Stopped." message.
pub fn stopped() {
    if is_quiet() { return; }
    println!("{}", "Stopped.".dimmed());
}
