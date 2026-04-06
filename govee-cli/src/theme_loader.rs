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
        Err(e) => {
            crate::ui::error(&format!("Could not read {}: {e}", dir.display()));
            return vec![];
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "toml") {
            match fs::read_to_string(&path) {
                Ok(content) => match toml::from_str::<ThemeDef>(&content) {
                    Ok(theme) => themes.push(theme),
                    Err(e) => crate::ui::error(&format!("Skipping {}: {e}", path.display())),
                },
                Err(e) => crate::ui::error(&format!("Could not read {}: {e}", path.display())),
            }
        }
    }

    themes
}

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
