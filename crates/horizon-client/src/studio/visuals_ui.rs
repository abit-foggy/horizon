//! Visuals & Overrides Sidebar UI
//!
//! Right-docked inspector panel providing toggles for GUI elements, world rendering,
//! cinematic overrides (Fog, FOV, Time of Day, Camera Shake, Roll), and framing guides
//! (Rule of Thirds, Center Crosshair, Aspect Ratio framing).
//!
//! Vertically constrained between the macOS top bar and bottom timeline, with mouse-wheel
//! scrolling, scrollbar thumb, and multi-option presets.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use horizon_client_render::{
    graphics_style as style,
    hud::{HudAtlas, HudBatch},
    text::Text,
    ui,
};

/// Framing aspect ratio modes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AspectRatio {
    None,
    Ratio16_9,
    Ratio4_3,
    Ratio1_1,
    Ratio21_9,
}

impl AspectRatio {
    pub fn label_key(self) -> &'static str {
        match self {
            Self::None => "horizon.studio.visuals.aspect.none",
            Self::Ratio16_9 => "horizon.studio.visuals.aspect.16_9",
            Self::Ratio4_3 => "horizon.studio.visuals.aspect.4_3",
            Self::Ratio1_1 => "horizon.studio.visuals.aspect.1_1",
            Self::Ratio21_9 => "horizon.studio.visuals.aspect.21_9",
        }
    }

    pub fn target_ratio(self) -> Option<f32> {
        match self {
            Self::None => None,
            Self::Ratio16_9 => Some(16.0 / 9.0),
            Self::Ratio4_3 => Some(4.0 / 3.0),
            Self::Ratio1_1 => Some(1.0),
            Self::Ratio21_9 => Some(21.0 / 9.0),
        }
    }

    pub fn cycle(self) -> Self {
        match self {
            Self::None => Self::Ratio16_9,
            Self::Ratio16_9 => Self::Ratio4_3,
            Self::Ratio4_3 => Self::Ratio1_1,
            Self::Ratio1_1 => Self::Ratio21_9,
            Self::Ratio21_9 => Self::None,
        }
    }
}

/// Settings and toggles stored in the visuals sidebar.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct VisualsSettings {
    // World section
    pub world_blocks: bool,
    pub world_entities: bool,
    pub world_players: bool,
    pub world_particles: bool,
    pub world_sky: bool,
    pub world_nametags: bool,

    // Overrides section
    pub override_fog: bool,
    pub fog_distance: f32, // 4..64 chunks
    pub override_fov: bool,
    pub fov_value: f32, // 30..130
    pub override_time: bool,
    pub time_of_day: f32, // 0..24000
    pub override_shake: bool,
    pub shake_intensity: f32, // 0.0..1.0
    pub override_roll: bool,
    pub camera_roll: f32, // -180.0..180.0

    // Other / Guides section
    pub guide_rule_of_thirds: bool,
    pub guide_center: bool,
    pub camera_path_3d: bool,
    pub aspect_ratio: AspectRatio,

    // Editor section
    pub editor_canvas_box: bool,
}

impl Default for VisualsSettings {
    fn default() -> Self {
        Self {
            world_blocks: true,
            world_entities: true,
            world_players: true,
            world_particles: true,
            world_sky: true,
            world_nametags: true,

            override_fog: false,
            fog_distance: 12.0,
            override_fov: false,
            fov_value: 70.0,
            override_time: false,
            time_of_day: 6000.0, // Noon
            override_shake: false,
            shake_intensity: 0.0,
            override_roll: false,
            camera_roll: 0.0,

            guide_rule_of_thirds: false,
            guide_center: false,
            camera_path_3d: true,
            aspect_ratio: AspectRatio::None,

            editor_canvas_box: false,
        }
    }
}

impl VisualsSettings {
    pub fn cycle_fog_distance(&mut self) {
        let presets = [4.0, 8.0, 12.0, 16.0, 24.0, 32.0];
        let idx = presets
            .iter()
            .position(|&d| (d - self.fog_distance).abs() < 1.0)
            .unwrap_or(2);
        self.fog_distance = presets[(idx + 1) % presets.len()];
    }

    pub fn cycle_fov_preset(&mut self) {
        let presets = [30.0, 60.0, 70.0, 90.0, 110.0];
        let idx = presets
            .iter()
            .position(|&f| (f - self.fov_value).abs() < 2.0)
            .unwrap_or(2);
        self.fov_value = presets[(idx + 1) % presets.len()];
    }

    pub fn cycle_time_preset(&mut self) {
        if self.time_of_day < 3000.0 {
            self.time_of_day = 6000.0;
        } else if self.time_of_day < 9000.0 {
            self.time_of_day = 12000.0;
        } else if self.time_of_day < 15000.0 {
            self.time_of_day = 18000.0;
        } else {
            self.time_of_day = 1000.0;
        }
    }

    pub fn time_preset_name(&self) -> &'static str {
        if self.time_of_day < 3000.0 {
            "Day"
        } else if self.time_of_day < 9000.0 {
            "Noon"
        } else if self.time_of_day < 15000.0 {
            "Sunset"
        } else {
            "Night"
        }
    }

    pub fn time_preset_key(&self) -> &'static str {
        match self.time_preset_name() {
            "Day" => "horizon.studio.visuals.value.day",
            "Noon" => "horizon.studio.visuals.value.noon",
            "Sunset" => "horizon.studio.visuals.value.sunset",
            _ => "horizon.studio.visuals.value.night",
        }
    }

    pub fn cycle_shake(&mut self) {
        if self.shake_intensity < 0.1 {
            self.shake_intensity = 0.2;
        } else if self.shake_intensity < 0.35 {
            self.shake_intensity = 0.5;
        } else if self.shake_intensity < 0.75 {
            self.shake_intensity = 1.0;
        } else {
            self.shake_intensity = 0.0;
        }
    }

    pub fn cycle_roll(&mut self) {
        let presets = [-90.0, 0.0, 90.0, 180.0];
        let current = if (self.camera_roll.abs() - 180.0).abs() < 1.0 {
            180.0
        } else {
            self.camera_roll
        };
        let idx = presets
            .iter()
            .position(|&degrees| (degrees - current).abs() < 1.0)
            .unwrap_or(1);
        self.camera_roll = presets[(idx + 1) % presets.len()];
    }

    pub fn shake_name(&self) -> &'static str {
        if self.shake_intensity < 0.1 {
            "Off"
        } else if self.shake_intensity < 0.35 {
            "Subtle"
        } else if self.shake_intensity < 0.75 {
            "Med"
        } else {
            "Heavy"
        }
    }

    pub fn shake_key(&self) -> &'static str {
        match self.shake_name() {
            "Off" => "horizon.studio.visuals.value.off",
            "Subtle" => "horizon.studio.visuals.value.subtle",
            "Med" => "horizon.studio.visuals.value.medium",
            _ => "horizon.studio.visuals.value.heavy",
        }
    }
}

/// Scope for visual settings: global or per-scene override.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsScope {
    Global,
    PerScene,
}

impl SettingsScope {
    pub fn label_key(self) -> &'static str {
        match self {
            Self::Global => "horizon.studio.visuals.scope.global",
            Self::PerScene => "horizon.studio.visuals.scope.per_scene",
        }
    }

    pub fn toggle(self) -> Self {
        match self {
            Self::Global => Self::PerScene,
            Self::PerScene => Self::Global,
        }
    }
}

/// Visuals sidebar UI controller and renderer.
#[derive(Clone, Debug)]
pub struct VisualsUi {
    pub open: bool,
    pub width: f32,
    pub resizing_width: bool,
    pub settings: VisualsSettings,
    pub scroll_y: f32,
    pub scope: SettingsScope,
    pub active_scene: usize,
    pub global_settings: VisualsSettings,
    pub scene_settings: HashMap<usize, VisualsSettings>,
    pub tabs: crate::studio::tabs::StudioTabSystem,
}

