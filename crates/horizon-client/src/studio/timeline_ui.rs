//! Timeline HUD Overlay
//!
//! Unified master sequence bar and transport controller docked to the bottom
//! of the screen. Supports scrubbing, zooming, keyframe insertion, selection,
//! deletion, and cutting/splitting the sequence between frames/scenes.

use super::camera::StudioFreecam;
use horizon_client_render::{
    graphics_style as style,
    hud::{HudAtlas, HudBatch},
    text::Text,
    ui,
};

/// Track types on the studio timeline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimelineTrackKind {
    Sequence,
    Camera,
    Fov,
    Timelapse,
    Shake,
}

impl TimelineTrackKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Sequence => "Sequence",
            Self::Camera => "Camera",
            Self::Fov => "FOV",
            Self::Timelapse => "Timelapse",
            Self::Shake => "Shake",
        }
    }
}

/// Action emitted when a user interacts with the timeline UI.
#[derive(Clone, Debug, PartialEq)]
pub enum TimelineAction {
    Seek(u64),
    TogglePlay,
    StepBack,
    StepForward,
    SeekStart,
    SeekEnd,
    SetSpeed(f64),
    ToggleFreecam,
    AddCameraKeyframe,
    RemoveCameraKeyframe(usize),
    SelectKeyframe(usize),
    SplitClip(u64),
    OpenExportDialog,
    ToggleVisualsSidebar,
    None,
}

/// In-game timeline UI state and renderer.
#[derive(Clone, Debug)]
pub struct TimelineUi {
    pub visible: bool,
    pub current_time_ms: u64,
    pub duration_ms: u64,
    pub zoom_px_per_sec: f32,
    pub scroll_offset_ms: u64,
    pub playing: bool,
    pub speed: f64,
    pub selected_keyframe: Option<usize>,
    pub dragging_playhead: bool,
    pub dragging_keyframe: Option<usize>,
    pub active_tracks: Vec<TimelineTrackKind>,
    pub split_points: Vec<u64>,
}

impl Default for TimelineUi {
    fn default() -> Self {
        Self {
            visible: true,
            current_time_ms: 0,
            duration_ms: 30_000, // 30 seconds default
            zoom_px_per_sec: 50.0,
            scroll_offset_ms: 0,
            playing: false,
            speed: 1.0,
            selected_keyframe: None,
            dragging_playhead: false,
            dragging_keyframe: None,
            active_tracks: vec![TimelineTrackKind::Sequence],
            split_points: Vec::new(),
        }
    }
}

impl TimelineUi {
    pub fn new() -> Self {
        Self::default()
    }

    /// Total timeline height in GUI pixels: Header (24px) + Ruler (18px) + Master Sequence Bar (26px) + Spacing (8px).
    pub fn height(&self) -> f32 {
        76.0
    }

    /// Split or cut the clip/sequence at `time_ms`.
    /// If within 250ms of an existing cut, toggles/removes that cut.
    pub fn split_at(&mut self, time_ms: u64) {
        if time_ms == 0 || time_ms >= self.duration_ms {
            return;
        }
        if let Some(pos) = self
            .split_points
            .iter()
            .position(|&t| (t as i64 - time_ms as i64).abs() < 250)
        {
            self.split_points.remove(pos);
        } else {
            self.split_points.push(time_ms);
            self.split_points.sort_unstable();
        }
    }

    /// Returns the index of a keyframe under `cursor`, if any.
    pub fn keyframe_at_cursor(
        &self,
        cursor: (f32, f32),
        screen_w: f32,
        screen_h: f32,
        freecam: &StudioFreecam,
    ) -> Option<usize> {
        let panel_h = self.height();
        let panel_y = screen_h - panel_h;
        let ruler_y = panel_y + 24.0;
        let bar_y = ruler_y + 18.0;
        let bar_h = 24.0;
        let track_start_x = 90.0;
        let track_w = screen_w - track_start_x - 10.0;
        let total_dur = self.duration_ms.max(1) as f32;

        for (idx, kf) in freecam.track.keyframes.iter().enumerate() {
            let norm = (kf.time_ms as f32 / total_dur).clamp(0.0, 1.0);
            let kx = track_start_x + norm * track_w;
            let ky = bar_y + (bar_h * 0.5);
            if (cursor.0 - kx).abs() <= 8.0 && (cursor.1 - ky).abs() <= 10.0 {
                return Some(idx);
            }
        }
        None
    }

