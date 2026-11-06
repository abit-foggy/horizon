//! Visuals & Overrides Sidebar UI
//!
//! Right-docked inspector panel providing toggles for GUI elements, world rendering,
//! cinematic overrides (Fog, FOV, Time of Day, Camera Shake, Roll), and framing guides
//! (Rule of Thirds, Center Crosshair, Aspect Ratio framing).

use horizon_client_render::{
    graphics_style as style,
    hud::{HudAtlas, HudBatch},
    text::Text,
    ui,
};

/// Framing aspect ratio modes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AspectRatio {
    None,
    Ratio16_9,
    Ratio4_3,
    Ratio1_1,
    Ratio21_9,
}

impl AspectRatio {
    pub fn translation_key(self) -> &'static str {
        match self {
            Self::None => "horizon.studio.visuals.aspect_ratio.none",
            Self::Ratio16_9 => "horizon.studio.visuals.aspect_ratio.16_9",
            Self::Ratio4_3 => "horizon.studio.visuals.aspect_ratio.4_3",
            Self::Ratio1_1 => "horizon.studio.visuals.aspect_ratio.1_1",
            Self::Ratio21_9 => "horizon.studio.visuals.aspect_ratio.21_9",
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
#[derive(Clone, Debug, PartialEq)]
pub struct VisualsSettings {
    // World section
    pub world_players: bool,
    pub world_particles: bool,
    pub override_fov: bool,
    pub fov_value: f32,

    // Other / Guides section
    pub guide_rule_of_thirds: bool,
    pub guide_center: bool,
    pub camera_path_3d: bool,
    pub aspect_ratio: AspectRatio,
}

impl Default for VisualsSettings {
    fn default() -> Self {
        Self {
            world_players: true,
            world_particles: true,
            override_fov: false,
            fov_value: 70.0,

            guide_rule_of_thirds: false,
            guide_center: false,
            camera_path_3d: true,
            aspect_ratio: AspectRatio::None,
        }
    }
}

/// Visuals sidebar UI controller and renderer.
#[derive(Clone, Debug)]
pub struct VisualsUi {
    pub open: bool,
    pub width: f32,
    pub settings: VisualsSettings,
    pub scroll_y: f32,
}

impl Default for VisualsUi {
    fn default() -> Self {
        Self {
            open: false,
            width: 175.0,
            settings: VisualsSettings::default(),
            scroll_y: 0.0,
        }
    }
}

impl VisualsUi {
    pub fn new() -> Self {
        Self::default()
    }

    /// Render framing guides (Rule of Thirds, Center Guide, Aspect Ratio matte bars).
    pub fn render_guides(&self, batch: &mut HudBatch, atlas: &HudAtlas) {
        let (w, h) = (batch.width, batch.height);

        // 1. Aspect Ratio Matte Bars (Letterbox / Pillarbox)
        if let Some(target) = self.settings.aspect_ratio.target_ratio() {
            let current = w / h;
            let matte_color = 0xFF00_0000;
            if current > target {
                // Pillarbox: black bars on left and right
                let active_w = (h * target).round();
                let bar_w = ((w - active_w) * 0.5).round();
                style::fill(batch, atlas, [0.0, 0.0, bar_w, h], matte_color);
                style::fill(batch, atlas, [w - bar_w, 0.0, bar_w, h], matte_color);
            } else if current < target {
                // Letterbox: black bars on top and bottom
                let active_h = (w / target).round();
                let bar_h = ((h - active_h) * 0.5).round();
                style::fill(batch, atlas, [0.0, 0.0, w, bar_h], matte_color);
                style::fill(batch, atlas, [0.0, h - bar_h, w, bar_h], matte_color);
            }
        }

        // 2. Rule of Thirds Guide (2 vertical lines, 2 horizontal lines)
        if self.settings.guide_rule_of_thirds {
            let guide_color = 0x66FF_FFFF; // Faint white lines
            let x1 = (w / 3.0).round();
            let x2 = ((2.0 * w) / 3.0).round();
            let y1 = (h / 3.0).round();
            let y2 = ((2.0 * h) / 3.0).round();

            style::fill(batch, atlas, [x1, 0.0, 1.0, h], guide_color);
            style::fill(batch, atlas, [x2, 0.0, 1.0, h], guide_color);
            style::fill(batch, atlas, [0.0, y1, w, 1.0], guide_color);
            style::fill(batch, atlas, [0.0, y2, w, 1.0], guide_color);
        }

        // 3. Center Crosshair Guide
        if self.settings.guide_center {
            let center_color = 0x88FF_FFFF;
            let cx = (w * 0.5).round();
            let cy = (h * 0.5).round();
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

    /// Render the sidebar panel on the right side of the screen.
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

        let screen_w = batch.width;
        let screen_h = batch.height;
        let panel_h = (screen_h - bottom_offset).max(100.0);
        let panel_x = screen_w - self.width;
        let rect = [panel_x, 0.0, self.width, panel_h];

        // Background panel
        style::panel(batch, atlas, rect);

        // Header
        let head_h = 22.0;
        style::fill(
            batch,
            atlas,
            [panel_x, 0.0, self.width, head_h],
            style::GROUP,
        );
        style::fill(
            batch,
            atlas,
            [panel_x, head_h - 1.0, self.width, 1.0],
            style::HEAD_RULE,
        );

        batch.text_styled(
            atlas,
            &Text::translate("horizon.studio.visuals.title", &[]),
            panel_x + 8.0,
            5.0,
            style::WHITE,
            true,
        );

        // Close button [X]
        let close_rect = [panel_x + self.width - 18.0, 3.0, 15.0, 15.0];
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
            &Text::literal("×"),
            (close_hover, true, false),
        );

        // Rows
        let mut y = head_h + 6.0;
        let right = panel_x + self.width - 8.0;

        // World controls with active rendering behavior.
        self.render_section_header(
            batch,
            atlas,
            panel_x,
            &mut y,
            "horizon.studio.visuals.section.world",
        );
        self.render_toggle_row(
            batch,
            atlas,
            panel_x,
            &mut y,
            right,
            "horizon.studio.visuals.particles",
            self.settings.world_particles,
        );
        self.render_toggle_row(
            batch,
            atlas,
            panel_x,
            &mut y,
            right,
            "horizon.studio.visuals.players",
            self.settings.world_players,
        );

        self.render_section_header(
            batch,
            atlas,
            panel_x,
            &mut y,
            "horizon.studio.visuals.section.overrides",
        );
        self.render_toggle_row(
            batch,
            atlas,
            panel_x,
            &mut y,
            right,
            "horizon.studio.visuals.fov",
            self.settings.override_fov,
        );
        // Group 4: Guides & Other
        self.render_section_header(
            batch,
            atlas,
            panel_x,
            &mut y,
            "horizon.studio.visuals.section.guides_other",
        );
        self.render_toggle_row(
            batch,
            atlas,
            panel_x,
            &mut y,
            right,
            "horizon.studio.visuals.rule_of_thirds",
            self.settings.guide_rule_of_thirds,
        );
        self.render_toggle_row(
            batch,
            atlas,
            panel_x,
            &mut y,
            right,
            "horizon.studio.visuals.center_guide",
            self.settings.guide_center,
        );
        self.render_toggle_row(
            batch,
            atlas,
            panel_x,
            &mut y,
            right,
            "horizon.studio.visuals.camera_path_3d",
            self.settings.camera_path_3d,
        );

        // Aspect Ratio Row
        let aspect_rect = [right - 50.0, y + 1.0, 50.0, 14.0];
        let aspect_hover = ui::hovered(
            cursor,
            aspect_rect[0],
            aspect_rect[1],
            aspect_rect[2],
            aspect_rect[3],
        );
        batch.text_styled(
            atlas,
            &Text::translate("horizon.studio.visuals.aspect_ratio", &[]),
            panel_x + 8.0,
            y + 2.0,
            style::MUTED,
            true,
        );
        style::button(
            batch,
            atlas,
            aspect_rect,
            &Text::translate(self.settings.aspect_ratio.translation_key(), &[]),
            (aspect_hover, true, false),
        );
    }

    fn render_section_header(
        &self,
        batch: &mut HudBatch,
        atlas: &HudAtlas,
        panel_x: f32,
        y: &mut f32,
        title: &str,
    ) {
        *y += 4.0;
        style::fill(
            batch,
            atlas,
            [panel_x + 6.0, *y + 11.0, self.width - 12.0, 1.0],
            style::ROW_RULE,
        );
        batch.text_styled(
            atlas,
            &Text::translate(title, &[]),
            panel_x + 8.0,
            *y,
            style::SOFT,
            true,
        );
        *y += 14.0;
    }

    fn render_toggle_row(
        &self,
        batch: &mut HudBatch,
        atlas: &HudAtlas,
        panel_x: f32,
        y: &mut f32,
        right: f32,
        label: &str,
        value: bool,
    ) {
        batch.text_styled(
            atlas,
            &Text::translate(label, &[]),
            panel_x + 10.0,
            *y + 2.0,
            style::WHITE,
            true,
        );
        style::checkbox(batch, atlas, right, *y, value, true);
        *y += 17.0;
    }

    /// Handle mouse click inside sidebar. Returns true if handled.
    pub fn handle_mouse_click(
        &mut self,
        cursor: (f32, f32),
        screen_w: f32,
        screen_h: f32,
        bottom_offset: f32,
    ) -> bool {
        if !self.open {
            return false;
        }

        let panel_h = (screen_h - bottom_offset).max(100.0);
        let panel_x = screen_w - self.width;
        if cursor.0 < panel_x || cursor.1 < 0.0 || cursor.1 > panel_h {
            return false;
        }

        // Close button
        let close_rect = [panel_x + self.width - 18.0, 3.0, 15.0, 15.0];
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

        let right = panel_x + self.width - 8.0;
        let mut y = 22.0 + 6.0;

        // World controls: particles then player models.
        y += 18.0;
        if Self::check_toggle_click(cursor, right, &mut y, &mut self.settings.world_particles) {
            return true;
        }
        if Self::check_toggle_click(cursor, right, &mut y, &mut self.settings.world_players) {
            return true;
        }
        y += 18.0;
        if Self::check_toggle_click(cursor, right, &mut y, &mut self.settings.override_fov) {
            return true;
        }
        // Guides & other working editor options.
        y += 18.0;
        if Self::check_toggle_click(
            cursor,
            right,
            &mut y,
            &mut self.settings.guide_rule_of_thirds,
        ) {
            return true;
        }
        if Self::check_toggle_click(cursor, right, &mut y, &mut self.settings.guide_center) {
            return true;
        }
        if Self::check_toggle_click(cursor, right, &mut y, &mut self.settings.camera_path_3d) {
            return true;
        }

        // Aspect Ratio button
        let aspect_rect = [right - 50.0, y + 1.0, 50.0, 14.0];
        if ui::hovered(
            cursor,
            aspect_rect[0],
            aspect_rect[1],
            aspect_rect[2],
            aspect_rect[3],
        ) {
            self.settings.aspect_ratio = self.settings.aspect_ratio.cycle();
            return true;
        }

        true
    }

    fn check_toggle_click(cursor: (f32, f32), right: f32, y: &mut f32, val: &mut bool) -> bool {
        let rect = [right - 14.0, *y + 2.0, 16.0, 16.0];
        let clicked = ui::hovered(cursor, rect[0], rect[1], rect[2], rect[3]);
        if clicked {
            *val = !*val;
        }
        *y += 17.0;
        clicked
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_visuals_settings_defaults() {
        let v = VisualsSettings::default();
        assert!(v.world_particles);
        assert!(!v.override_fov);
        assert!(v.camera_path_3d);
        assert_eq!(v.aspect_ratio, AspectRatio::None);
    }

    #[test]
    fn test_aspect_ratio_cycling() {
        let mut ar = AspectRatio::None;
        ar = ar.cycle();
        assert_eq!(ar, AspectRatio::Ratio16_9);
        ar = ar.cycle();
        assert_eq!(ar, AspectRatio::Ratio4_3);
    }
}
