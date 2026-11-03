//! Timeline HUD Overlay
//!
//! Multi-track keyframe sequencer and transport controller docked to the bottom
//! of the screen. Supports scrubbing, zooming, multi-track keyframe insertion,
//! selection, deletion, and transport controls.

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
    Camera,
    Fov,
    Timelapse,
    Shake,
}

impl TimelineTrackKind {
    pub fn name(self) -> &'static str {
        match self {
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
            active_tracks: vec![
                TimelineTrackKind::Camera,
                TimelineTrackKind::Fov,
                TimelineTrackKind::Timelapse,
            ],
        }
    }
}

impl TimelineUi {
    pub fn new() -> Self {
        Self::default()
    }

    /// Total timeline height in GUI pixels.
    pub fn height(&self) -> f32 {
        // Header: 24px, Ruler: 18px, Tracks: ~20px each + spacing
        42.0 + (self.active_tracks.len() as f32 * 22.0) + 8.0
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

        // 2. Transport Header Bar
        let header_y = panel_y + 3.0;
        self.render_header(batch, atlas, cursor, freecam, header_y, screen_w);

        // 3. Time Ruler Bar
        let ruler_y = panel_y + 24.0;
        let track_start_x = 90.0;
        let track_w = screen_w - track_start_x - 10.0;
        self.render_ruler(batch, atlas, cursor, ruler_y, track_start_x, track_w);

        // 4. Tracks (Camera, FOV, Timelapse, etc.)
        let mut curr_y = ruler_y + 18.0;
        for track in &self.active_tracks {
            self.render_track_row(
                batch,
                atlas,
                cursor,
                freecam,
                *track,
                curr_y,
                track_start_x,
                track_w,
            );
            curr_y += 22.0;
        }

        // 5. Draw Playhead Vertical Indicator
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

    fn render_track_row(
        &self,
        batch: &mut HudBatch,
        atlas: &HudAtlas,
        cursor: (f32, f32),
        freecam: &StudioFreecam,
        kind: TimelineTrackKind,
        y: f32,
        start_x: f32,
        track_w: f32,
    ) {
        // Track Header (left column: label and [+] [-] [o] buttons)
        let label_w = 44.0;
        batch.text_styled(
            atlas,
            &Text::literal(kind.name()),
            6.0,
            y + 4.0,
            style::WHITE,
            true,
        );

        let btn_sz = 12.0;
        let mut bx = 6.0 + label_w;

        // [+] Add Keyframe
        let add_rect = [bx, y + 3.0, btn_sz, btn_sz];
        let add_hover = ui::hovered(cursor, bx, y + 3.0, btn_sz, btn_sz);
        style::button(
            batch,
            atlas,
            add_rect,
            &Text::literal("+"),
            (add_hover, true, false),
        );
        bx += btn_sz + 2.0;

        // [-] Remove Keyframe
        let sub_rect = [bx, y + 3.0, btn_sz, btn_sz];
        let sub_hover = ui::hovered(cursor, bx, y + 3.0, btn_sz, btn_sz);
        let has_sel = self.selected_keyframe.is_some();
        style::button(
            batch,
            atlas,
            sub_rect,
            &Text::literal("-"),
            (sub_hover, has_sel, false),
        );
        bx += btn_sz + 2.0;

        // [o] Jump to Keyframe
        let jump_rect = [bx, y + 3.0, btn_sz, btn_sz];
        let jump_hover = ui::hovered(cursor, bx, y + 3.0, btn_sz, btn_sz);
        style::button(
            batch,
            atlas,
            jump_rect,
            &Text::literal("o"),
            (jump_hover, true, false),
        );

        // Track Lane
        let lane_rect = [start_x, y + 2.0, track_w, 16.0];
        style::fill(batch, atlas, lane_rect, style::GROUP);
        style::ring(batch, atlas, lane_rect, style::ROW_RULE);

        // Render keyframe markers for Camera track
        if kind == TimelineTrackKind::Camera {
            let total_dur = self.duration_ms.max(1) as f32;
            for (idx, kf) in freecam.track.keyframes.iter().enumerate() {
                let norm = (kf.time_ms as f32 / total_dur).clamp(0.0, 1.0);
                let kx = start_x + norm * track_w;
                let ky = y + 5.0;

                let is_sel = self.selected_keyframe == Some(idx);
                let node_color = if is_sel { style::FACE } else { style::ACCENT };

                // Draw keyframe diamond ◆
                self.draw_diamond(batch, atlas, kx, ky, 8.0, node_color);
            }
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

        // Check [+] [-] [o] buttons on Camera track
        let ruler_y = panel_y + 24.0;
        let camera_y = ruler_y + 18.0;
        let label_w = 44.0;
        let btn_sz = 12.0;
        let mut bx = 6.0 + label_w;

        // [+] Add Keyframe
        if ui::hovered(cursor, bx, camera_y + 3.0, btn_sz, btn_sz) {
            return TimelineAction::AddCameraKeyframe;
        }
        bx += btn_sz + 2.0;

        // [-] Remove Keyframe
        if ui::hovered(cursor, bx, camera_y + 3.0, btn_sz, btn_sz)
            && let Some(sel) = self.selected_keyframe
        {
            return TimelineAction::RemoveCameraKeyframe(sel);
        }
        bx += btn_sz + 2.0;

        // [o] Jump to Keyframe
        if ui::hovered(cursor, bx, camera_y + 3.0, btn_sz, btn_sz)
            && let Some(sel) = self.selected_keyframe
            && let Some(kf) = freecam.track.keyframes.get(sel)
        {
            return TimelineAction::Seek(kf.time_ms as u64);
        }

        // Check keyframe nodes selection in Camera track
        let track_start_x = 90.0;
        let track_w = screen_w - track_start_x - 10.0;
        let total_dur = self.duration_ms.max(1) as f32;

        for (idx, kf) in freecam.track.keyframes.iter().enumerate() {
            let norm = (kf.time_ms as f32 / total_dur).clamp(0.0, 1.0);
            let kx = track_start_x + norm * track_w;
            if (cursor.0 - kx).abs() <= 6.0 && (cursor.1 - (camera_y + 9.0)).abs() <= 8.0 {
                self.selected_keyframe = Some(idx);
                self.dragging_keyframe = Some(idx);
                return TimelineAction::SelectKeyframe(idx);
            }
        }

        // Check click on Ruler or track lane to seek playhead
        if cursor.0 >= track_start_x && cursor.0 <= track_start_x + track_w {
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
    fn test_timeline_ui_defaults() {
        let ui = TimelineUi::new();
        assert!(ui.visible);
        assert_eq!(ui.current_time_ms, 0);
        assert_eq!(ui.speed, 1.0);
        assert!(ui.height() > 80.0);
    }
}
