//! Controls that change the current Studio replay view.

use horizon_client_render::{
    graphics_style as style,
    hud::{HudAtlas, HudBatch},
    text::Text,
    ui,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AspectRatio {
    #[default]
    None,
    Ratio16_9,
    Ratio4_3,
    Ratio1_1,
    Ratio21_9,
}

impl AspectRatio {
    pub fn next(self) -> Self {
        match self {
            Self::None => Self::Ratio16_9,
            Self::Ratio16_9 => Self::Ratio4_3,
            Self::Ratio4_3 => Self::Ratio1_1,
            Self::Ratio1_1 => Self::Ratio21_9,
            Self::Ratio21_9 => Self::None,
        }
    }

    pub fn ratio(self) -> Option<f32> {
        match self {
            Self::None => None,
            Self::Ratio16_9 => Some(16.0 / 9.0),
            Self::Ratio4_3 => Some(4.0 / 3.0),
            Self::Ratio1_1 => Some(1.0),
            Self::Ratio21_9 => Some(21.0 / 9.0),
        }
    }

    fn label_key(self) -> &'static str {
        match self {
            Self::None => "horizon.studio.visuals.aspect.none",
            Self::Ratio16_9 => "horizon.studio.visuals.aspect.16_9",
            Self::Ratio4_3 => "horizon.studio.visuals.aspect.4_3",
            Self::Ratio1_1 => "horizon.studio.visuals.aspect.1_1",
            Self::Ratio21_9 => "horizon.studio.visuals.aspect.21_9",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct VisualsSettings {
    pub world_blocks: bool,
    pub world_entities: bool,
    pub world_players: bool,
    pub world_particles: bool,
    pub override_fov: bool,
    pub fov_degrees: f32,
    pub rule_of_thirds: bool,
    pub center_guide: bool,
    pub aspect_ratio: AspectRatio,
    pub camera_path: bool,
}

impl Default for VisualsSettings {
    fn default() -> Self {
        Self {
            world_blocks: true,
            world_entities: true,
            world_players: true,
            world_particles: true,
            override_fov: false,
            fov_degrees: 70.0,
            rule_of_thirds: false,
            center_guide: false,
            aspect_ratio: AspectRatio::None,
            camera_path: true,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct VisualsUi {
    pub open: bool,
    pub settings: VisualsSettings,
}

impl VisualsUi {
    pub fn is_hovered(&self, cursor: (f32, f32), size: (f32, f32), bottom: f32) -> bool {
        if !self.open {
            return false;
        }
        let rect = self.panel_rect(size, bottom);
        ui::hovered(cursor, rect[0], rect[1], rect[2], rect[3])
    }

    fn panel_rect(&self, size: (f32, f32), bottom: f32) -> [f32; 4] {
        let width = 184.0_f32.min(size.0.max(120.0));
        let top = 24.0_f32.min(size.1);
        [size.0 - width, top, width, (size.1 - top - bottom).max(0.0)]
    }

    pub fn render(&self, batch: &mut HudBatch, atlas: &HudAtlas, cursor: (f32, f32), bottom: f32) {
        if !self.open {
            return;
        }
        let rect = self.panel_rect((batch.width, batch.height), bottom);
        if rect[3] < 34.0 {
            return;
        }
        style::panel(batch, atlas, rect);
        style::fill(batch, atlas, [rect[0], rect[1], rect[2], 22.0], style::OPEN);
        batch.text_styled(
            atlas,
            &Text::translate("horizon.studio.visuals.title", &[]),
            rect[0] + 8.0,
            rect[1] + 5.0,
            style::WHITE,
            true,
        );
        let close = [rect[0] + rect[2] - 19.0, rect[1] + 3.0, 16.0, 16.0];
        let close_hovered = ui::hovered(cursor, close[0], close[1], close[2], close[3]);
        style::button(
            batch,
            atlas,
            close,
            &Text::translate("gui.close", &[]),
            (close_hovered, true, false),
        );

        let rows: [(&str, bool); 8] = [
            ("horizon.studio.visuals.blocks", self.settings.world_blocks),
            (
                "horizon.studio.visuals.entities",
                self.settings.world_entities,
            ),
            (
                "horizon.studio.visuals.players",
                self.settings.world_players,
            ),
            (
                "horizon.studio.visuals.particles",
                self.settings.world_particles,
            ),
            (
                "horizon.studio.visuals.rule_of_thirds",
                self.settings.rule_of_thirds,
            ),
            (
                "horizon.studio.visuals.center_guide",
                self.settings.center_guide,
            ),
            (
                "horizon.studio.visuals.camera_path",
                self.settings.camera_path,
            ),
            (
                "horizon.studio.visuals.fov_override",
                self.settings.override_fov,
            ),
        ];
        let row_h = 19.0;
        for (i, (key, enabled)) in rows.iter().enumerate() {
            let y = rect[1] + 24.0 + i as f32 * row_h;
            if y + row_h > rect[1] + rect[3] {
                break;
            }
            let hover = ui::hovered(cursor, rect[0] + 3.0, y, rect[2] - 6.0, row_h);
            if hover {
                style::fill(
                    batch,
                    atlas,
                    [rect[0] + 3.0, y, rect[2] - 6.0, row_h],
                    style::HOVER,
                );
            }
            batch.text_styled(
                atlas,
                &Text::translate(key, &[]),
                rect[0] + 8.0,
                y + 5.0,
                style::WHITE,
                true,
            );
            style::checkbox(
                batch,
                atlas,
                rect[0] + rect[2] - 8.0,
                y + 1.0,
                *enabled,
                true,
            );
        }
        let fov_y = rect[1] + 24.0 + rows.len() as f32 * row_h;
        if fov_y + row_h <= rect[1] + rect[3] {
            let hover = ui::hovered(cursor, rect[0] + 3.0, fov_y, rect[2] - 6.0, row_h);
            if hover {
                style::fill(
                    batch,
                    atlas,
                    [rect[0] + 3.0, fov_y, rect[2] - 6.0, row_h],
                    style::HOVER,
                );
            }
            batch.text_styled(
                atlas,
                &Text::translate("horizon.studio.visuals.fov", &[]),
                rect[0] + 8.0,
                fov_y + 5.0,
                if self.settings.override_fov {
                    style::WHITE
                } else {
                    style::MUTED
                },
                true,
            );
            let value = Text::literal(format!("{}°", self.settings.fov_degrees.round() as i32));
            batch.text_styled(
                atlas,
                &value,
                rect[0] + rect[2] - atlas.font.width_text(&value) - 10.0,
                fov_y + 5.0,
                style::ACCENT,
                true,
            );
        }
        let y = fov_y + row_h;
        if y + row_h <= rect[1] + rect[3] {
            let hover = ui::hovered(cursor, rect[0] + 3.0, y, rect[2] - 6.0, row_h);
            if hover {
                style::fill(
                    batch,
                    atlas,
                    [rect[0] + 3.0, y, rect[2] - 6.0, row_h],
                    style::HOVER,
                );
            }
            batch.text_styled(
                atlas,
                &Text::translate("horizon.studio.visuals.aspect", &[]),
                rect[0] + 8.0,
                y + 5.0,
                style::WHITE,
                true,
            );
            let aspect = Text::translate(self.settings.aspect_ratio.label_key(), &[]);
            batch.text_styled(
                atlas,
                &aspect,
                rect[0] + rect[2] - atlas.font.width_text(&aspect) - 10.0,
                y + 5.0,
                style::ACCENT,
                true,
            );
        }
    }

    /// Returns true if the pointer is over the panel. Clicks outside remain available to the game.
    pub fn handle_mouse_click(
        &mut self,
        cursor: (f32, f32),
        size: (f32, f32),
        bottom: f32,
    ) -> bool {
        if !self.open {
            return false;
        }
        let rect = self.panel_rect(size, bottom);
        if !ui::hovered(cursor, rect[0], rect[1], rect[2], rect[3]) {
            return false;
        }
        if cursor.1 < rect[1] + 22.0 && cursor.0 >= rect[0] + rect[2] - 22.0 {
            self.open = false;
            return true;
        }
        let index = ((cursor.1 - rect[1] - 24.0) / 19.0).floor();
        if index < 0.0 {
            return true;
        }
        match index as usize {
            0 => self.settings.world_blocks = !self.settings.world_blocks,
            1 => self.settings.world_entities = !self.settings.world_entities,
            2 => self.settings.world_players = !self.settings.world_players,
            3 => self.settings.world_particles = !self.settings.world_particles,
            4 => self.settings.rule_of_thirds = !self.settings.rule_of_thirds,
            5 => self.settings.center_guide = !self.settings.center_guide,
            6 => self.settings.camera_path = !self.settings.camera_path,
            7 => self.settings.override_fov = !self.settings.override_fov,
            8 => {
                const FOV_PRESETS: [f32; 5] = [50.0, 70.0, 90.0, 110.0, 130.0];
                let current = FOV_PRESETS
                    .iter()
                    .position(|value| (*value - self.settings.fov_degrees).abs() < 1.0)
                    .unwrap_or(1);
                self.settings.fov_degrees = FOV_PRESETS[(current + 1) % FOV_PRESETS.len()];
                self.settings.override_fov = true;
            }
            9 => self.settings.aspect_ratio = self.settings.aspect_ratio.next(),
            _ => {}
        }
        true
    }

    pub fn render_guides(&self, batch: &mut HudBatch, atlas: &HudAtlas, bottom: f32) {
        let width = batch.width;
        let height = batch.height;
        let [x, y, w, h] = self.composition_rect(width, height, bottom);
        let color = 0xAAFF_FFFF;
        if self.settings.rule_of_thirds {
            for fraction in [1.0 / 3.0, 2.0 / 3.0] {
                style::fill(batch, atlas, [x + w * fraction, y, 1.0, h], color);
                style::fill(batch, atlas, [x, y + h * fraction, w, 1.0], color);
            }
        }
        if self.settings.center_guide {
            style::fill(batch, atlas, [x + w * 0.5, y, 1.0, h], color);
            style::fill(batch, atlas, [x, y + h * 0.5, w, 1.0], color);
        }
        if self.settings.aspect_ratio != AspectRatio::None {
            style::fill(batch, atlas, [x, y, w, 1.0], color);
            style::fill(batch, atlas, [x, y + h - 1.0, w, 1.0], color);
            style::fill(batch, atlas, [x, y, 1.0, h], color);
            style::fill(batch, atlas, [x + w - 1.0, y, 1.0, h], color);
        }
    }

    fn composition_rect(&self, width: f32, height: f32, bottom: f32) -> [f32; 4] {
        let top = 24.0_f32.min(height);
        let avail_w = (width - if self.open { 184.0 } else { 0.0 }).max(1.0);
        let avail_h = (height - top - bottom).max(1.0);
        let ratio = self
            .settings
            .aspect_ratio
            .ratio()
            .unwrap_or(avail_w / avail_h);
        let (w, h) = if avail_w / avail_h > ratio {
            (avail_h * ratio, avail_h)
        } else {
            (avail_w, avail_w / ratio)
        };
        [(avail_w - w) * 0.5, top + (avail_h - h) * 0.5, w, h]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aspect_ratio_cycles_through_supported_presets() {
        let mut ratio = AspectRatio::None;
        for expected in [
            AspectRatio::Ratio16_9,
            AspectRatio::Ratio4_3,
            AspectRatio::Ratio1_1,
            AspectRatio::Ratio21_9,
            AspectRatio::None,
        ] {
            ratio = ratio.next();
            assert_eq!(ratio, expected);
        }
    }

    #[test]
    fn default_visuals_preserve_replay_rendering() {
        let settings = VisualsSettings::default();
        assert!(settings.world_blocks && settings.world_entities);
        assert!(settings.world_players && settings.world_particles);
        assert!(!settings.override_fov);
    }

    #[test]
    fn visuals_clicks_toggle_visibility_and_change_view_settings() {
        let mut ui = VisualsUi {
            open: true,
            ..Default::default()
        };
        let size = (800.0, 600.0);
        assert!(ui.handle_mouse_click((620.0, 50.0), size, 80.0));
        assert!(!ui.settings.world_blocks);
        assert!(ui.handle_mouse_click((620.0, 200.0), size, 80.0));
        assert!(ui.settings.override_fov);
        assert_eq!(ui.settings.fov_degrees, 90.0);
        assert!(ui.handle_mouse_click((620.0, 219.0), size, 80.0));
        assert_eq!(ui.settings.aspect_ratio, AspectRatio::Ratio16_9);
        assert!(!ui.handle_mouse_click((100.0, 100.0), size, 80.0));
    }
}
