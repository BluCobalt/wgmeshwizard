//! User preferences persisted as `settings.toml`.
//!
//! Project/mesh data lives in the project CSV or `.wgm` archive. This module
//! only owns application-level preferences such as theme, UI scale, language
//! and the auto-arrange layout. Window geometry and egui widget memory are
//! persisted separately in `.wgmw.ron` next to `settings.toml`.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const SETTINGS_VERSION: u32 = 1;

/// Optional path override, useful for portable installs and testing.
pub const SETTINGS_ENV: &str = "WGMESHWIZARD_SETTINGS";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub version: u32,
    pub appearance: AppearanceSettings,
    pub language: LanguageSettings,
    pub mesh_view: MeshViewSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: SETTINGS_VERSION,
            appearance: AppearanceSettings::default(),
            language: LanguageSettings::default(),
            mesh_view: MeshViewSettings::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppearanceSettings {
    /// `system`, `dark`, or `light`.
    pub theme: String,
    /// egui zoom factor. Clamped to the UI's supported range on load.
    pub ui_scale: f32,
}

impl Default for AppearanceSettings {
    fn default() -> Self {
        Self {
            theme: "system".to_owned(),
            ui_scale: 1.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LanguageSettings {
    /// `system` or a locale code such as `en` or `pt-BR`.
    pub locale: String,
}

impl Default for LanguageSettings {
    fn default() -> Self {
        Self {
            locale: "system".to_owned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MeshViewSettings {
    /// `circle`, `square`, `cards`, or `rows`.
    pub arrange_layout: String,
}

impl Default for MeshViewSettings {
    fn default() -> Self {
        Self {
            arrange_layout: "circle".to_owned(),
        }
    }
}

impl Settings {
    /// Normalize values read from disk so a hand-edited file cannot produce an
    /// unusable UI.
    pub fn sanitized(mut self) -> Self {
        self.version = SETTINGS_VERSION;
        self.appearance.theme = match self.appearance.theme.trim().to_ascii_lowercase().as_str() {
            "dark" => "dark",
            "light" => "light",
            _ => "system",
        }
        .to_owned();

        if !self.appearance.ui_scale.is_finite() {
            self.appearance.ui_scale = 1.0;
        }
        self.appearance.ui_scale = self.appearance.ui_scale.clamp(0.75, 2.0);

        self.language.locale = self.language.locale.trim().to_owned();
        if self.language.locale.is_empty() {
            self.language.locale = "system".to_owned();
        }

        self.mesh_view.arrange_layout = match self
            .mesh_view
            .arrange_layout
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "square" => "square",
            "cards" | "grid" => "cards",
            "rows" => "rows",
            _ => "circle",
        }
        .to_owned();

        self
    }
}

/// Config-file location:
///
/// * `WGMESHWIZARD_SETTINGS`, if set
/// * Linux: `$XDG_CONFIG_HOME/wgmeshwizard/settings.toml` or
///   `~/.config/wgmeshwizard/settings.toml`
/// * macOS: `~/Library/Application Support/wgmeshwizard/settings.toml`
/// * Windows: `%APPDATA%\wgmeshwizard\settings.toml`
pub fn settings_path() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os(SETTINGS_ENV) {
        return Some(PathBuf::from(path));
    }

    if cfg!(target_os = "windows") {
        return std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .map(|base| base.join("wgmeshwizard").join("settings.toml"));
    }

    if cfg!(target_os = "macos") {
        return home_dir().map(|home| {
            home.join("Library")
                .join("Application Support")
                .join("wgmeshwizard")
                .join("settings.toml")
        });
    }

    if let Some(config_home) = std::env::var_os("XDG_CONFIG_HOME") {
        let config_home = PathBuf::from(config_home);
        if config_home.is_absolute() {
            return Some(config_home.join("wgmeshwizard").join("settings.toml"));
        }
    }

    home_dir().map(|home| {
        home.join(".config")
            .join("wgmeshwizard")
            .join("settings.toml")
    })
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

/// Location of eframe's persisted UI state, kept next to `settings.toml`.
pub fn persistence_path() -> Option<PathBuf> {
    settings_path().map(|path| path.with_file_name(".wgmw.ron"))
}

/// Load settings. A missing file is not an error; malformed files are.
pub fn load(path: &Path) -> Result<Option<Settings>, String> {
    if !path.exists() {
        return Ok(None);
    }

    let text = fs::read_to_string(path)
        .map_err(|error| format!("could not read {}: {error}", path.display()))?;
    let settings: Settings = toml::from_str(&text)
        .map_err(|error| format!("could not parse {}: {error}", path.display()))?;
    Ok(Some(settings.sanitized()))
}

/// Save settings atomically with a human-friendly header.
pub fn save(path: &Path, settings: &Settings) -> Result<(), String> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
    }

    let body = toml::to_string_pretty(settings)
        .map_err(|error| format!("could not encode settings: {error}"))?;
    let content = format!(
        "# WireGuard Mesh Wizard settings\n\
         # This file is safe to edit by hand. It is rewritten when a setting changes.\n\
         # Window geometry and egui state are stored in .wgmw.ron next to this file.\n\n\
         {body}"
    );

    let mut temporary_name = path
        .file_name()
        .map(|name| name.to_os_string())
        .unwrap_or_default();
    temporary_name.push(".tmp");
    let temporary = path.with_file_name(temporary_name);

    fs::write(&temporary, content)
        .map_err(|error| format!("could not write {}: {error}", temporary.display()))?;
    fs::rename(&temporary, path)
        .map_err(|error| format!("could not replace {}: {error}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "wgmeshwizard-settings-{name}-{}/settings.toml",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(path.parent().unwrap());
        path
    }

    #[test]
    fn round_trip_settings() {
        let path = temp_path("roundtrip");
        let settings = Settings {
            version: SETTINGS_VERSION,
            appearance: AppearanceSettings {
                theme: "light".to_owned(),
                ui_scale: 1.25,
            },
            language: LanguageSettings {
                locale: "de".to_owned(),
            },
            mesh_view: MeshViewSettings {
                arrange_layout: "cards".to_owned(),
            },
        };

        save(&path, &settings).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains("[appearance]"));
        assert!(text.contains("theme = \"light\""));
        assert_eq!(load(&path).unwrap(), Some(settings));
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn missing_file_is_not_an_error() {
        let path = temp_path("missing");
        assert_eq!(load(&path).unwrap(), None);
    }

    #[test]
    fn invalid_values_are_sanitized() {
        let path = temp_path("invalid");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            &path,
            "version = 99\n\
             [appearance]\n\
             theme = \"neon\"\n\
             ui_scale = 99.0\n\
             [language]\n\
             locale = \"  \"\n\
             [mesh_view]\n\
             arrange_layout = \"hexagon\"\n",
        )
        .unwrap();

        let settings = load(&path).unwrap().unwrap();
        assert_eq!(settings.version, SETTINGS_VERSION);
        assert_eq!(settings.appearance.theme, "system");
        assert_eq!(settings.appearance.ui_scale, 2.0);
        assert_eq!(settings.language.locale, "system");
        assert_eq!(settings.mesh_view.arrange_layout, "circle");
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn malformed_toml_is_an_error() {
        let path = temp_path("malformed");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "[appearance\nui_scale = 1.0").unwrap();
        assert!(load(&path).is_err());
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
}