    /// Render the timeline overlay onto `batch`.
    pub fn render(
        &self,
        batch: &mut HudBatch,
        atlas: &HudAtlas,
        cursor: (f32, f32),
        freecam: &StudioFreecam,
    ) {
        if !self.visible {
            return;
        }

        let screen_w = batch.width;
        let screen_h = batch.height;
        let panel_h = self.height();
        let panel_y = screen_h - panel_h;
        let panel_rect = [0.0, panel_y, screen_w, panel_h];

        // 1. Draw glass background panel
        style::panel(batch, atlas, panel_rect);

        // 2. Transport & Action Header Bar
        let header_y = panel_y + 3.0;
        self.render_header(batch, atlas, cursor, freecam, header_y, screen_w);

        // 3. Time Ruler Bar
        let ruler_y = panel_y + 24.0;
        let track_start_x = 90.0;
        let track_w = screen_w - track_start_x - 10.0;
        self.render_ruler(batch, atlas, cursor, ruler_y, track_start_x, track_w);

        // 4. Unified Big Master Sequence Bar with Scenes and Keyframes
        let bar_y = ruler_y + 18.0;
        self.render_master_sequence_bar(
            batch,
            atlas,
            cursor,
            freecam,
            bar_y,
            track_start_x,
            track_w,
        );

        // 5. Draw Playhead Vertical Scrubber
        self.render_playhead(
            batch,
            atlas,
            panel_y + 24.0,
            panel_h - 24.0,
            track_start_x,
            track_w,
        );
    }