impl Default for VisualsUi {
    fn default() -> Self {
        let default_settings = VisualsSettings::default();
        Self {
            open: false,
            width: 195.0,
            resizing_width: false,
            settings: default_settings.clone(),
            scroll_y: 0.0,
            scope: SettingsScope::Global,
            active_scene: 0,
            global_settings: default_settings,
            scene_settings: HashMap::new(),
            tabs: crate::studio::tabs::StudioTabSystem::default(),
        }
    }
}

impl VisualsUi {
    pub fn handle_resize_drag(&mut self, cursor_x: f32, screen_width: f32) {
        if self.resizing_width {
            let max_w = (screen_width - 180.0).clamp(160.0, 480.0);
            let min_w = 160.0_f32.min(max_w);
            let new_width = (screen_width - cursor_x).clamp(min_w, max_w);
            self.width = new_width.round();
        }
    }
    pub fn new() -> Self {
        Self::default()
    }

    /// Start each replay from the last-used visual settings while discarding
    /// scene overrides belonging to the previously opened recording.
    pub fn begin_replay_session(&mut self) {
        let defaults = self.settings.clone();
        self.global_settings = defaults.clone();
        self.settings = defaults;
        self.scene_settings.clear();
        self.active_scene = 0;
        self.scope = SettingsScope::Global;
    }

    /// Set scope (Global or Per-Scene). Automatically persists and switches settings.
    pub fn set_scope(&mut self, scope: SettingsScope) {
        if self.scope == scope {
            return;
        }
        self.sync_active_settings();
        self.scope = scope;
        if self.scope == SettingsScope::Global {
            self.settings = self.global_settings.clone();
        } else {
            self.settings = self
                .scene_settings
                .get(&self.active_scene)
                .cloned()
                .unwrap_or_else(|| self.global_settings.clone());
        }
    }

    /// Toggle scope between Global and Per-Scene.
    pub fn toggle_scope(&mut self) {
        self.set_scope(self.scope.toggle());
    }

    /// Set active scene index. When in Per-Scene scope, automatically persists and loads that scene's settings.
    pub fn set_active_scene(&mut self, scene_idx: usize) {
        if self.active_scene == scene_idx {
            return;
        }
        if self.scope == SettingsScope::PerScene {
            self.sync_active_settings();
            self.active_scene = scene_idx;
            self.settings = self
                .scene_settings
                .get(&scene_idx)
                .cloned()
                .unwrap_or_else(|| self.global_settings.clone());
        } else {
            self.active_scene = scene_idx;
        }
    }

    /// Persist current settings into either global_settings or scene_settings.
    pub fn sync_active_settings(&mut self) {
        if self.scope == SettingsScope::Global {
            self.global_settings = self.settings.clone();
        } else {
            self.scene_settings
                .insert(self.active_scene, self.settings.clone());
        }
    }

    pub const TOP_OFFSET: f32 = crate::studio::topbar::TopBarUi::HEIGHT;
    pub const TAB_BAR_HEIGHT: f32 = crate::studio::tabs::StudioTabSystem::BAR_HEIGHT;

    /// Check if cursor is hovered over the visuals sidebar.
    pub fn is_hovered(
        &self,
        cursor: (f32, f32),
        screen_w: f32,
        screen_h: f32,
        bottom_offset: f32,
    ) -> bool {
        if !self.open {
            return false;
        }
        let top_offset = Self::TOP_OFFSET;
        let panel_h = (screen_h - top_offset - bottom_offset).max(80.0);
        let panel_x = screen_w - self.width;
        let panel_y = top_offset;
        cursor.0 >= panel_x
            && cursor.0 <= screen_w
            && cursor.1 >= panel_y
            && cursor.1 <= panel_y + panel_h
    }

    /// Scroll content with mouse wheel.
    pub fn handle_scroll(&mut self, delta_y: f32, screen_h: f32, bottom_offset: f32) -> bool {
        if !self.open {
            return false;
        }
        let top_offset = Self::TOP_OFFSET;
        let panel_h = (screen_h - top_offset - bottom_offset).max(80.0);
        let content_h = panel_h - 22.0;
        let total_content_h = 480.0;
        let max_scroll = (total_content_h - content_h).max(0.0);
        self.scroll_y = (self.scroll_y - delta_y * 18.0).clamp(0.0, max_scroll);
        true
    }

    /// Calculate the composition box bounds `[x, y, w, h]` strictly constrained between
    /// the top menu bar (y = 20) + tab bar and timeline (y = screen_h - bottom_offset), and offset horizontally
    /// when the visuals sidebar is open.
    pub fn comp_box_rect(&self, screen_w: f32, screen_h: f32, bottom_offset: f32) -> [f32; 4] {
        let top_offset = Self::TOP_OFFSET;
        let tab_bar_h = Self::TAB_BAR_HEIGHT;
        let right_offset = if self.open { self.width } else { 0.0 };
        let workspace_x = 0.0;
        let workspace_y = top_offset + tab_bar_h;
        let workspace_w = (screen_w - right_offset).max(10.0);
        let workspace_h = (screen_h - workspace_y - bottom_offset).max(10.0);

        let pad_x = (workspace_w * 0.05).round().max(16.0);
        let pad_y = (workspace_h * 0.05).round().max(16.0);
        let max_comp_w = (workspace_w - pad_x * 2.0).max(10.0);
        let max_comp_h = (workspace_h - pad_y * 2.0).max(10.0);

        let canvas_ratio = 16.0 / 9.0;
        let (comp_w, comp_h) = if max_comp_w / max_comp_h > canvas_ratio {
            let h = max_comp_h;
            ((h * canvas_ratio).round(), h)
        } else {
            let w = max_comp_w;
            (w, (w / canvas_ratio).round())
        };

        let comp_x = workspace_x + ((workspace_w - comp_w) * 0.5).round();
        let comp_y = workspace_y + ((workspace_h - comp_h) * 0.5).round();

        [comp_x, comp_y, comp_w, comp_h]
    }

    /// Composition viewport in render-target pixels. Studio UI geometry uses `gui_size`,
    /// while the renderer may target a differently scaled framebuffer.
    pub fn comp_box_viewport(
        &self,
        gui_size: [f32; 2],
        framebuffer_size: [u32; 2],
        bottom_offset: f32,
    ) -> Option<[u32; 4]> {
        if !self.settings.editor_canvas_box
            || gui_size[0] <= 0.0
            || gui_size[1] <= 0.0
            || framebuffer_size[0] == 0
            || framebuffer_size[1] == 0
        {
            return None;
        }
        let [x, y, width, height] = self.comp_box_rect(gui_size[0], gui_size[1], bottom_offset);
        let scale_x = framebuffer_size[0] as f32 / gui_size[0];
        let scale_y = framebuffer_size[1] as f32 / gui_size[1];
        let left = (x * scale_x).round().clamp(0.0, framebuffer_size[0] as f32) as u32;
        let top = (y * scale_y).round().clamp(0.0, framebuffer_size[1] as f32) as u32;
        let right = ((x + width) * scale_x)
            .round()
            .clamp(left as f32, framebuffer_size[0] as f32) as u32;
        let bottom = ((y + height) * scale_y)
            .round()
            .clamp(top as f32, framebuffer_size[1] as f32) as u32;
        Some([
            left,
            top,
            right.saturating_sub(left).max(1),
            bottom.saturating_sub(top).max(1),
        ])
    }

