//! Per-player Studio recording preferences (`horizon-studio.txt`).

use std::{fs, path::Path};

pub const FILE: &str = "horizon-studio.txt";

/// Privacy-sensitive player appearance captured beside a recording, never in its `.mcpr`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StudioPreferences {
    /// Capture the sounds the current recorder already includes.
    pub record_sounds: bool,
    /// Save resolved player skins and capes in the local appearance store.
    pub record_player_skins: bool,
    /// Save real player names for rendered name tags in the local appearance store.
    pub record_player_names: bool,
    /// Write an optional mods.json compatibility manifest.
    pub include_mods_manifest: bool,
    /// Bundle the active server resource pack into recordings (may be large).
    pub include_resource_pack: bool,
}

impl Default for StudioPreferences {
    fn default() -> Self {
        Self {
            record_sounds: true,
            record_player_skins: false,
            record_player_names: false,
            include_mods_manifest: false,
            include_resource_pack: false,
        }
    }
}

impl StudioPreferences {
    pub fn load(dir: &Path) -> Self {
        fs::read_to_string(dir.join(FILE))
            .map(|text| Self::parse(&text))
            .unwrap_or_default()
    }

    pub fn parse(text: &str) -> Self {
        let mut preferences = Self::default();
        for line in text.lines() {
            let Some((key, value)) = line.split_once(':') else {
                continue;
            };
            let enabled = match value.trim() {
                "true" => true,
                "false" => false,
                _ => continue,
            };
            match key.trim() {
                "recordPlayerSkins" => preferences.record_player_skins = enabled,
                "recordPlayerNames" => preferences.record_player_names = enabled,
                "recordSounds" => preferences.record_sounds = enabled,
                "includeModsManifest" => preferences.include_mods_manifest = enabled,
                "includeResourcePack" => preferences.include_resource_pack = enabled,
                _ => {}
            }
        }
        preferences
    }

    pub fn serialize(&self) -> String {
        format!(
            "recordSounds:{}\nrecordPlayerSkins:{}\nrecordPlayerNames:{}\nincludeModsManifest:{}\nincludeResourcePack:{}\n",
            self.record_sounds,
            self.record_player_skins,
            self.record_player_names,
            self.include_mods_manifest,
            self.include_resource_pack
        )
    }

    pub fn save(&self, dir: &Path) {
        let path = dir.join(FILE);
        if *self == Self::default() && !path.exists() {
            return;
        }
        if let Err(error) = fs::write(path, self.serialize()) {
            log::warn!("Horizon: Studio settings are not saved: {error}");
        }
    }
}