    fn render_header(
        &self,
        batch: &mut HudBatch,
        atlas: &HudAtlas,
        cursor: (f32, f32),
        freecam: &StudioFreecam,
        y: f32,
        w: f32,
    ) {
        let mut x = 6.0;

        // Transport buttons: |<<  <<  > / ||  >>  >>|
        let btn_w = 18.0;
        let btn_h = 16.0;
        let buttons = [
            ("|<<", "Start"),
            ("<<", "-1s"),
            (if self.playing { "||" } else { ">" }, "Play/Pause"),
            (">>", "+1s"),
            (">>|", "End"),
        ];

        for (label, _hint) in buttons {
            let rect = [x, y, btn_w, btn_h];
            let hover = ui::hovered(cursor, x, y, btn_w, btn_h);
            style::button(
                batch,
                atlas,
                rect,
                &Text::literal(label),
                (hover, true, false),
            );
            x += btn_w + 2.0;
        }

        x += 6.0;

        // Time Readout: 00:00.00 / 00:30.00
        let time_str = format!(
            "{:02}:{:02}.{:02} / {:02}:{:02}.{:02}",
            self.current_time_ms / 60_000,
            (self.current_time_ms % 60_000) / 1000,
            (self.current_time_ms % 1000) / 10,
            self.duration_ms / 60_000,
            (self.duration_ms % 60_000) / 1000,
            (self.duration_ms % 1000) / 10
        );
        batch.text_styled(
            atlas,
            &Text::literal(&time_str),
            x,
            y + 4.0,
            style::WHITE,
            true,
        );
        x += 135.0;

        // Speed badge: 1.0x
        let speed_str = format!("{:.1}x", self.speed);
        let speed_rect = [x, y, 32.0, btn_h];
        let speed_hover = ui::hovered(cursor, x, y, 32.0, btn_h);
        style::button(
            batch,
            atlas,
            speed_rect,
            &Text::literal(&speed_str),
            (speed_hover, true, false),
        );
        x += 38.0;

        // Freecam Toggle Button
        let freecam_label = if freecam.active {
            "Freecam: ON"
        } else {
            "Freecam: OFF"
        };
        let freecam_rect = [x, y, 78.0, btn_h];
        let freecam_hover = ui::hovered(cursor, x, y, 78.0, btn_h);
        style::button(
            batch,
            atlas,
            freecam_rect,
            &Text::literal(freecam_label),
            (freecam_hover, true, freecam.active),
        );
        x += 84.0;

        // Keyframe controls: [+ Key] [- Del] [Split]
        let add_w = 40.0;
        let add_hover = ui::hovered(cursor, x, y, add_w, btn_h);
        style::button(
            batch,
            atlas,
            [x, y, add_w, btn_h],
            &Text::literal("+ Key"),
            (add_hover, true, false),
        );
        x += add_w + 3.0;

        let has_sel = self.selected_keyframe.is_some();
        let del_w = 38.0;
        let del_hover = ui::hovered(cursor, x, y, del_w, btn_h);
        style::button(
            batch,
            atlas,
            [x, y, del_w, btn_h],
            &Text::literal("- Del"),
            (del_hover, has_sel, has_sel),
        );
        x += del_w + 3.0;

        let split_w = 38.0;
        let split_hover = ui::hovered(cursor, x, y, split_w, btn_h);
        style::button(
            batch,
            atlas,
            [x, y, split_w, btn_h],
            &Text::literal("Split"),
            (split_hover, true, false),
        );

        // Right side: Visuals toggle & Export button
        let export_w = 64.0;
        let export_x = w - export_w - 6.0;
        let export_hover = ui::hovered(cursor, export_x, y, export_w, btn_h);
        style::button(
            batch,
            atlas,
            [export_x, y, export_w, btn_h],
            &Text::literal("Export..."),
            (export_hover, true, true), // Primary violet accent button
        );

        let visuals_w = 60.0;
        let visuals_x = export_x - visuals_w - 4.0;
        let visuals_hover = ui::hovered(cursor, visuals_x, y, visuals_w, btn_h);
        style::button(
            batch,
            atlas,
            [visuals_x, y, visuals_w, btn_h],
            &Text::literal("Visuals"),
            (visuals_hover, true, false),
        );
    }

    fn render_ruler(
        &self,
        batch: &mut HudBatch,
        atlas: &HudAtlas,
        _cursor: (f32, f32),
        y: f32,
        start_x: f32,
        w: f32,
    ) {
        let ruler_rect = [start_x, y, w, 16.0];
        style::well(batch, atlas, ruler_rect);

        // Draw second tick marks and labels
        let sec_step = 1.0; // 1s ticks
        let total_secs = (self.duration_ms as f32 / 1000.0).max(1.0);
        let px_per_sec = w / total_secs;

        let mut sec = 0.0;
        while sec <= total_secs {
            let tx = start_x + sec * px_per_sec;
            if tx <= start_x + w {
                // Major tick line
                style::fill(batch, atlas, [tx, y + 8.0, 1.0, 8.0], style::EDGE);
                // Label every 2 or 5 seconds depending on width
                let is_labeled = if total_secs > 60.0 {
                    (sec as u32).is_multiple_of(10)
                } else if total_secs > 20.0 {
                    (sec as u32).is_multiple_of(5)
                } else {
                    (sec as u32).is_multiple_of(2)
                };

                if is_labeled {
                    let lbl = format!("{}s", sec as u32);
                    batch.text_styled(
                        atlas,
                        &Text::literal(&lbl),
                        tx + 2.0,
                        y + 1.0,
                        style::SOFT,
                        true,
                    );
                }
            }
            sec += sec_step;
        }
    }