    /// Render framing guides (Rule of Thirds, Center Guide, Aspect Ratio matte bars, AE Composition Box).
    pub fn render_guides(&self, batch: &mut HudBatch, atlas: &HudAtlas, bottom_offset: f32) {
        let (w, h) = (batch.width, batch.height);

        let (guide_x, guide_y, guide_w, guide_h) = if self.settings.editor_canvas_box {
            let [comp_x, comp_y, comp_w, comp_h] = self.comp_box_rect(w, h, bottom_offset);
            let top_offset = Self::TOP_OFFSET;
            let workspace_w = if self.open {
                (w - self.width).max(0.0)
            } else {
                w
            };

            // Workspace dark charcoal matte surrounding the composition box (strictly bounded by workspace_w)
            let matte_color = 0xFF14_161D;
            let tab_bar_y = comp_y - Self::TAB_BAR_HEIGHT;
            // Top matte (between topbar and tab bar, constrained to workspace_w)
            if tab_bar_y > top_offset && workspace_w > 0.0 {
                style::fill(
                    batch,
                    atlas,
                    [0.0, top_offset, workspace_w, tab_bar_y - top_offset],
                    matte_color,
                );
            }
            // Left of tab bar matte
            if comp_x > 0.0 {
                style::fill(
                    batch,
                    atlas,
                    [0.0, tab_bar_y, comp_x, Self::TAB_BAR_HEIGHT],
                    matte_color,
                );
            }
            // Right of tab bar matte (up to workspace_w)
            let right_tab_x = comp_x + comp_w;
            let right_tab_w = (workspace_w - right_tab_x).max(0.0);
            if right_tab_w > 0.0 {
                style::fill(
                    batch,
                    atlas,
                    [right_tab_x, tab_bar_y, right_tab_w, Self::TAB_BAR_HEIGHT],
                    matte_color,
                );
            }
            // Bottom matte (between comp box and timeline, constrained to workspace_w)
            let bottom_matte_y = comp_y + comp_h;
            let bottom_matte_h = (h - bottom_offset - bottom_matte_y).max(0.0);
            if bottom_matte_h > 0.0 && workspace_w > 0.0 {
                style::fill(
                    batch,
                    atlas,
                    [0.0, bottom_matte_y, workspace_w, bottom_matte_h],
                    matte_color,
                );
            }
            // Left matte
            if comp_x > 0.0 {
                style::fill(batch, atlas, [0.0, comp_y, comp_x, comp_h], matte_color);
            }
            // Right matte (up to workspace_w, never bleeding behind sidebar)
            let right_matte_x = comp_x + comp_w;
            let right_matte_w = (workspace_w - right_matte_x).max(0.0);
            if right_matte_w > 0.0 {
                style::fill(
                    batch,
                    atlas,
                    [right_matte_x, comp_y, right_matte_w, comp_h],
                    matte_color,
                );
            }

            // 1px Composition frame border
            let border_color = 0xFF4A_5264;
            style::fill(
                batch,
                atlas,
                [comp_x - 1.0, comp_y - 1.0, comp_w + 2.0, 1.0],
                border_color,
            );
            style::fill(
                batch,
                atlas,
                [comp_x - 1.0, comp_y + comp_h, comp_w + 2.0, 1.0],
                border_color,
            );
            style::fill(
                batch,
                atlas,
                [comp_x - 1.0, comp_y, 1.0, comp_h],
                border_color,
            );
            style::fill(
                batch,
                atlas,
                [comp_x + comp_w, comp_y, 1.0, comp_h],
                border_color,
            );

            // Extensible Composition Tab Bar header spanning comp_w directly atop composition canvas
            let tab_bar_rect = [comp_x, tab_bar_y, comp_w, Self::TAB_BAR_HEIGHT];
            style::fill(batch, atlas, tab_bar_rect, 0xFF18_1B22);
            style::fill(batch, atlas, [comp_x, tab_bar_y, comp_w, 1.0], 0xFF36_3D4D);
            style::fill(
                batch,
                atlas,
                [comp_x - 1.0, tab_bar_y, 1.0, Self::TAB_BAR_HEIGHT],
                0xFF36_3D4D,
            );
            style::fill(
                batch,
                atlas,
                [comp_x + comp_w, tab_bar_y, 1.0, Self::TAB_BAR_HEIGHT],
                0xFF36_3D4D,
            );

            let num_tabs = self.tabs.tabs().len().max(1);
            let tab_w = comp_w / num_tabs as f32;

            for (idx, tab) in self.tabs.tabs().iter().enumerate() {
                let is_active = idx == self.tabs.active_index();
                let title = if let Some(custom) = &tab.custom_title {
                    custom.clone()
                } else {
                    crate::lang::get(&tab.title_key)
                };

                let tab_x = comp_x + idx as f32 * tab_w;
                let tab_rect = [tab_x, tab_bar_y + 1.0, tab_w, Self::TAB_BAR_HEIGHT - 1.0];

                if is_active {
                    style::fill(batch, atlas, tab_rect, 0xFF28_2E3B);
                    // Active top accent line matching tab width exactly
                    style::fill(batch, atlas, [tab_x, tab_bar_y, tab_w, 2.0], style::ACCENT);
                }

                // Separator line between tabs
                if idx + 1 < num_tabs {
                    style::fill(
                        batch,
                        atlas,
                        [
                            tab_x + tab_w - 1.0,
                            tab_bar_y + 3.0,
                            1.0,
                            Self::TAB_BAR_HEIGHT - 6.0,
                        ],
                        if is_active { 0xFF3E_4759 } else { 0xFF2A_303C },
                    );
                }

                let close_w = if tab.closable { 16.0 } else { 0.0 };
                let max_text_w = (tab_w - 16.0 - close_w).max(10.0);
                let fitted_title = style::fit(atlas, &title, max_text_w);
                batch.text_styled(
                    atlas,
                    &Text::literal(fitted_title),
                    tab_x + 8.0,
                    tab_bar_y + 6.0,
                    if is_active {
                        style::WHITE
                    } else {
                        style::MUTED
                    },
                    true,
                );

                if tab.closable && tab_w >= 40.0 {
                    let close_x = tab_x + tab_w - 14.0;
                    batch.text_styled(
                        atlas,
                        &Text::literal("x"),
                        close_x,
                        tab_bar_y + 5.0,
                        style::MUTED,
                        true,
                    );
                }
            }

            // Resolution/zoom badge at bottom-right
            style::fill(
                batch,
                atlas,
                [comp_x + comp_w - 76.0, comp_y + comp_h - 18.0, 72.0, 15.0],
                0xCC1F_232B,
            );
            batch.text_styled(
                atlas,
                &Text::translate("horizon.studio.visuals.composition.fit", &[]),
                comp_x + comp_w - 70.0,
                comp_y + comp_h - 16.0,
                style::MUTED,
                true,
            );

            // Corner registration tick marks
            let tick_color = 0xAA6E_788E;
            let tick_len = 8.0;
            style::fill(
                batch,
                atlas,
                [comp_x + 4.0, comp_y + 4.0, tick_len, 1.0],
                tick_color,
            );
            style::fill(
                batch,
                atlas,
                [comp_x + 4.0, comp_y + 4.0, 1.0, tick_len],
                tick_color,
            );
            style::fill(
                batch,
                atlas,
                [
                    comp_x + comp_w - 4.0 - tick_len,
                    comp_y + 4.0,
                    tick_len,
                    1.0,
                ],
                tick_color,
            );
            style::fill(
                batch,
                atlas,
                [comp_x + comp_w - 5.0, comp_y + 4.0, 1.0, tick_len],
                tick_color,
            );
            style::fill(
                batch,
                atlas,
                [comp_x + 4.0, comp_y + comp_h - 5.0, tick_len, 1.0],
                tick_color,
            );
            style::fill(
                batch,
                atlas,
                [
                    comp_x + 4.0,
                    comp_y + comp_h - 4.0 - tick_len,
                    1.0,
                    tick_len,
                ],
                tick_color,
            );
            style::fill(
                batch,
                atlas,
                [
                    comp_x + comp_w - 4.0 - tick_len,
                    comp_y + comp_h - 5.0,
                    tick_len,
                    1.0,
                ],
                tick_color,
            );
            style::fill(
                batch,
                atlas,
                [
                    comp_x + comp_w - 5.0,
                    comp_y + comp_h - 4.0 - tick_len,
                    1.0,
                    tick_len,
                ],
                tick_color,
            );

            // Aspect Ratio Matte Bars (Letterbox / Pillarbox) overlay inside composition box
            let (active_x, active_y, active_w, active_h) =
                if let Some(target) = self.settings.aspect_ratio.target_ratio() {
                    let current = comp_w / comp_h;
                    let matte_color = 0xFF00_0000;
                    if current > target {
                        let active_w = (comp_h * target).round();
                        let bar_w = ((comp_w - active_w) * 0.5).round();
                        style::fill(batch, atlas, [comp_x, comp_y, bar_w, comp_h], matte_color);
                        style::fill(
                            batch,
                            atlas,
                            [comp_x + comp_w - bar_w, comp_y, bar_w, comp_h],
                            matte_color,
                        );
                        (comp_x + bar_w, comp_y, active_w, comp_h)
                    } else if current < target {
                        let active_h = (comp_w / target).round();
                        let bar_h = ((comp_h - active_h) * 0.5).round();
                        style::fill(batch, atlas, [comp_x, comp_y, comp_w, bar_h], matte_color);
                        style::fill(
                            batch,
                            atlas,
                            [comp_x, comp_y + comp_h - bar_h, comp_w, bar_h],
                            matte_color,
                        );
                        (comp_x, comp_y + bar_h, comp_w, active_h)
                    } else {
                        (comp_x, comp_y, comp_w, comp_h)
                    }
                } else {
                    (comp_x, comp_y, comp_w, comp_h)
                };

            (active_x, active_y, active_w, active_h)
        } else {
            // 1. Aspect Ratio Matte Bars (Letterbox / Pillarbox) when full screen
            if let Some(target) = self.settings.aspect_ratio.target_ratio() {
                let current = w / h;
                let matte_color = 0xFF00_0000;
                if current > target {
                    let active_w = (h * target).round();
                    let bar_w = ((w - active_w) * 0.5).round();
                    style::fill(batch, atlas, [0.0, 0.0, bar_w, h], matte_color);
                    style::fill(batch, atlas, [w - bar_w, 0.0, bar_w, h], matte_color);
                } else if current < target {
                    let active_h = (w / target).round();
                    let bar_h = ((h - active_h) * 0.5).round();
                    style::fill(batch, atlas, [0.0, 0.0, w, bar_h], matte_color);
                    style::fill(batch, atlas, [0.0, h - bar_h, w, bar_h], matte_color);
                }
            }
            (0.0, 0.0, w, h)
        };

        // 2. Rule of Thirds Guide (2 vertical lines, 2 horizontal lines)
        if self.settings.guide_rule_of_thirds {
            let guide_color = 0x66FF_FFFF;
            let x1 = guide_x + (guide_w / 3.0).round();
            let x2 = guide_x + ((2.0 * guide_w) / 3.0).round();
            let y1 = guide_y + (guide_h / 3.0).round();
            let y2 = guide_y + ((2.0 * guide_h) / 3.0).round();

            style::fill(batch, atlas, [x1, guide_y, 1.0, guide_h], guide_color);
            style::fill(batch, atlas, [x2, guide_y, 1.0, guide_h], guide_color);
            style::fill(batch, atlas, [guide_x, y1, guide_w, 1.0], guide_color);
            style::fill(batch, atlas, [guide_x, y2, guide_w, 1.0], guide_color);
        }

        // 3. Center Crosshair Guide
        if self.settings.guide_center {
            let center_color = 0x88FF_FFFF;
            let cx = guide_x + (guide_w * 0.5).round();
            let cy = guide_y + (guide_h * 0.5).round();
            let len = 10.0;
            style::fill(
                batch,
                atlas,
                [cx - len, cy, len * 2.0 + 1.0, 1.0],
                center_color,
            );
            style::fill(
                batch,
                atlas,
                [cx, cy - len, 1.0, len * 2.0 + 1.0],
                center_color,
            );
        }
    }

