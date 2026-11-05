//! Export to Video Modal UI Dialog
//!
//! Provides video export configuration (resolution, framerate, codec, container,
//! bitrate, audio recording, and tick range) styled with Horizon's dark glass
//! theme and violet primary action buttons.

use horizon_client_render::{
    graphics_style as style,
    hud::{HudAtlas, HudBatch},
    text::Text,
    ui,
};

/// Export resolution presets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportResolution {
    Res720p,
    Res1080p,
    Res1440p,
    Res4k,
}

impl ExportResolution {
    pub fn dimensions(self) -> (u32, u32) {
        match self {
            Self::Res720p => (1280, 720),
            Self::Res1080p => (1920, 1080),
            Self::Res1440p => (2560, 1440),
            Self::Res4k => (3840, 2160),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Res720p => "1280x720 (720p)",
            Self::Res1080p => "1920x1080 (1080p)",
            Self::Res1440p => "2560x1440 (1440p)",
            Self::Res4k => "3840x2160 (4K UHD)",
        }
    }

    pub fn cycle(self) -> Self {
        match self {
            Self::Res720p => Self::Res1080p,
            Self::Res1080p => Self::Res1440p,
            Self::Res1440p => Self::Res4k,
            Self::Res4k => Self::Res720p,
        }
    }
}

/// Container format.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContainerFormat {
    Mp4,
    Mkv,
}

impl ContainerFormat {
    pub fn label(self) -> &'static str {
        match self {
            Self::Mp4 => "MP4 (.mp4)",
            Self::Mkv => "Matroska (.mkv)",
        }
    }

    pub fn cycle(self) -> Self {
        match self {
            Self::Mp4 => Self::Mkv,
            Self::Mkv => Self::Mp4,
        }
    }
}

/// Video codec.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VideoCodec {
    H264,
    Hevc,
    ProRes,
    Av1,
}

impl VideoCodec {
    pub fn label(self) -> &'static str {
        match self {
            Self::H264 => "H.264 / AVC",
            Self::Hevc => "H.265 / HEVC",
            Self::ProRes => "Apple ProRes",
            Self::Av1 => "AV1",
        }
    }

    pub fn cycle(self) -> Self {
        match self {
            Self::H264 => Self::Hevc,
            Self::Hevc => Self::ProRes,
            Self::ProRes => Self::Av1,
            Self::Av1 => Self::H264,
        }
    }
}

/// Encoder backend.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EncoderBackend {
    Software,
    Hardware,
}

impl EncoderBackend {
    pub fn label(self) -> &'static str {
        match self {
            Self::Software => "Software (CPU)",
            Self::Hardware => "Hardware (GPU)",
        }
    }

    pub fn cycle(self) -> Self {
        match self {
            Self::Software => Self::Hardware,
            Self::Hardware => Self::Software,
        }
    }
}

/// Video export settings state.
#[derive(Clone, Debug, PartialEq)]
pub struct ExportSettings {
    pub resolution: ExportResolution,
    pub start_tick: u64,
    pub end_tick: u64,
    pub fps: u32,
    pub reset_rng: bool,
    pub container: ContainerFormat,
    pub codec: VideoCodec,
    pub encoder: EncoderBackend,
    pub bitrate_kbps: u32,
    pub record_audio: bool,
    pub audio_codec: &'static str,
}

impl Default for ExportSettings {
    fn default() -> Self {
        Self {
            resolution: ExportResolution::Res1080p,
            start_tick: 0,
            end_tick: 600, // 30s at 20 tps
            fps: 60,
            reset_rng: true,
            container: ContainerFormat::Mp4,
            codec: VideoCodec::H264,
            encoder: EncoderBackend::Software,
            bitrate_kbps: 18000,
            record_audio: true,
            audio_codec: "AAC",
        }
    }
}

/// Export UI modal controller.
#[derive(Clone, Debug)]
pub struct ExportUi {
    pub open: bool,
    pub settings: ExportSettings,
    pub width: f32,
    pub height: f32,
}

impl Default for ExportUi {
    fn default() -> Self {
        Self {
            open: false,
            settings: ExportSettings::default(),
            width: 320.0,
            height: 290.0,
        }
    }
}

impl ExportUi {
    pub fn new() -> Self {
        Self::default()
    }

    /// Render the modal dialog centered on screen.
    pub fn render(&self, batch: &mut HudBatch, atlas: &HudAtlas, cursor: (f32, f32)) {
        if !self.open {
            return;
        }

        let (w, h) = (batch.width, batch.height);

        // Dark modal veil over background
        style::fill(batch, atlas, [0.0, 0.0, w, h], style::VEIL);

        // Center modal box
        let x = ((w - self.width) * 0.5).round();
        let y = ((h - self.height) * 0.5).round();
        let modal_rect = [x, y, self.width, self.height];

        // Panel background + edge
        style::panel(batch, atlas, modal_rect);

        // Title bar
        let head_h = 24.0;
        style::fill(batch, atlas, [x, y, self.width, head_h], style::GROUP);
        style::fill(
            batch,
            atlas,
            [x, y + head_h - 1.0, self.width, 1.0],
            style::HEAD_RULE,
        );
        batch.text_styled(
            atlas,
            &Text::literal("Export to video"),
            x + 12.0,
            y + 6.0,
            style::WHITE,
            true,
        );

        // Close button [X]
        let close_rect = [x + self.width - 20.0, y + 4.0, 16.0, 16.0];
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
            &Text::literal("x"),
            (close_hover, true, false),
        );