    /// Render the unified big master sequence bar with scene blocks, splits, and keyframes.
    fn render_master_sequence_bar(
        &self,
        batch: &mut HudBatch,
        atlas: &HudAtlas,
        _cursor: (f32, f32),
        freecam: &StudioFreecam,
        y: f32,
        start_x: f32,
        track_w: f32,
    ) {
        let bar_h = 24.0;

        // Track label column
        let num_scenes = self.split_points.len() + 1;
        let track_lbl = if num_scenes > 1 {
            format!("{num_scenes} Scenes")
        } else {
            "Sequence".to_string()
        };
        batch.text_styled(
            atlas,
            &Text::literal(&track_lbl),
            6.0,
            y + 6.0,
            style::WHITE,
            true,
        );

        // Base lane well
        let lane_rect = [start_x, y, track_w, bar_h];
        style::fill(batch, atlas, lane_rect, 0xFF14_171F);
        style::ring(batch, atlas, lane_rect, style::ROW_RULE);

        let total_dur = self.duration_ms.max(1) as f32;

        // Render scene blocks between split points
        let mut segments = Vec::new();
        let mut prev_t = 0u64;
        for &split_t in &self.split_points {
            if split_t > prev_t && split_t < self.duration_ms {
                segments.push((prev_t, split_t));
                prev_t = split_t;
            }
        }
        segments.push((prev_t, self.duration_ms));

        for (idx, (t0, t1)) in segments.iter().enumerate() {
            let norm0 = (*t0 as f32 / total_dur).clamp(0.0, 1.0);
            let norm1 = (*t1 as f32 / total_dur).clamp(0.0, 1.0);
            let seg_x = start_x + norm0 * track_w;
            let seg_w = ((norm1 - norm0) * track_w).max(2.0);

            // Alternating clip background shades
            let bg_color = if idx % 2 == 0 {
                0xFF22_2735
            } else {
                0xFF2A_3040
            };
            style::fill(batch, atlas, [seg_x + 1.0, y + 1.0, seg_w - 2.0, bar_h - 2.0], bg_color);

            // Scene name badge inside clip
            if seg_w > 45.0 {
                let scene_title = format!("Scene {}", idx + 1);
                batch.text_styled(
                    atlas,
                    &Text::literal(&scene_title),
                    seg_x + 5.0,
                    y + 5.0,
                    style::MUTED,
                    true,
                );
            }

            // Split line / cut marker between scenes
            if idx > 0 {
                // Vertical cut line
                style::fill(batch, atlas, [seg_x - 1.0, y, 2.0, bar_h], 0xFF8A_98B8);
                // Top and bottom cut notch tick marks
                style::fill(batch, atlas, [seg_x - 3.0, y, 6.0, 2.0], 0xFFB4_C4E6);
                style::fill(batch, atlas, [seg_x - 3.0, y + bar_h - 2.0, 6.0, 2.0], 0xFFB4_C4E6);
            }
        }

        // Render camera keyframe diamonds directly on this big master bar
        for (idx, kf) in freecam.track.keyframes.iter().enumerate() {
            let norm = (kf.time_ms as f32 / total_dur).clamp(0.0, 1.0);
            let kx = start_x + norm * track_w;
            let ky = y + (bar_h * 0.5) - 5.0;

            let is_sel = self.selected_keyframe == Some(idx);
            let node_color = if is_sel {
                0xFFFF_C83B // Bright gold/amber selected keyframe
            } else {
                0xFF9E_7BFF // Violet accent unselected keyframe
            };

            // Selected keyframe halo
            if is_sel {
                self.draw_diamond(batch, atlas, kx, ky - 1.0, 12.0, 0x88FF_C83B);
            }

            // Draw keyframe diamond ◆
            self.draw_diamond(batch, atlas, kx, ky, 10.0, node_color);
        }
    }