    /// Render the sidebar panel constrained between the top menu bar and the bottom timeline.
    pub fn render(
        &self,
        batch: &mut HudBatch,
        atlas: &HudAtlas,
        cursor: (f32, f32),
        bottom_offset: f32,
    ) {
        if !self.open {
            return;
        }

        let top_offset = Self::TOP_OFFSET;
        let screen_w = batch.width;
        let screen_h = batch.height;
        let panel_h = (screen_h - top_offset - bottom_offset).max(80.0);
        let panel_x = screen_w - self.width;
        let panel_y = top_offset;
        let rect = [panel_x, panel_y, self.width, panel_h];

        // Background panel: solid dark studio charcoal (opaque so sky never bleeds through as a white box)
        style::fill(batch, atlas, rect, 0xFF14_161D);
        style::fill(batch, atlas, [panel_x, panel_y, 1.0, panel_h], 0xFF2D_323F);

        // Left resize border handle
        let handle_hovered = cursor.0 >= panel_x - 3.0
            && cursor.0 <= panel_x + 3.0
            && cursor.1 >= panel_y
            && cursor.1 <= panel_y + panel_h;
        if self.resizing_width {
            style::fill(
                batch,
                atlas,
                [panel_x - 1.0, panel_y, 2.0, panel_h],
                style::ACCENT,
            );
        } else if handle_hovered {
            style::fill(
                batch,
                atlas,
                [panel_x - 1.0, panel_y, 2.0, panel_h],
                0xAAFF_FFFF,
            );
        }

        // Header: solid studio header dark
        let head_h = 22.0;
        style::fill(
            batch,
            atlas,
            [panel_x, panel_y, self.width, head_h],
            0xFF18_1B24,
        );
        style::fill(
            batch,
            atlas,
            [panel_x, panel_y + head_h - 1.0, self.width, 1.0],
            0xFF2D_323F,
        );

        batch.text_styled(
            atlas,
            &Text::translate("horizon.studio.visuals.title", &[]),
            panel_x + 8.0,
            panel_y + 5.0,
            style::WHITE,
            true,
        );

        // Close button [x]
        let close_rect = [panel_x + self.width - 18.0, panel_y + 3.0, 15.0, 15.0];
        let close_hover = ui::hovered(
            cursor,
            close_rect[0],
            close_rect[1],
            close_rect[2],
            close_rect[3],
        );
        style::button(
            batch,
            atlas,
            close_rect,
            &Text::literal("X"),
            (close_hover, true, false),
        );

        // Content Area with scroll clipping
        let content_top = panel_y + head_h;
        let content_bottom = panel_y + panel_h;
        let content_h = content_bottom - content_top;
        let right = panel_x + self.width - 8.0;

        let mut row_y = 6.0;

        // Scope row: Scope label and [Global] / [Per-Scene] toggle button
        let scope_lbl = if self.scope == SettingsScope::PerScene {
            Text::translate(
                "horizon.studio.visuals.scope.scene",
                &[Text::literal((self.active_scene + 1).to_string())],
            )
        } else {
            Text::translate("horizon.studio.visuals.scope.global_label", &[])
        };
        let scope_btn_text = Text::translate(self.scope.label_key(), &[]);
        self.render_row_button_toggle(
            batch,
            atlas,
            panel_x,
            right,
            content_top,
            content_bottom,
            &mut row_y,
            cursor,
            &scope_lbl,
            self.scope == SettingsScope::PerScene,
            &scope_btn_text,
        );
        row_y += 4.0;

        // Group 1: Editor
        self.render_row_section(
            batch,
            atlas,
            panel_x,
            content_top,
            content_bottom,
            &mut row_y,
            "horizon.studio.visuals.section.editor",
        );
        self.render_row_toggle(
            batch,
            atlas,
            panel_x,
            right,
            content_top,
            content_bottom,
            &mut row_y,
            cursor,
            "horizon.studio.visuals.composition.box",
            self.settings.editor_canvas_box,
        );

        // Group 2: World
        self.render_row_section(
            batch,
            atlas,
            panel_x,
            content_top,
            content_bottom,
            &mut row_y,
            "horizon.studio.visuals.section.world",
        );
        self.render_row_toggle(
            batch,
            atlas,
            panel_x,
            right,
            content_top,
            content_bottom,
            &mut row_y,
            cursor,
            "horizon.studio.visuals.blocks",
            self.settings.world_blocks,
        );
        self.render_row_toggle(
            batch,
            atlas,
            panel_x,
            right,
            content_top,
            content_bottom,
            &mut row_y,
            cursor,
            "horizon.studio.visuals.entities",
            self.settings.world_entities,
        );
        self.render_row_toggle(
            batch,
            atlas,
            panel_x,
            right,
            content_top,
            content_bottom,
            &mut row_y,
            cursor,
            "horizon.studio.visuals.particles",
            self.settings.world_particles,
        );
        self.render_row_toggle(
            batch,
            atlas,
            panel_x,
            right,
            content_top,
            content_bottom,
            &mut row_y,
            cursor,
            "horizon.studio.visuals.players",
            self.settings.world_players,
        );
        self.render_row_toggle(
            batch,
            atlas,
            panel_x,
            right,
            content_top,
            content_bottom,
            &mut row_y,
            cursor,
            "horizon.studio.visuals.world.sky",
            self.settings.world_sky,
        );
        self.render_row_toggle(
            batch,
            atlas,
            panel_x,
            right,
            content_top,
            content_bottom,
            &mut row_y,
            cursor,
            "horizon.studio.visuals.world.nametags",
            self.settings.world_nametags,
        );

        // Group 4: Overrides
        self.render_row_section(
            batch,
            atlas,
            panel_x,
            content_top,
            content_bottom,
            &mut row_y,
            "horizon.studio.visuals.section.overrides",
        );
        let fog_label = Text::translate(
            "horizon.studio.visuals.override.fog",
            &[Text::literal(
                (self.settings.fog_distance as i32).to_string(),
            )],
        );
        self.render_row_button_toggle(
            batch,
            atlas,
            panel_x,
            right,
            content_top,
            content_bottom,
            &mut row_y,
            cursor,
            &fog_label,
            self.settings.override_fog,
            &Text::translate("horizon.studio.visuals.cycle", &[]),
        );
        let fov_label = Text::translate(
            "horizon.studio.visuals.override.fov",
            &[Text::literal((self.settings.fov_value as i32).to_string())],
        );
        self.render_row_button_toggle(
            batch,
            atlas,
            panel_x,
            right,
            content_top,
            content_bottom,
            &mut row_y,
            cursor,
            &fov_label,
            self.settings.override_fov,
            &Text::translate("horizon.studio.visuals.cycle", &[]),
        );
        let time_label = Text::translate(
            "horizon.studio.visuals.override.time",
            &[Text::translate(self.settings.time_preset_key(), &[])],
        );
        self.render_row_button_toggle(
            batch,
            atlas,
            panel_x,
            right,
            content_top,
            content_bottom,
            &mut row_y,
            cursor,
            &time_label,
            self.settings.override_time,
            &Text::translate("horizon.studio.visuals.cycle", &[]),
        );
        let shake_label = Text::translate(
            "horizon.studio.visuals.override.shake",
            &[Text::translate(self.settings.shake_key(), &[])],
        );
        self.render_row_button_toggle(
            batch,
            atlas,
            panel_x,
            right,
            content_top,
            content_bottom,
            &mut row_y,
            cursor,
            &shake_label,
            self.settings.override_shake,
            &Text::translate("horizon.studio.visuals.cycle", &[]),
        );
        let roll_label = Text::translate(
            "horizon.studio.visuals.override.roll",
            &[Text::literal(
                (self.settings.camera_roll as i32).to_string(),
            )],
        );
        self.render_row_button_toggle(
            batch,
            atlas,
            panel_x,
            right,
            content_top,
            content_bottom,
            &mut row_y,
            cursor,
            &roll_label,
            self.settings.override_roll,
            &Text::translate("horizon.studio.visuals.cycle", &[]),
        );

        // Group 5: Guides & Other
        self.render_row_section(
            batch,
            atlas,
            panel_x,
            content_top,
            content_bottom,
            &mut row_y,
            "horizon.studio.visuals.section.guides",
        );
        self.render_row_toggle(
            batch,
            atlas,
            panel_x,
            right,
            content_top,
            content_bottom,
            &mut row_y,
            cursor,
            "horizon.studio.visuals.rule_of_thirds",
            self.settings.guide_rule_of_thirds,
        );
        self.render_row_toggle(
            batch,
            atlas,
            panel_x,
            right,
            content_top,
            content_bottom,
            &mut row_y,
            cursor,
            "horizon.studio.visuals.center_guide",
            self.settings.guide_center,
        );
        self.render_row_toggle(
            batch,
            atlas,
            panel_x,
            right,
            content_top,
            content_bottom,
            &mut row_y,
            cursor,
            "horizon.studio.visuals.camera_path",
            self.settings.camera_path_3d,
        );

        // Aspect Ratio Row
        let y_screen = content_top + row_y - self.scroll_y;
        if y_screen >= content_top && y_screen + 16.0 <= content_bottom {
            let aspect_rect = [right - 48.0, y_screen, 48.0, 15.0];
            let aspect_hover = ui::hovered(
                cursor,
                aspect_rect[0],
                aspect_rect[1],
                aspect_rect[2],
                aspect_rect[3],
            );
            batch.text_styled(
                atlas,
                &Text::translate("horizon.studio.visuals.aspect", &[]),
                panel_x + 10.0,
                y_screen + 2.0,
                style::WHITE,
                true,
            );
            style::button(
                batch,
                atlas,
                aspect_rect,
                &Text::translate(self.settings.aspect_ratio.label_key(), &[]),
                (aspect_hover, true, false),
            );
        }
        row_y += 18.0;

        // Scrollbar track & thumb
        let total_content_h = row_y + 10.0;
        let max_scroll = (total_content_h - content_h).max(0.0);
        if max_scroll > 0.0 {
            let sb_x = panel_x + self.width - 4.0;
            style::fill(
                batch,
                atlas,
                [sb_x, content_top, 2.0, content_h],
                0x22FF_FFFF,
            );
            let thumb_h = ((content_h / total_content_h) * content_h).max(18.0);
            let thumb_y = content_top + (self.scroll_y / max_scroll) * (content_h - thumb_h);
            style::fill(
                batch,
                atlas,
                [sb_x - 1.0, thumb_y, 3.0, thumb_h],
                0x88CC_CCCC,
            );
        }
    }

