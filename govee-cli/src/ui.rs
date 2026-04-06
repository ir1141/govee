use colored::Colorize;

const DIAMOND: &str = "◆";
const ERROR_X: &str = "✖";
const FILLED: char = '█';
const EMPTY: char = '░';

// ── Banner ─────────────────────────────────────────────────────────────────

pub fn banner() {
    println!(
        "{} {} {}",
        "░▒▓".purple(),
        "govee".purple().bold(),
        format!("v{}", env!("CARGO_PKG_VERSION")).dimmed()
    );
    println!("{}", "LAN control · no cloud · no keys".dimmed());
    println!("{}", "─────────────────────────────────".dimmed());
}

// ── Info / status lines ────────────────────────────────────────────────────

pub fn info(label: &str, value: &str) {
    println!("{} {} {}", DIAMOND.purple(), label, value);
}

// ── Errors ─────────────────────────────────────────────────────────────────

pub fn error(msg: &str) {
    eprintln!("{} {}", ERROR_X.red(), msg.red());
}

pub fn error_hint(msg: &str, hint: &str) {
    eprintln!("{} {}", ERROR_X.red(), msg.red());
    eprintln!("  {}", hint.dimmed());
}

// ── Brightness bar ─────────────────────────────────────────────────────────

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

// ── Color swatch ───────────────────────────────────────────────────────────

pub fn color_swatch(r: u8, g: u8, b: u8) -> String {
    format!(
        "{} {}",
        "██".truecolor(r, g, b),
        format!("#{r:02X}{g:02X}{b:02X}").dimmed()
    )
}

pub fn color_swatch_full(r: u8, g: u8, b: u8) -> String {
    format!(
        "{} {} {}",
        "██".truecolor(r, g, b),
        format!("#{r:02X}{g:02X}{b:02X}").dimmed(),
        format!("({r}, {g}, {b})").dimmed()
    )
}

// ── Segment blocks ─────────────────────────────────────────────────────────

pub fn segment_blocks(colors: &[(u8, u8, u8)]) -> String {
    colors
        .iter()
        .map(|&(r, g, b)| format!("{}", "██".truecolor(r, g, b)))
        .collect::<String>()
}

// ── Live status line (continuous modes) ────────────────────────────────────

pub fn status_line(segments: &[(u8, u8, u8)], meta: &str) {
    let blocks = segment_blocks(segments);
    print!("\r{} {}", blocks, meta.dimmed());
    use std::io::Write;
    std::io::stdout().flush().ok();
}

pub fn status_line_finish() {
    println!();
}

// ── Discovery ──────────────────────────────────────────────────────────────

pub fn discovery_scanning() {
    eprintln!("{} {}", DIAMOND.purple(), "Scanning for devices...".dimmed());
}

pub fn discovery_found(name: &str, ip: &str) {
    eprintln!(
        "{} Found {} {} {}",
        DIAMOND.cyan(),
        name.white().bold(),
        "at".dimmed(),
        ip.cyan()
    );
}

// ── Theme list ─────────────────────────────────────────────────────────────

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
    let known = ["static", "nature", "vibes", "functional", "seasonal"];
    let mut categories: Vec<&str> = known.to_vec();
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

// ── Shutdown messages ──────────────────────────────────────────────────────

pub fn deactivating() {
    println!("{}", "Deactivating DreamView mode...".dimmed());
}

pub fn stopped() {
    println!("{}", "Stopped.".dimmed());
}