    fn render_playhead(
        &self,
        batch: &mut HudBatch,
        atlas: &HudAtlas,
        y: f32,
        h: f32,
        start_x: f32,
        track_w: f32,
    ) {
        let total_dur = self.duration_ms.max(1) as f32;
        let norm = (self.current_time_ms as f32 / total_dur).clamp(0.0, 1.0);
        let px = start_x + norm * track_w;

        // Scrubber line from ruler through bottom of tracks
        style::fill(batch, atlas, [px - 1.0, y, 2.0, h], style::FACE);

        // Inverted triangle marker at ruler top
        let tri_color = style::WHITE;
        for i in 0..5 {
            let w = (5 - i) * 2 - 1;
            let ox = px - (w as f32 * 0.5);
            style::fill(batch, atlas, [ox, y + i as f32, w as f32, 1.0], tri_color);
        }
    }

    fn draw_diamond(
        &self,
        batch: &mut HudBatch,
        atlas: &HudAtlas,
        cx: f32,
        cy: f32,
        size: f32,
        color: u32,
    ) {
        let half = (size * 0.5).round();
        for i in 0..=(half as i32) {
            let w = (i * 2 + 1) as f32;
            let ox = cx - (w * 0.5).floor();
            let oy_top = cy + i as f32;
            let oy_bot = cy + (size - i as f32 - 1.0);
            style::fill(batch, atlas, [ox, oy_top, w, 1.0], color);
            if oy_bot != oy_top {
                style::fill(batch, atlas, [ox, oy_bot, w, 1.0], color);
            }
        }
    }

    /// Handle mouse click down inside the timeline bounds.
    pub fn handle_mouse_down(
        &mut self,
        cursor: (f32, f32),
        screen_w: f32,
        screen_h: f32,
        freecam: &StudioFreecam,
    ) -> TimelineAction {
        if !self.visible {
            return TimelineAction::None;
        }

        let panel_h = self.height();
        let panel_y = screen_h - panel_h;
        if cursor.1 < panel_y || cursor.1 > screen_h {
            return TimelineAction::None;
        }

        let y = panel_y + 3.0;
        let btn_w = 18.0;
        let btn_h = 16.0;

        // Transport buttons
        let mut x = 6.0;
        if ui::hovered(cursor, x, y, btn_w, btn_h) {
            return TimelineAction::SeekStart;
        }
        x += btn_w + 2.0;
        if ui::hovered(cursor, x, y, btn_w, btn_h) {
            return TimelineAction::StepBack;
        }
        x += btn_w + 2.0;
        if ui::hovered(cursor, x, y, btn_w, btn_h) {
            return TimelineAction::TogglePlay;
        }
        x += btn_w + 2.0;
        if ui::hovered(cursor, x, y, btn_w, btn_h) {
            return TimelineAction::StepForward;
        }
        x += btn_w + 2.0;
        if ui::hovered(cursor, x, y, btn_w, btn_h) {
            return TimelineAction::SeekEnd;
        }

        // Speed button
        let speed_x = 6.0 + 5.0 * (btn_w + 2.0) + 6.0 + 135.0;
        if ui::hovered(cursor, speed_x, y, 32.0, btn_h) {
            let next_speed = match (self.speed * 10.0).round() as i32 {
                2 => 0.5,
                5 => 1.0,
                10 => 2.0,
                20 => 5.0,
                _ => 1.0,
            };
            return TimelineAction::SetSpeed(next_speed);
        }

        // Freecam button
        let freecam_x = speed_x + 38.0;
        if ui::hovered(cursor, freecam_x, y, 78.0, btn_h) {
            return TimelineAction::ToggleFreecam;
        }

        // Keyframe controls: [+ Key] [- Del] [Split]
        let mut kf_x = freecam_x + 84.0;
        let add_w = 40.0;
        if ui::hovered(cursor, kf_x, y, add_w, btn_h) {
            return TimelineAction::AddCameraKeyframe;
        }
        kf_x += add_w + 3.0;

        let del_w = 38.0;
        if ui::hovered(cursor, kf_x, y, del_w, btn_h)
            && let Some(sel) = self.selected_keyframe
        {
            return TimelineAction::RemoveCameraKeyframe(sel);
        }
        kf_x += del_w + 3.0;

        let split_w = 38.0;
        if ui::hovered(cursor, kf_x, y, split_w, btn_h) {
            return TimelineAction::SplitClip(self.current_time_ms);
        }

        // Export button
        let export_w = 64.0;
        let export_x = screen_w - export_w - 6.0;
        if ui::hovered(cursor, export_x, y, export_w, btn_h) {
            return TimelineAction::OpenExportDialog;
        }

        // Visuals button
        let visuals_w = 60.0;
        let visuals_x = export_x - visuals_w - 4.0;
        if ui::hovered(cursor, visuals_x, y, visuals_w, btn_h) {
            return TimelineAction::ToggleVisualsSidebar;
        }

        // Check keyframe nodes selection in the master sequence bar
        if let Some(idx) = self.keyframe_at_cursor(cursor, screen_w, screen_h, freecam) {
            self.selected_keyframe = Some(idx);
            self.dragging_keyframe = Some(idx);
            return TimelineAction::SelectKeyframe(idx);
        }

        // Check click on Ruler or Master Sequence Bar to seek playhead
        let track_start_x = 90.0;
        let track_w = screen_w - track_start_x - 10.0;
        let total_dur = self.duration_ms.max(1) as f32;
        let ruler_y = panel_y + 24.0;

        if cursor.1 >= ruler_y
            && cursor.1 <= screen_h
            && cursor.0 >= track_start_x
            && cursor.0 <= track_start_x + track_w
        {
            let norm = ((cursor.0 - track_start_x) / track_w).clamp(0.0, 1.0);
            let seek_time = (norm * total_dur).round() as u64;
            self.current_time_ms = seek_time;
            self.dragging_playhead = true;
            return TimelineAction::Seek(seek_time);
        }

        TimelineAction::None
    }