    fn render_row_section(
        &self,
        batch: &mut HudBatch,
        atlas: &HudAtlas,
        panel_x: f32,
        top: f32,
        bottom: f32,
        y: &mut f32,
        title: &str,
    ) {
        let y_screen = top + *y - self.scroll_y;
        if y_screen >= top - 14.0 && y_screen + 14.0 <= bottom {
            style::fill(
                batch,
                atlas,
                [panel_x + 6.0, y_screen + 11.0, self.width - 12.0, 1.0],
                style::ROW_RULE,
            );
            batch.text_styled(
                atlas,
                &Text::translate(title, &[]),
                panel_x + 8.0,
                y_screen,
                style::SOFT,
                true,
            );
        }
        *y += 16.0;
    }

    fn render_row_toggle(
        &self,
        batch: &mut HudBatch,
        atlas: &HudAtlas,
        panel_x: f32,
        right: f32,
        top: f32,
        bottom: f32,
        y: &mut f32,
        cursor: (f32, f32),
        label: &str,
        val: bool,
    ) {
        let y_screen = top + *y - self.scroll_y;
        if y_screen >= top - 2.0 && y_screen + 16.0 <= bottom + 2.0 {
            let row_rect = [panel_x + 4.0, y_screen, self.width - 8.0, 17.0];
            let hovered = ui::hovered(cursor, row_rect[0], row_rect[1], row_rect[2], row_rect[3]);
            if hovered {
                style::fill(
                    batch,
                    atlas,
                    [panel_x + 6.0, y_screen + 1.0, self.width - 12.0, 15.0],
                    0x22FF_FFFF,
                );
            }
            let text_color = if val || hovered {
                style::WHITE
            } else {
                style::MUTED
            };
            batch.text_styled(
                atlas,
                &Text::translate(label, &[]),
                panel_x + 10.0,
                y_screen + 2.0,
                text_color,
                true,
            );
            style::checkbox(batch, atlas, right - 4.0, y_screen, val, true);
        }
        *y += 17.0;
    }