        // Form fields
        let mut fy = y + head_h + 10.0;
        let right = x + self.width - 12.0;
        let ctrl_w = 140.0;
        let ctrl_h = 16.0;

        // Resolution
        self.render_row_button(
            batch,
            atlas,
            cursor,
            x,
            &mut fy,
            right,
            ctrl_w,
            ctrl_h,
            "Resolution",
            self.settings.resolution.label(),
        );

        // Start / End Tick
        let tick_str = format!("{} .. {}", self.settings.start_tick, self.settings.end_tick);
        self.render_row_button(
            batch,
            atlas,
            cursor,
            x,
            &mut fy,
            right,
            ctrl_w,
            ctrl_h,
            "Tick Range",
            &tick_str,
        );

        // Framerate
        let fps_str = format!("{} FPS", self.settings.fps);
        self.render_row_button(
            batch,
            atlas,
            cursor,
            x,
            &mut fy,
            right,
            ctrl_w,
            ctrl_h,
            "Framerate",
            &fps_str,
        );

        // Reset RNG Checkbox
        self.render_row_checkbox(
            batch,
            atlas,
            x,
            &mut fy,
            right,
            "Reset RNG",
            self.settings.reset_rng,
        );

        // Container
        self.render_row_button(
            batch,
            atlas,
            cursor,
            x,
            &mut fy,
            right,
            ctrl_w,
            ctrl_h,
            "Container",
            self.settings.container.label(),
        );

        // Video Codec
        self.render_row_button(
            batch,
            atlas,
            cursor,
            x,
            &mut fy,
            right,
            ctrl_w,
            ctrl_h,
            "Video Codec",
            self.settings.codec.label(),
        );

        // Encoder Backend
        self.render_row_button(
            batch,
            atlas,
            cursor,
            x,
            &mut fy,
            right,
            ctrl_w,
            ctrl_h,
            "Encoder",
            self.settings.encoder.label(),
        );

        // Bitrate
        let bitrate_str = format!("{} kbps", self.settings.bitrate_kbps);
        self.render_row_button(
            batch,
            atlas,
            cursor,
            x,
            &mut fy,
            right,
            ctrl_w,
            ctrl_h,
            "Bitrate",
            &bitrate_str,
        );

        // Record Audio Checkbox
        self.render_row_checkbox(
            batch,
            atlas,
            x,
            &mut fy,
            right,
            "Record Audio",
            self.settings.record_audio,
        );

        // Bottom action buttons: [Cancel]  [Add to Queue]  [Start Export]
        let btn_y = y + self.height - 28.0;

        // Cancel
        let cancel_rect = [x + 12.0, btn_y, 60.0, 18.0];
        let cancel_hover = ui::hovered(
            cursor,
            cancel_rect[0],
            cancel_rect[1],
            cancel_rect[2],
            cancel_rect[3],
        );
        style::button(
            batch,
            atlas,
            cancel_rect,
            &Text::literal("Cancel"),
            (cancel_hover, true, false),
        );

        // Queueing and export need the encoder, which is not implemented yet.
        // Show the setting panel without presenting nonfunctional actions.
        let queue_rect = [x + 78.0, btn_y, 90.0, 18.0];
        style::button(
            batch,
            atlas,
            queue_rect,
            &Text::translate("horizon.studio.export.unavailable", &[]),
            (false, false, false),
        );

