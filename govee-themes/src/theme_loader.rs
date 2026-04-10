//! Theme loader that merges user TOML themes from `~/.config/govee/themes/`
//! with built-in definitions. User themes with matching names override builtins.

use std::fs;
use std::path::PathBuf;

use crate::theme_defs::builtin_themes;
use crate::themes::ThemeDef;

fn user_themes_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("govee").join("themes"))
}

fn load_user_themes() -> Vec<ThemeDef> {
    let dir = match user_themes_dir() {
        Some(d) if d.is_dir() => d,
        _ => return vec![],
    };

    let mut themes = Vec::new();
    let entries = match fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return vec![],
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "toml") {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(theme) = toml::from_str::<ThemeDef>(&content) {
                    themes.push(theme);
                }
            }
        }
    }

    themes
}

/// Load all themes: builtins merged with user-defined TOML themes.
///
/// User themes with the same name as a builtin replace it; otherwise they're appended.
pub fn load_all_themes() -> Vec<ThemeDef> {
    let mut themes = builtin_themes();
    for ut in load_user_themes() {
        if let Some(pos) = themes.iter().position(|t| t.name == ut.name) {
            themes[pos] = ut;
        } else {
            themes.push(ut);
        }
    }
    themes
}