    fn render_row_button_toggle(
        &self,
        batch: &mut HudBatch,
        atlas: &HudAtlas,
        panel_x: f32,
        right: f32,
        top: f32,
        bottom: f32,
        y: &mut f32,
        cursor: (f32, f32),
        label: &Text,
        enabled: bool,
        btn_text: &Text,
    ) {
        let y_screen = top + *y - self.scroll_y;
        if y_screen >= top && y_screen + 16.0 <= bottom {
            let color = if enabled { style::WHITE } else { style::MUTED };
            batch.text_styled(atlas, label, panel_x + 10.0, y_screen + 2.0, color, true);

            let btn_rect = [right - 44.0, y_screen, 44.0, 15.0];
            let hover = ui::hovered(cursor, btn_rect[0], btn_rect[1], btn_rect[2], btn_rect[3]);
            style::button(batch, atlas, btn_rect, btn_text, (hover, true, false));
        }
        *y += 18.0;
    }

    /// Handle mouse click inside composition tab bar or sidebar. Returns true if handled.
    pub fn handle_mouse_click(
        &mut self,
        cursor: (f32, f32),
        screen_w: f32,
        screen_h: f32,
        bottom_offset: f32,
    ) -> bool {
        if self.settings.editor_canvas_box {
            let [comp_x, comp_y, comp_w, _] = self.comp_box_rect(screen_w, screen_h, bottom_offset);
            let tab_bar_y = comp_y - Self::TAB_BAR_HEIGHT;
            if cursor.0 >= comp_x
                && cursor.0 <= comp_x + comp_w
                && cursor.1 >= tab_bar_y
                && cursor.1 <= tab_bar_y + Self::TAB_BAR_HEIGHT
            {
                let num_tabs = self.tabs.tabs().len().max(1);
                let tab_w = comp_w / num_tabs as f32;
                let clicked_idx = ((cursor.0 - comp_x) / tab_w).floor() as usize;
                if clicked_idx < self.tabs.tabs().len() {
                    let tab_x = comp_x + clicked_idx as f32 * tab_w;
                    let tab = &self.tabs.tabs()[clicked_idx];
                    if tab.closable && cursor.0 >= tab_x + tab_w - 16.0 {
                        self.tabs.close_tab_at(clicked_idx);
                    } else {
                        self.tabs.select_tab(clicked_idx);
                    }
                    return true;
                }
            }
        }

        let handled = self.handle_mouse_click_inner(cursor, screen_w, screen_h, bottom_offset);
        if handled {
            self.sync_active_settings();
        }
        handled
    }

    fn handle_mouse_click_inner(
        &mut self,
        cursor: (f32, f32),
        screen_w: f32,
        screen_h: f32,
        bottom_offset: f32,
    ) -> bool {
        if !self.open {
            return false;
        }

        let top_offset = Self::TOP_OFFSET;
        let panel_h = (screen_h - top_offset - bottom_offset).max(80.0);
        let panel_x = screen_w - self.width;
        let panel_y = top_offset;

        // Check if clicking on left resize handle
        if cursor.0 >= panel_x - 4.0
            && cursor.0 <= panel_x + 4.0
            && cursor.1 >= panel_y
            && cursor.1 <= panel_y + panel_h
        {
            self.resizing_width = true;
            return true;
        }

        if cursor.0 < panel_x
            || cursor.0 > screen_w
            || cursor.1 < panel_y
            || cursor.1 > panel_y + panel_h
        {
            return false;
        }

        // Close button
        let close_rect = [panel_x + self.width - 18.0, panel_y + 3.0, 15.0, 15.0];
        if ui::hovered(
            cursor,
            close_rect[0],
            close_rect[1],
            close_rect[2],
            close_rect[3],
        ) {
            self.open = false;
            return true;
        }

        let content_top = panel_y + 22.0;
        let content_bottom = panel_y + panel_h;
        if cursor.1 < content_top || cursor.1 > content_bottom {
            return true;
        }

        let right = panel_x + self.width - 8.0;
        let mut row_y = 6.0;

        // Scope toggle
        if Self::check_button_toggle(
            cursor,
            panel_x,
            self.width,
            right,
            content_top,
            content_bottom,
            self.scroll_y,
            &mut row_y,
        ) != ButtonToggleAction::None
        {
            self.toggle_scope();
            return true;
        }
        row_y += 4.0;

        // Group 1: Editor
        row_y += 16.0;
        if Self::check_toggle(
            cursor,
            panel_x,
            self.width,
            content_top,
            content_bottom,
            self.scroll_y,
            &mut row_y,
        ) {
            self.settings.editor_canvas_box = !self.settings.editor_canvas_box;
            return true;
        }

        // Group 2: World
        row_y += 16.0;
        if Self::check_toggle(
            cursor,
            panel_x,
            self.width,
            content_top,
            content_bottom,
            self.scroll_y,
            &mut row_y,
        ) {
            self.settings.world_blocks = !self.settings.world_blocks;
            return true;
        }
        if Self::check_toggle(
            cursor,
            panel_x,
            self.width,
            content_top,
            content_bottom,
            self.scroll_y,
            &mut row_y,
        ) {
            self.settings.world_entities = !self.settings.world_entities;
            return true;
        }
        if Self::check_toggle(
            cursor,
            panel_x,
            self.width,
            content_top,
            content_bottom,
            self.scroll_y,
            &mut row_y,
        ) {
            self.settings.world_particles = !self.settings.world_particles;
            return true;
        }
        if Self::check_toggle(
            cursor,
            panel_x,
            self.width,
            content_top,
            content_bottom,
            self.scroll_y,
            &mut row_y,
        ) {
            self.settings.world_players = !self.settings.world_players;
            return true;
        }
        if Self::check_toggle(
            cursor,
            panel_x,
            self.width,
            content_top,
            content_bottom,
            self.scroll_y,
            &mut row_y,
        ) {
            self.settings.world_sky = !self.settings.world_sky;
            return true;
        }
        if Self::check_toggle(
            cursor,
            panel_x,
            self.width,
            content_top,
            content_bottom,
            self.scroll_y,
            &mut row_y,
        ) {
            self.settings.world_nametags = !self.settings.world_nametags;
            return true;
        }

        // Group 4: Overrides
        row_y += 16.0;
        // Fog
        match Self::check_button_toggle(
            cursor,
            panel_x,
            self.width,
            right,
            content_top,
            content_bottom,
            self.scroll_y,
            &mut row_y,
        ) {
            ButtonToggleAction::Toggle => {
                self.settings.override_fog = !self.settings.override_fog;
                return true;
            }
            ButtonToggleAction::Cycle => {
                self.settings.override_fog = true;
                self.settings.cycle_fog_distance();
                return true;
            }
            ButtonToggleAction::None => {}
        }
        // FOV
        match Self::check_button_toggle(
            cursor,
            panel_x,
            self.width,
            right,
            content_top,
            content_bottom,
            self.scroll_y,
            &mut row_y,
        ) {
            ButtonToggleAction::Toggle => {
                self.settings.override_fov = !self.settings.override_fov;
                return true;
            }
            ButtonToggleAction::Cycle => {
                self.settings.override_fov = true;
                self.settings.cycle_fov_preset();
                return true;
            }
            ButtonToggleAction::None => {}
        }
        // Time
        match Self::check_button_toggle(
            cursor,
            panel_x,
            self.width,
            right,
            content_top,
            content_bottom,
            self.scroll_y,
            &mut row_y,
        ) {
            ButtonToggleAction::Toggle => {
                self.settings.override_time = !self.settings.override_time;
                return true;
            }
            ButtonToggleAction::Cycle => {
                self.settings.override_time = true;
                self.settings.cycle_time_preset();
                return true;
            }
            ButtonToggleAction::None => {}
        }
        // Shake
        match Self::check_button_toggle(
            cursor,
            panel_x,
            self.width,
            right,
            content_top,
            content_bottom,
            self.scroll_y,
            &mut row_y,
        ) {
            ButtonToggleAction::Toggle => {
                self.settings.override_shake = !self.settings.override_shake;
                return true;
            }
            ButtonToggleAction::Cycle => {
                self.settings.override_shake = true;
                self.settings.cycle_shake();
                return true;
            }
            ButtonToggleAction::None => {}
        }
        // Camera roll
        match Self::check_button_toggle(
            cursor,
            panel_x,
            self.width,
            right,
            content_top,
            content_bottom,
            self.scroll_y,
            &mut row_y,
        ) {
            ButtonToggleAction::Toggle => {
                self.settings.override_roll = !self.settings.override_roll;
                return true;
            }
            ButtonToggleAction::Cycle => {
                self.settings.override_roll = true;
                self.settings.cycle_roll();
                return true;
            }
            ButtonToggleAction::None => {}
        }

        // Group 5: Guides & Other
        row_y += 16.0;
        if Self::check_toggle(
            cursor,
            panel_x,
            self.width,
            content_top,
            content_bottom,
            self.scroll_y,
            &mut row_y,
        ) {
            self.settings.guide_rule_of_thirds = !self.settings.guide_rule_of_thirds;
            return true;
        }
        if Self::check_toggle(
            cursor,
            panel_x,
            self.width,
            content_top,
            content_bottom,
            self.scroll_y,
            &mut row_y,
        ) {
            self.settings.guide_center = !self.settings.guide_center;
            return true;
        }
        if Self::check_toggle(
            cursor,
            panel_x,
            self.width,
            content_top,
            content_bottom,
            self.scroll_y,
            &mut row_y,
        ) {
            self.settings.camera_path_3d = !self.settings.camera_path_3d;
            return true;
        }

        // Aspect Ratio
        let y_screen = content_top + row_y - self.scroll_y;
        if y_screen >= content_top && y_screen + 16.0 <= content_bottom {
            let row_rect = [panel_x + 4.0, y_screen, self.width - 8.0, 17.0];
            if ui::hovered(cursor, row_rect[0], row_rect[1], row_rect[2], row_rect[3]) {
                self.settings.aspect_ratio = self.settings.aspect_ratio.cycle();
                return true;
            }
        }

        true
    }