        // Start Export (Primary violet face)
        let export_rect = [x + self.width - 110.0, btn_y, 98.0, 18.0];
        style::button(
            batch,
            atlas,
            export_rect,
            &Text::translate("horizon.studio.export.unavailable", &[]),
            (false, false, true),
        );
    }

    fn render_row_button(
        &self,
        batch: &mut HudBatch,
        atlas: &HudAtlas,
        cursor: (f32, f32),
        x: f32,
        fy: &mut f32,
        right: f32,
        w: f32,
        h: f32,
        label: &str,
        val: &str,
    ) {
        batch.text_styled(
            atlas,
            &Text::literal(label),
            x + 14.0,
            *fy + 2.0,
            style::WHITE,
            true,
        );
        let btn_rect = [right - w, *fy, w, h];
        let hover = ui::hovered(cursor, btn_rect[0], btn_rect[1], btn_rect[2], btn_rect[3]);
        style::button(
            batch,
            atlas,
            btn_rect,
            &Text::literal(val),
            (hover, true, false),
        );
        *fy += h + 5.0;
    }

    fn render_row_checkbox(
        &self,
        batch: &mut HudBatch,
        atlas: &HudAtlas,
        x: f32,
        fy: &mut f32,
        right: f32,
        label: &str,
        val: bool,
    ) {
        batch.text_styled(
            atlas,
            &Text::literal(label),
            x + 14.0,
            *fy + 2.0,
            style::WHITE,
            true,
        );
        style::checkbox(batch, atlas, right, *fy - 2.0, val, true);
        *fy += 21.0;
    }

    /// Handle mouse click in the export modal dialog. Returns true if handled.
    pub fn handle_mouse_click(&mut self, cursor: (f32, f32), w: f32, h: f32) -> bool {
        if !self.open {
            return false;
        }

        let x = ((w - self.width) * 0.5).round();
        let y = ((h - self.height) * 0.5).round();

        // Close button
        let close_rect = [x + self.width - 20.0, y + 4.0, 16.0, 16.0];
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

        // Cancel button
        let btn_y = y + self.height - 28.0;
        let cancel_rect = [x + 12.0, btn_y, 60.0, 18.0];
        if ui::hovered(
            cursor,
            cancel_rect[0],
            cancel_rect[1],
            cancel_rect[2],
            cancel_rect[3],
        ) {
            self.open = false;
            return true;
        }

        // Queue and export actions remain disabled until an encoder is available.
        let queue_rect = [x + 78.0, btn_y, 90.0, 18.0];
        if ui::hovered(
            cursor,
            queue_rect[0],
            queue_rect[1],
            queue_rect[2],
            queue_rect[3],
        ) {
            return true;
        }

        let export_rect = [x + self.width - 110.0, btn_y, 98.0, 18.0];
        if ui::hovered(
            cursor,
            export_rect[0],
            export_rect[1],
            export_rect[2],
            export_rect[3],
        ) {
            return true;
        }

        let head_h = 24.0;
        let mut fy = y + head_h + 10.0;
        let right = x + self.width - 12.0;
        let ctrl_w = 140.0;
        let ctrl_h = 16.0;

        // Resolution
        if ui::hovered(cursor, right - ctrl_w, fy, ctrl_w, ctrl_h) {
            self.settings.resolution = self.settings.resolution.cycle();
            return true;
        }
        fy += ctrl_h + 5.0;

        // Tick range (click cycles between 30s, 60s, full)
        if ui::hovered(cursor, right - ctrl_w, fy, ctrl_w, ctrl_h) {
            self.settings.end_tick = if self.settings.end_tick == 600 {
                1200
            } else {
                600
            };
            return true;
        }
        fy += ctrl_h + 5.0;

        // Framerate
        if ui::hovered(cursor, right - ctrl_w, fy, ctrl_w, ctrl_h) {
            self.settings.fps = match self.settings.fps {
                24 => 30,
                30 => 60,
                60 => 120,
                _ => 24,
            };
            return true;
        }
        fy += ctrl_h + 5.0;

        // Reset RNG
        let rng_box = [right - 14.0, fy + 2.0, 16.0, 16.0];
        if ui::hovered(cursor, rng_box[0], rng_box[1], rng_box[2], rng_box[3]) {
            self.settings.reset_rng = !self.settings.reset_rng;
            return true;
        }
        fy += 21.0;

        // Container
        if ui::hovered(cursor, right - ctrl_w, fy, ctrl_w, ctrl_h) {
            self.settings.container = self.settings.container.cycle();
            return true;
        }
        fy += ctrl_h + 5.0;

        // Codec
        if ui::hovered(cursor, right - ctrl_w, fy, ctrl_w, ctrl_h) {
            self.settings.codec = self.settings.codec.cycle();
            return true;
        }
        fy += ctrl_h + 5.0;

        // Encoder
        if ui::hovered(cursor, right - ctrl_w, fy, ctrl_w, ctrl_h) {
            self.settings.encoder = self.settings.encoder.cycle();
            return true;
        }
        fy += ctrl_h + 5.0;

        // Bitrate
        if ui::hovered(cursor, right - ctrl_w, fy, ctrl_w, ctrl_h) {
            self.settings.bitrate_kbps = match self.settings.bitrate_kbps {
                8000 => 18000,
                18000 => 35000,
                35000 => 50000,
                _ => 8000,
            };
            return true;
        }
        fy += ctrl_h + 5.0;

        // Record audio
        let audio_box = [right - 14.0, fy + 2.0, 16.0, 16.0];
        if ui::hovered(
            cursor,
            audio_box[0],
            audio_box[1],
            audio_box[2],
            audio_box[3],
        ) {
            self.settings.record_audio = !self.settings.record_audio;
            return true;
        }

        // Inside modal absorbs click
        ui::hovered(cursor, x, y, self.width, self.height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_settings_defaults() {
        let e = ExportSettings::default();
        assert_eq!(e.resolution, ExportResolution::Res1080p);
        assert_eq!(e.fps, 60);
        assert_eq!(e.container, ContainerFormat::Mp4);
        assert_eq!(e.codec, VideoCodec::H264);
        assert!(e.record_audio);
    }

    #[test]
    fn test_resolution_cycle() {
        let mut r = ExportResolution::Res720p;
        r = r.cycle();
        assert_eq!(r, ExportResolution::Res1080p);
        r = r.cycle();
        assert_eq!(r, ExportResolution::Res1440p);
        r = r.cycle();
        assert_eq!(r, ExportResolution::Res4k);
    }
}
