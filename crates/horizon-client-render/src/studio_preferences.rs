//! Per-player Studio recording preferences (`horizon-studio.txt`).

use std::{collections::BTreeMap, fs, path::Path};

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
    /// Rebound Studio actions; keys live in this file rather than vanilla's options.txt.
    pub keybinds: BTreeMap<String, String>,
}

impl Default for StudioPreferences {
    fn default() -> Self {
        Self {
            record_sounds: true,
            record_player_skins: false,
            record_player_names: false,
            include_mods_manifest: false,
            include_resource_pack: false,
            keybinds: BTreeMap::new(),
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
            if key.trim().starts_with("key.studio.") {
                let value = value.trim();
                if crate::keybinds::KeyBinds::default_of(key.trim()) != "key.keyboard.unknown"
                    && (crate::keybinds::key_code(value).is_some()
                        || crate::keybinds::mouse_button(value).is_some())
                {
                    preferences
                        .keybinds
                        .insert(key.trim().to_owned(), value.to_owned());
                }
                continue;
            }
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
        let mut text = format!(
            "recordSounds:{}\nrecordPlayerSkins:{}\nrecordPlayerNames:{}\nincludeModsManifest:{}\nincludeResourcePack:{}\n",
            self.record_sounds,
            self.record_player_skins,
            self.record_player_names,
            self.include_mods_manifest,
            self.include_resource_pack
        );
        for (name, key) in &self.keybinds {
            text.push_str(name);
            text.push(':');
            text.push_str(key);
            text.push('\n');
        }
        text
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn privacy_and_archive_preferences_have_safe_defaults_and_round_trip() {
        let defaults = StudioPreferences::default();
        assert!(defaults.record_sounds);
        assert!(!defaults.record_player_skins);
        assert!(!defaults.record_player_names);
        assert!(!defaults.include_mods_manifest);
        assert!(!defaults.include_resource_pack);

        let mut enabled = defaults;
        enabled.record_player_skins = true;
        enabled.record_player_names = true;
        enabled.include_mods_manifest = true;
        enabled.include_resource_pack = true;
        enabled
            .keybinds
            .insert("key.studio.addMarker".into(), "key.mouse.4".into());
        assert_eq!(StudioPreferences::parse(&enabled.serialize()), enabled);
    }
}
