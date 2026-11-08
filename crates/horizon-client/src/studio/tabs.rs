use serde::{Deserialize, Serialize};

/// A tab in the Studio composition workspace.
/// Mods can register tabs to provide custom views, editors, or camera monitors.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StudioTab {
    pub id: String,
    pub title_key: String,
    pub custom_title: Option<String>,
    pub closable: bool,
    pub mod_id: Option<String>,
}

impl StudioTab {
    pub fn new(id: impl Into<String>, title_key: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title_key: title_key.into(),
            custom_title: None,
            closable: false,
            mod_id: None,
        }
    }

    pub fn with_custom_title(mut self, title: impl Into<String>) -> Self {
        self.custom_title = Some(title.into());
        self
    }

    pub fn with_mod_id(mut self, mod_id: impl Into<String>) -> Self {
        self.mod_id = Some(mod_id.into());
        self.closable = true;
        self
    }
}

/// Extensible tab system for Studio composition workspace.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StudioTabSystem {
    tabs: Vec<StudioTab>,
    active_index: usize,
}

impl Default for StudioTabSystem {
    fn default() -> Self {
        Self {
            tabs: vec![StudioTab::new(
                "composition",
                "horizon.studio.visuals.composition.active_camera",
            )],
            active_index: 0,
        }
    }
}

impl StudioTabSystem {
    pub const BAR_HEIGHT: f32 = 20.0;

    pub fn new() -> Self {
        Self::default()
    }

    pub fn tabs(&self) -> &[StudioTab] {
        &self.tabs
    }

    pub fn active_index(&self) -> usize {
        self.active_index
    }

    pub fn active_tab(&self) -> Option<&StudioTab> {
        self.tabs.get(self.active_index)
    }

    pub fn select_tab(&mut self, index: usize) -> bool {
        if index < self.tabs.len() {
            self.active_index = index;
            true
        } else {
            false
        }
    }

    pub fn select_tab_by_id(&mut self, id: &str) -> bool {
        if let Some(pos) = self.tabs.iter().position(|t| t.id == id) {
            self.active_index = pos;
            true
        } else {
            false
        }
    }

    /// Mod hook: register a custom tab provided by a content or native mod.
    pub fn register_tab(&mut self, tab: StudioTab) {
        if !self.tabs.iter().any(|t| t.id == tab.id) {
            self.tabs.push(tab);
        }
    }

    /// Unregister a tab by ID (e.g. when a mod is unloaded).
    pub fn unregister_tab(&mut self, id: &str) -> bool {
        if let Some(pos) = self.tabs.iter().position(|t| t.id == id && t.closable) {
            self.tabs.remove(pos);
            if self.active_index >= self.tabs.len() {
                self.active_index = self.tabs.len().saturating_sub(1);
            }
            true
        } else {
            false
        }
    }

    pub fn close_tab_at(&mut self, index: usize) -> bool {
        if index < self.tabs.len() && self.tabs[index].closable {
            self.tabs.remove(index);
            if self.active_index >= self.tabs.len() {
                self.active_index = self.tabs.len().saturating_sub(1);
            }
            true
        } else {
            false
        }
    }

    /// Compute display width for a tab based on title length and closability.
    pub fn tab_width(title_len: usize, closable: bool) -> f32 {
        let text_w = title_len as f32 * 6.5;
        let close_w = if closable { 14.0 } else { 0.0 };
        (text_w + 24.0 + close_w).clamp(80.0, 180.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_tab_system_has_composition_active() {
        let tabs = StudioTabSystem::default();
        assert_eq!(tabs.tabs().len(), 1);
        assert_eq!(tabs.active_index(), 0);
        assert_eq!(tabs.active_tab().unwrap().id, "composition");
        assert!(!tabs.active_tab().unwrap().closable);
    }

    #[test]
    fn mod_can_register_and_switch_tabs() {
        let mut tabs = StudioTabSystem::default();
        let mod_tab = StudioTab::new("preview", "custom.mod.preview")
            .with_custom_title("3D Preview")
            .with_mod_id("camera_tools");
        tabs.register_tab(mod_tab);
        assert_eq!(tabs.tabs().len(), 2);

        assert!(tabs.select_tab(1));
        assert_eq!(tabs.active_tab().unwrap().id, "preview");
        assert_eq!(
            tabs.active_tab().unwrap().mod_id.as_deref(),
            Some("camera_tools")
        );

        assert!(tabs.select_tab_by_id("composition"));
        assert_eq!(tabs.active_index(), 0);
    }

    #[test]
    fn closing_closable_tab_updates_active_index() {
        let mut tabs = StudioTabSystem::default();
        tabs.register_tab(StudioTab::new("extra", "extra.tab").with_mod_id("mod_a"));
        assert_eq!(tabs.tabs().len(), 2);
        tabs.select_tab(1);

        assert!(tabs.close_tab_at(1));
        assert_eq!(tabs.tabs().len(), 1);
        assert_eq!(tabs.active_index(), 0);

        // Core composition tab cannot be closed
        assert!(!tabs.close_tab_at(0));
        assert_eq!(tabs.tabs().len(), 1);
    }
}
