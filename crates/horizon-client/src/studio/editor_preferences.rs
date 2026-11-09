//! Persistent defaults shared by Studio replay sessions.

use std::{fs, path::Path};

use serde::{Deserialize, Serialize};

use super::VisualsSettings;

pub const FILE: &str = "horizon-studio-editor.json";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct EditorPreferences {
    pub timeline_visible: bool,
    pub visuals_open: bool,
    pub visuals_width: f32,
    pub visuals: VisualsSettings,
}

impl Default for EditorPreferences {
    fn default() -> Self {
        Self {
            timeline_visible: true,
            visuals_open: false,
            visuals_width: 195.0,
            visuals: VisualsSettings::default(),
        }
    }
}

impl EditorPreferences {
    pub fn load(path: &Path) -> Self {
        fs::read(path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) {
        let Ok(bytes) = serde_json::to_vec_pretty(self) else {
            return;
        };
        let tmp = path.with_extension("json.tmp");
        if fs::write(&tmp, bytes).is_ok()
            && let Err(error) = fs::rename(&tmp, path)
        {
            log::warn!("Studio: could not save editor preferences: {error}");
            let _ = fs::remove_file(tmp);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_defaults_round_trip_between_replays() {
        let dir = std::env::temp_dir().join(format!(
            "horizon-studio-editor-prefs-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(FILE);
        let mut settings = EditorPreferences::default();
        settings.timeline_visible = false;
        settings.visuals_open = true;
        settings.visuals.editor_canvas_box = true;
        settings.visuals.override_time = true;
        settings.visuals.time_of_day = 18000.0;
        settings.save(&path);

        let loaded = EditorPreferences::load(&path);
        assert!(!loaded.timeline_visible);
        assert!(loaded.visuals_open);
        assert!(loaded.visuals.editor_canvas_box);
        assert!(loaded.visuals.override_time);
        assert_eq!(loaded.visuals.time_of_day, 18000.0);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn old_removed_gui_toggles_are_ignored() {
        let loaded: EditorPreferences = serde_json::from_str(
            r#"{"visuals":{"gui_player_hotbar":true,"gui_player_vitals":true,"gui_player_inventory":true,"editor_canvas_box":true}}"#,
        )
        .unwrap();
        assert!(loaded.visuals.editor_canvas_box);
    }
}