    fn check_toggle(
        cursor: (f32, f32),
        panel_x: f32,
        width: f32,
        top: f32,
        bottom: f32,
        scroll_y: f32,
        y: &mut f32,
    ) -> bool {
        let y_screen = top + *y - scroll_y;
        let mut clicked = false;
        if y_screen >= top - 2.0 && y_screen + 16.0 <= bottom + 2.0 {
            let row_rect = [panel_x + 4.0, y_screen, width - 8.0, 17.0];
            if ui::hovered(cursor, row_rect[0], row_rect[1], row_rect[2], row_rect[3]) {
                clicked = true;
            }
        }
        *y += 17.0;
        clicked
    }

    fn check_button_toggle(
        cursor: (f32, f32),
        panel_x: f32,
        width: f32,
        right: f32,
        top: f32,
        bottom: f32,
        scroll_y: f32,
        y: &mut f32,
    ) -> ButtonToggleAction {
        let y_screen = top + *y - scroll_y;
        let mut action = ButtonToggleAction::None;
        if y_screen >= top && y_screen + 16.0 <= bottom {
            let btn_rect = [right - 44.0, y_screen, 44.0, 15.0];
            let label_rect = [panel_x + 10.0, y_screen, width - 58.0, 15.0];
            if ui::hovered(cursor, btn_rect[0], btn_rect[1], btn_rect[2], btn_rect[3]) {
                action = ButtonToggleAction::Cycle;
            } else if ui::hovered(
                cursor,
                label_rect[0],
                label_rect[1],
                label_rect[2],
                label_rect[3],
            ) {
                action = ButtonToggleAction::Toggle;
            }
        }
        *y += 18.0;
        action
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ButtonToggleAction {
    None,
    Toggle,
    Cycle,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_visuals_settings_defaults() {
        let v = VisualsSettings::default();
        assert!(v.world_blocks);
        assert!(v.world_entities);
        assert!(!v.override_fog);
        assert!(v.camera_path_3d);
        assert_eq!(v.aspect_ratio, AspectRatio::None);
        assert!(!v.editor_canvas_box);
    }

    #[test]
    fn test_aspect_ratio_cycling() {
        let mut ar = AspectRatio::None;
        ar = ar.cycle();
        assert_eq!(ar, AspectRatio::Ratio16_9);
        ar = ar.cycle();
        assert_eq!(ar, AspectRatio::Ratio4_3);
    }

    #[test]
    fn test_visuals_presets_cycling() {
        let mut v = VisualsSettings::default();
        assert_eq!(v.time_preset_name(), "Noon");
        v.cycle_time_preset();
        assert_eq!(v.time_preset_name(), "Sunset");
        v.cycle_time_preset();
        assert_eq!(v.time_preset_name(), "Night");
        v.cycle_time_preset();
        assert_eq!(v.time_preset_name(), "Day");

        assert_eq!(v.shake_name(), "Off");
        v.cycle_shake();
        assert_eq!(v.shake_name(), "Subtle");
        v.cycle_shake();
        assert_eq!(v.shake_name(), "Med");
        v.cycle_shake();
        assert_eq!(v.shake_name(), "Heavy");

        let initial_fog = v.fog_distance;
        v.cycle_fog_distance();
        assert_ne!(v.fog_distance, initial_fog);
    }

    #[test]
    fn test_visuals_scrolling() {
        let mut ui = VisualsUi::new();
        ui.open = true;
        assert_eq!(ui.scroll_y, 0.0);
        // Scroll down
        ui.handle_scroll(-1.0, 400.0, 100.0);
        assert!(ui.scroll_y > 0.0);
    }

    #[test]
    fn test_visuals_toggle_mouse_clicks() {
        let mut ui = VisualsUi::new();
        ui.open = true;

        let screen_w = 1280.0;
        let screen_h = 720.0;
        let panel_x = screen_w - ui.width; // 1085.0
        let _content_top = 24.0 + 22.0; // 46.0

        // Scope toggle (row_y = 6.0, y_screen = 46.0 + 6.0 = 52.0)
        assert_eq!(ui.scope, SettingsScope::Global);
        let handled = ui.handle_mouse_click((panel_x + 30.0, 58.0), screen_w, screen_h, 0.0);
        assert!(handled);
        assert_eq!(ui.scope, SettingsScope::PerScene);
        // Toggle back to Global for remaining toggle tests
        ui.set_scope(SettingsScope::Global);

        // 1. Group 1: Editor -> Composition Box (row_y = 6.0 + 18.0 + 4.0 + 16.0 = 44.0, y_screen = 46.0 + 44.0 = 90.0)
        assert!(!ui.settings.editor_canvas_box);
        // Click on the text label "Composition Box" at x = panel_x + 30.0, y = 94.0
        let handled = ui.handle_mouse_click((panel_x + 30.0, 94.0), screen_w, screen_h, 0.0);
        assert!(handled);
        assert!(ui.settings.editor_canvas_box);

        // Click again on the checkbox at x = screen_w - 15.0, y = 94.0
        let handled = ui.handle_mouse_click((screen_w - 15.0, 94.0), screen_w, screen_h, 0.0);
        assert!(handled);
        assert!(!ui.settings.editor_canvas_box);

        // 2. Group 2: World -> Blocks (row_y = 44.0 + 17.0 + 16.0 = 77.0, y_screen = 46.0 + 77.0 = 123.0)
        assert!(ui.settings.world_blocks);
        // Click the World Blocks row at x = panel_x + 20.0, y = 127.0
        let handled = ui.handle_mouse_click((panel_x + 20.0, 127.0), screen_w, screen_h, 0.0);
        assert!(handled);
        assert!(!ui.settings.world_blocks);
    }

    #[test]
    fn test_scope_and_per_scene_overrides() {
        let mut ui = VisualsUi::new();
        ui.open = true;

        // Default scope is Global
        assert_eq!(ui.scope, SettingsScope::Global);
        ui.toggle_scope();
        assert_eq!(ui.scope, SettingsScope::PerScene);

        // In PerScene scope for scene 0: toggle world blocks off
        ui.settings.world_blocks = false;
        ui.sync_active_settings();

        // Switch to scene 1: should have default (true)
        ui.set_active_scene(1);
        assert!(ui.settings.world_blocks);

        // Switch back to scene 0: should be false
        ui.set_active_scene(0);
        assert!(!ui.settings.world_blocks);

        // Switch back to Global scope: should have global default (true)
        ui.set_scope(SettingsScope::Global);
        assert!(ui.settings.world_blocks);
    }

    #[test]
    fn test_comp_box_constraints_and_offset() {
        let mut ui = VisualsUi::new();
        let screen_w = 1920.0;
        let screen_h = 1080.0;
        let bottom_offset = 120.0; // Timeline height

        // 1. When sidebar is closed: centered within full screen_w
        ui.open = false;
        let [x1, y1, w1, h1] = ui.comp_box_rect(screen_w, screen_h, bottom_offset);
        assert!(
            y1 >= VisualsUi::TOP_OFFSET,
            "comp box y must be below top bar"
        );
        assert!(
            y1 + h1 <= screen_h - bottom_offset,
            "comp box bottom must be above timeline"
        );
        let center_x1 = x1 + w1 * 0.5;
        assert!(
            (center_x1 - (screen_w * 0.5)).abs() < 2.0,
            "comp box should be centered in screen"
        );

        // 2. When sidebar is open: offset/centered in remaining workspace
        ui.open = true;
        let [x2, y2, w2, _h2] = ui.comp_box_rect(screen_w, screen_h, bottom_offset);
        let workspace_w = screen_w - ui.width;
        let center_x2 = x2 + w2 * 0.5;
        assert!(
            (center_x2 - (workspace_w * 0.5)).abs() < 2.0,
            "comp box should be centered in available workspace"
        );
        assert!(
            center_x2 < center_x1,
            "comp box must shift left when visuals panel opens"
        );
        assert!(
            x2 + w2 <= workspace_w,
            "comp box must not overlap the visuals panel"
        );
        assert!(y2 >= VisualsUi::TOP_OFFSET);
    }

    #[test]
    fn composition_viewport_tracks_framebuffer_scale_and_toggle() {
        let mut ui = VisualsUi::new();
        assert_eq!(
            ui.comp_box_viewport([960.0, 540.0], [1920, 1080], 60.0),
            None
        );

        ui.settings.editor_canvas_box = true;
        let viewport = ui
            .comp_box_viewport([960.0, 540.0], [1920, 1080], 60.0)
            .expect("composition viewport is enabled");
        let [x, y, width, height] = ui.comp_box_rect(960.0, 540.0, 60.0);
        assert_eq!(
            viewport,
            [
                (x * 2.0).round() as u32,
                (y * 2.0).round() as u32,
                (width * 2.0).round() as u32,
                (height * 2.0).round() as u32,
            ]
        );
        assert!(viewport[0] + viewport[2] <= 1920);
        assert!(viewport[1] + viewport[3] <= 1080);
    }

    #[test]
    fn cycle_roll_skips_redundant_negative_180() {
        let mut settings = VisualsSettings {
            camera_roll: 0.0,
            ..Default::default()
        };
        settings.cycle_roll();
        assert_eq!(settings.camera_roll, 90.0);
        settings.cycle_roll();
        assert_eq!(settings.camera_roll, 180.0);
        settings.cycle_roll();
        assert_eq!(settings.camera_roll, -90.0);
        settings.cycle_roll();
        assert_eq!(settings.camera_roll, 0.0);

        // If starting at -180, normalize to 180 and cycle to -90
        settings.camera_roll = -180.0;
        settings.cycle_roll();
        assert_eq!(settings.camera_roll, -90.0);
    }

    #[test]
    fn visuals_sidebar_resizing_clamps_within_bounds() {
        let mut ui = VisualsUi::new();
        assert!(!ui.resizing_width);
        ui.handle_resize_drag(500.0, 960.0);
        assert_eq!(ui.width, 195.0);

        ui.resizing_width = true;
        ui.handle_resize_drag(700.0, 960.0);
        assert_eq!(ui.width, 260.0);

        ui.handle_resize_drag(900.0, 960.0);
        assert_eq!(ui.width, 160.0);

        ui.handle_resize_drag(100.0, 960.0);
        assert_eq!(ui.width, 480.0);
    }

    #[test]
    fn aspect_ratio_overlay_preserves_canvas_size() {
        let mut ui = VisualsUi::new();
        let default_rect = ui.comp_box_rect(1920.0, 1080.0, 120.0);

        ui.settings.aspect_ratio = AspectRatio::Ratio21_9;
        assert_eq!(
            ui.comp_box_rect(1920.0, 1080.0, 120.0),
            default_rect,
            "21:9 should preserve fixed 16:9 canvas size"
        );

        ui.settings.aspect_ratio = AspectRatio::Ratio4_3;
        assert_eq!(
            ui.comp_box_rect(1920.0, 1080.0, 120.0),
            default_rect,
            "4:3 should preserve fixed 16:9 canvas size"
        );
    }

    #[test]
    fn test_tabs_resize_with_composition_and_can_be_selected() {
        let mut ui = VisualsUi::new();
        ui.settings.editor_canvas_box = true;
        let screen_w = 1920.0;
        let screen_h = 1080.0;
        let bottom_offset = 120.0;

        let [x1, y1, w1, _] = ui.comp_box_rect(screen_w, screen_h, bottom_offset);
        assert!(y1 - VisualsUi::TAB_BAR_HEIGHT >= VisualsUi::TOP_OFFSET);

        ui.tabs.register_tab(
            crate::studio::tabs::StudioTab::new("preview", "custom.preview")
                .with_custom_title("3D Preview")
                .with_mod_id("mod_cam"),
        );
        assert_eq!(ui.tabs.tabs().len(), 2);

        // Click on the second tab to switch
        let tab1_w = w1 / 2.0;
        let click_x = x1 + tab1_w + 10.0;
        let click_y = y1 - VisualsUi::TAB_BAR_HEIGHT + 10.0;
        let handled = ui.handle_mouse_click((click_x, click_y), screen_w, screen_h, bottom_offset);
        assert!(handled);
        assert_eq!(ui.tabs.active_index(), 1);

        // Resize visuals sidebar: comp box width resizes, tab bar width matches
        ui.open = true;
        ui.width = 300.0;
        let [x2, y2, w2, _] = ui.comp_box_rect(screen_w, screen_h, bottom_offset);
        assert!(
            w2 < w1,
            "composition box and tab bar resize dynamically with sidebar"
        );
        assert!(y2 - VisualsUi::TAB_BAR_HEIGHT >= VisualsUi::TOP_OFFSET);

        // Close second tab via close button at right of tab 2
        let close_x = x2 + w2 - 5.0;
        let close_y = y2 - VisualsUi::TAB_BAR_HEIGHT + 10.0;
        let handled = ui.handle_mouse_click((close_x, close_y), screen_w, screen_h, bottom_offset);
        assert!(handled);
        assert_eq!(ui.tabs.tabs().len(), 1);
        assert_eq!(ui.tabs.active_index(), 0);
    }
}