    /// Handle mouse drag movement while dragging playhead or keyframe.
    pub fn handle_mouse_drag(
        &mut self,
        cursor: (f32, f32),
        screen_w: f32,
        freecam: &mut StudioFreecam,
    ) -> TimelineAction {
        let track_start_x = 90.0;
        let track_w = screen_w - track_start_x - 10.0;
        let total_dur = self.duration_ms.max(1) as f32;
        let norm = ((cursor.0 - track_start_x) / track_w).clamp(0.0, 1.0);
        let time_ms = (norm * total_dur).round() as u64;

        if self.dragging_playhead {
            self.current_time_ms = time_ms;
            return TimelineAction::Seek(time_ms);
        }

        if let Some(idx) = self.dragging_keyframe
            && idx < freecam.track.keyframes.len()
        {
            let target_time = time_ms.min(u32::MAX as u64) as u32;
            freecam.track.keyframes[idx].time_ms = target_time;
            // Keep keyframes sorted by time
            freecam.track.keyframes.sort_by_key(|k| k.time_ms);
            // Re-find the selected keyframe index
            if let Some(pos) = freecam
                .track
                .keyframes
                .iter()
                .position(|k| k.time_ms == target_time)
            {
                self.selected_keyframe = Some(pos);
                self.dragging_keyframe = Some(pos);
            }
            return TimelineAction::Seek(time_ms);
        }

        TimelineAction::None
    }

    /// Release mouse drag state.
    pub fn handle_mouse_up(&mut self) {
        self.dragging_playhead = false;
        self.dragging_keyframe = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeline_ui_defaults_and_split() {
        let mut ui = TimelineUi::new();
        assert!(ui.visible);
        assert_eq!(ui.current_time_ms, 0);
        assert_eq!(ui.speed, 1.0);
        assert_eq!(ui.height(), 76.0);
        assert!(ui.split_points.is_empty());

        // Split at 5000ms
        ui.split_at(5000);
        assert_eq!(ui.split_points, vec![5000]);

        // Split at 12000ms
        ui.split_at(12000);
        assert_eq!(ui.split_points, vec![5000, 12000]);

        // Splitting close to 5000ms toggles/removes it
        ui.split_at(5100);
        assert_eq!(ui.split_points, vec![12000]);
    }
}
