//! Replay Studio Browser & Saved Recordings UI
//!
//! Provides an in-game graphical browser to inspect saved `.mcpr` recordings,
//! view recording metadata (duration, packet count, timestamp, server),
//! and open the recordings directory.

use crate::{
    graphics_style as style,
    hud::{HudAtlas, HudBatch},
    lang,
    text::Text,
    ui,
};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};

/// High-level session metadata read from `metaData.json` inside `.mcpr`.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingMetadata {
    #[serde(default)]
    pub server_name: String,
    #[serde(default)]
    pub custom_server_name: Option<String>,
    #[serde(default)]
    pub singleplayer: bool,
    #[serde(default)]
    pub duration: u32,
    #[serde(default)]
    pub date: u64,
    #[serde(default)]
    pub mcversion: String,
    #[serde(default)]
    pub file_format: String,
    #[serde(default)]
    pub file_format_version: u32,
    #[serde(default)]
    pub generator: String,
    #[serde(default)]
    pub players: Vec<String>,
}

/// A saved replay item displayed in the Studio Browser list.
#[derive(Clone, Debug, PartialEq)]
pub struct SavedRecordingItem {
    pub path: PathBuf,
    pub filename: String,
    pub title: String,
    pub duration_ms: u32,
    pub date: u64,
    pub file_size: u64,
    pub server_name: String,
    pub singleplayer: bool,
}

impl SavedRecordingItem {
    /// Formatted duration string `MM:SS` or `HH:MM:SS`.
    pub fn formatted_duration(&self) -> String {
        let total_secs = self.duration_ms / 1000;
        let hours = total_secs / 3600;
        let mins = (total_secs % 3600) / 60;
        let secs = total_secs % 60;
        if hours > 0 {
            format!("{hours:02}:{mins:02}:{secs:02}")
        } else {
            format!("{mins:02}:{secs:02}")
        }
    }

    /// Formatted file size string (KB or MB).
    pub fn formatted_size(&self) -> String {
        let bytes = self.file_size;
        if bytes >= 1024 * 1024 {
            let mb = bytes as f64 / (1024.0 * 1024.0);
            format!("{mb:.1} MB")
        } else {
            let kb = bytes.div_ceil(1024);
            format!("{kb} KB")
        }
    }
}

/// User actions triggered by the Studio Browser UI.
#[derive(Clone, Debug, PartialEq)]
pub enum StudioAction {
    None,
    LoadAndEdit(PathBuf),
    OpenSettings,
    OpenFolder,
    Refresh,
    Back,
}

/// The Studio Replay Browser screen state and renderer.
#[derive(Clone, Debug)]
pub struct StudioUi {
    pub recordings: Vec<SavedRecordingItem>,
    pub selected: Option<usize>,
    pub scroll: f32,
    pub search: String,
    pub record_dir: PathBuf,
}

/// A recording's row and the gap under it, in GUI pixels.
const ROW_STEP: f32 = 40.0;
const ROW_GAP: f32 = 4.0;
/// The panel's margins above and below its rows.
const LIST_MARGINS: f32 = 10.0;

impl Default for StudioUi {
    fn default() -> Self {
        let dir = PathBuf::from("recordings");
        let mut ui = Self {
            recordings: Vec::new(),
            selected: None,
            scroll: 0.0,
            search: String::new(),
            record_dir: dir.clone(),
        };
        ui.scan(&dir);
        ui
    }
}

impl StudioUi {
    const MAX_METADATA_BYTES: u64 = 1 << 20;
    /// Create a new StudioUi for the given recordings directory.
    pub fn new(record_dir: impl Into<PathBuf>) -> Self {
        let dir = record_dir.into();
        let mut ui = Self {
            recordings: Vec::new(),
            selected: None,
            scroll: 0.0,
            search: String::new(),
            record_dir: dir.clone(),
        };
        ui.scan(&dir);
        ui
    }

    /// Re-scan the recordings folder.
    pub fn refresh(&mut self) {
        self.scan(&self.record_dir.clone());
    }

    /// Read metadata from a `.mcpr` archive without extracting the full packet stream.
    pub fn read_metadata(path: &Path) -> Option<RecordingMetadata> {
        let file = File::open(path).ok()?;
        let mut archive = zip::ZipArchive::new(file).ok()?;
        let meta_file = archive.by_name("metaData.json").ok()?;
        let mut meta_bytes = Vec::new();
        meta_file
            .take(Self::MAX_METADATA_BYTES + 1)
            .read_to_end(&mut meta_bytes)
            .ok()?;
        if meta_bytes.len() as u64 > Self::MAX_METADATA_BYTES {
            return None;
        }
        serde_json::from_slice(&meta_bytes).ok()
    }

    /// Scan `dir` for `.mcpr` files and parse their metadata.
    pub fn scan(&mut self, dir: &Path) {
        self.recordings.clear();
        self.selected = None;
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("mcpr") {
                continue;
            }

            let Ok(meta) = entry.metadata() else {
                continue;
            };
            let file_size = meta.len();
            let filename = path
                .file_name()
                .map(|f| f.to_string_lossy().into_owned())
                .unwrap_or_default();
            let title = path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| filename.clone());

            let (duration_ms, date, server_name, singleplayer) =
                if let Some(parsed) = Self::read_metadata(&path) {
                    let srv = if parsed.singleplayer {
                        lang::get("menu.singleplayer")
                    } else if !parsed.server_name.is_empty() {
                        parsed.server_name
                    } else {
                        lang::get("menu.multiplayer")
                    };
                    (parsed.duration, parsed.date, srv, parsed.singleplayer)
                } else {
                    let modified = meta
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_millis() as u64)
                        .unwrap_or(0);
                    (
                        0,
                        modified,
                        lang::get("horizon.studio.unknown_server"),
                        true,
                    )
                };

            self.recordings.push(SavedRecordingItem {
                path,
                filename,
                title,
                duration_ms,
                date,
                file_size,
                server_name,
                singleplayer,
            });
        }

        // Sort descending by date (newest recordings first)
        self.recordings.sort_by_key(|a| std::cmp::Reverse(a.date));
        if !self.recordings.is_empty() {
            self.selected = Some(0);
        }
    }

    /// Draw the Studio Browser screen.
    pub fn draw(
        &mut self,
        batch: &mut HudBatch,
        atlas: &HudAtlas,
        cursor: (f32, f32),
        in_world: bool,
    ) {
        let (w, h) = (batch.width, batch.height);
        ui::menu_background(batch, atlas, 0.0, 0.0, w, h, in_world);

        // Leave the upper-right corner clear for Studio Settings, including at
        // small GUI widths where a centered title would collide with it.
        ui::centered_text(
            batch,
            atlas,
            &Text::tr("horizon.pages.studio", &[]),
            0.0,
            12.0,
            (w - 124.0).max(1.0),
            style::WHITE,
        );

        // Subtitle strip: centered info text
        let subtitle = Text::tr("horizon.studio.saved_recordings", &[]);
        ui::centered_text(batch, atlas, &subtitle, 0.0, 26.0, w, style::GREY);

        // Main content area layout
        let panel_w = (w - 40.0).clamp(280.0, 440.0);
        let panel_x = ((w - panel_w) / 2.0).floor();
        let panel_y = 42.0;
        let panel_h = (h - 76.0).max(100.0);

        // Glass panel backdrop for recordings list
        style::panel(batch, atlas, [panel_x, panel_y, panel_w, panel_h]);

        if self.recordings.is_empty() {
            // Empty state
            let empty_text = Text::tr("horizon.studio.empty", &[]);
            let hint_text = Text::tr("horizon.studio.empty_hint", &[]);
            ui::centered_text(
                batch,
                atlas,
                &empty_text,
                panel_x,
                panel_y + panel_h / 2.0 - 16.0,
                panel_w,
                style::WHITE,
            );
            ui::centered_text(
                batch,
                atlas,
                &hint_text,
                panel_x,
                panel_y + panel_h / 2.0,
                panel_w,
                style::GREY,
            );
        } else {
            // Render scrollable cards
            let card_h = ROW_STEP - ROW_GAP;
            let row_step = ROW_STEP;
            // Rows that fit whole between the panel's margins, as the loop below draws them.
            let visible_cards = ((panel_h - LIST_MARGINS + ROW_GAP) / row_step).floor() as usize;
            let max_scroll =
                (self.recordings.len().saturating_sub(visible_cards) as f32 * row_step).max(0.0);
            self.scroll = self.scroll.clamp(0.0, max_scroll);

            let card_w = panel_w - 12.0;
            let card_x = panel_x + 6.0;
            // Whole rows: nothing here clips one that's partly scrolled off.
            let mut y = panel_y + 6.0;
            let start_idx = (self.scroll / row_step).round() as usize;

            for (idx, item) in self.recordings.iter().enumerate().skip(start_idx) {
                if y + card_h > panel_y + panel_h - 4.0 {
                    break;
                }

                let rect = [card_x, y, card_w, card_h];
                crate::widgets::record_as(&item.title, rect, true, crate::widgets::Kind::Row);
                crate::widgets::target_as(rect, crate::widgets::Kind::Row);
                let is_hovered = ui::hovered(cursor, card_x, y, card_w, card_h);
                let is_selected = self.selected == Some(idx);

                // Card background & highlight
                if is_selected {
                    style::fill(batch, atlas, rect, style::OPEN);
                } else if is_hovered {
                    style::fill(batch, atlas, rect, style::HOVER);
                } else {
                    style::well(batch, atlas, rect);
                }

                let border_color = if is_selected {
                    style::ACCENT
                } else if is_hovered {
                    style::WHITE
                } else {
                    style::ROW_RULE
                };
                style::ring(batch, atlas, rect, border_color);

                // Card Line 1: Recording Title (bold white)
                let title_text = Text::literal(&item.title);
                batch.text_styled(
                    atlas,
                    &title_text,
                    card_x + 8.0,
                    y + 6.0,
                    style::WHITE,
                    true,
                );

                // Card Line 2: Duration, Size, Server
                let meta_str = format!(
                    "{} • {} • {}",
                    item.formatted_duration(),
                    item.formatted_size(),
                    item.server_name
                );
                let meta_text = Text::literal(&meta_str);
                batch.text_styled(atlas, &meta_text, card_x + 8.0, y + 20.0, style::GREY, true);

                y += row_step;
            }

            crate::widgets::scroll_area(
                [panel_x, panel_y, panel_w, panel_h],
                self.scroll,
                max_scroll,
            );

            // Scrollbar track & thumb if scrollable
            if max_scroll > 0.0 {
                let track_x = panel_x + panel_w - 4.0;
                let track_y = panel_y + 2.0;
                let track_h = panel_h - 4.0;
                let thumb_h = ((panel_h / (panel_h + max_scroll)) * track_h).max(12.0);
                let thumb_y = track_y + (self.scroll / max_scroll) * (track_h - thumb_h);
                style::scroller(
                    batch,
                    atlas,
                    [track_x, track_y, 2.0, track_h],
                    [track_x, thumb_y, 2.0, thumb_h],
                );
            }
        }

        // Bottom Button Bar
        let btn_y = h - 28.0;
        let btn_h = 20.0;

        // Button 1: Open Folder (Left)
        let f_btn = [panel_x, btn_y, 130.0, btn_h];
        let f_hover = ui::hovered(cursor, f_btn[0], f_btn[1], f_btn[2], f_btn[3]);
        let label = Text::tr("horizon.studio.open_folder", &[]);
        crate::widgets::record(&label.plain(), f_btn, true);
        style::button(batch, atlas, f_btn, &label, (f_hover, true, false));

        // Button 2: Refresh (Center-Left)
        let r_btn = [panel_x + 136.0, btn_y, 65.0, btn_h];
        let r_hover = ui::hovered(cursor, r_btn[0], r_btn[1], r_btn[2], r_btn[3]);
        let label = Text::tr("horizon.studio.refresh", &[]);
        crate::widgets::record(&label.plain(), r_btn, true);
        style::button(batch, atlas, r_btn, &label, (r_hover, true, false));

        // Open the selected recording for playback.
        let e_btn = [panel_x + panel_w - 180.0, btn_y, 110.0, btn_h];
        let e_hover = ui::hovered(cursor, e_btn[0], e_btn[1], e_btn[2], e_btn[3]);
        crate::widgets::record(
            &Text::tr("horizon.studio.open_replay", &[]).plain(),
            e_btn,
            true,
        );
        style::button(
            batch,
            atlas,
            e_btn,
            &Text::tr("horizon.studio.open_replay", &[]),
            (e_hover, true, false),
        );

        // Shared settings stay inside Studio rather than the Horizon settings hub.
        let settings_btn = [w - 116.0, 4.0, 108.0, 22.0];
        let settings_hover = ui::hovered(
            cursor,
            settings_btn[0],
            settings_btn[1],
            settings_btn[2],
            settings_btn[3],
        );
        let label = Text::tr("horizon.studio.settings.button", &[]);
        crate::widgets::record(&label.plain(), settings_btn, true);
        style::button(
            batch,
            atlas,
            settings_btn,
            &label,
            (settings_hover, true, false),
        );

        // Button 3: Done / Back (Right)
        let d_btn = [panel_x + panel_w - 64.0, btn_y, 64.0, btn_h];
        let d_hover = ui::hovered(cursor, d_btn[0], d_btn[1], d_btn[2], d_btn[3]);
        let label = Text::tr("gui.done", &[]);
        crate::widgets::record(&label.plain(), d_btn, true);
        style::button(batch, atlas, d_btn, &label, (d_hover, true, false));
        crate::widgets::declare_action(crate::widgets::Action::Done, d_btn, true);
    }

    /// Process mouse click on the Studio Browser screen.
    pub fn click(&mut self, cursor: (f32, f32), size: (f32, f32)) -> StudioAction {
        let (w, h) = size;
        let panel_w = (w - 40.0).clamp(280.0, 440.0);
        let panel_x = ((w - panel_w) / 2.0).floor();
        let panel_y = 42.0;
        let panel_h = (h - 76.0).max(100.0);
        let btn_y = h - 28.0;
        let btn_h = 20.0;

        let settings_btn = [w - 116.0, 4.0, 108.0, 22.0];
        if ui::hovered(
            cursor,
            settings_btn[0],
            settings_btn[1],
            settings_btn[2],
            settings_btn[3],
        ) {
            return StudioAction::OpenSettings;
        }

        // Button 1: Open Folder
        let f_btn = [panel_x, btn_y, 130.0, btn_h];
        if ui::hovered(cursor, f_btn[0], f_btn[1], f_btn[2], f_btn[3]) {
            return StudioAction::OpenFolder;
        }

        // Button 2: Refresh
        let r_btn = [panel_x + 136.0, btn_y, 65.0, btn_h];
        if ui::hovered(cursor, r_btn[0], r_btn[1], r_btn[2], r_btn[3]) {
            self.refresh();
            return StudioAction::Refresh;
        }

        // Button 3: Edit Replay
        let e_btn = [panel_x + panel_w - 180.0, btn_y, 110.0, btn_h];
        if ui::hovered(cursor, e_btn[0], e_btn[1], e_btn[2], e_btn[3]) {
            if let Some(idx) = self.selected
                && let Some(item) = self.recordings.get(idx)
            {
                return StudioAction::LoadAndEdit(item.path.clone());
            } else if let Some(item) = self.recordings.first() {
                self.selected = Some(0);
                return StudioAction::LoadAndEdit(item.path.clone());
            }
        }

        // Button 4: Done
        let d_btn = [panel_x + panel_w - 64.0, btn_y, 64.0, btn_h];
        if ui::hovered(cursor, d_btn[0], d_btn[1], d_btn[2], d_btn[3]) {
            return StudioAction::Back;
        }

        // Card clicks
        let card_h = ROW_STEP - ROW_GAP;
        let row_step = ROW_STEP;
        let card_w = panel_w - 12.0;
        let card_x = panel_x + 6.0;
        let start_idx = (self.scroll / row_step).round() as usize;
        let mut y = panel_y + 6.0;

        for (idx, _) in self.recordings.iter().enumerate().skip(start_idx) {
            if y + card_h > panel_y + panel_h - 4.0 {
                break;
            }

            if ui::hovered(cursor, card_x, y, card_w, card_h) {
                self.selected = Some(idx);
                return StudioAction::None;
            }

            y += row_step;
        }

        StudioAction::None
    }

    /// Scroll list by mouse wheel delta.
    pub fn wheel(&mut self, amount: f32) {
        // A notch is a row, as a focus step needs (`gamepad::list_step`).
        self.scroll = (self.scroll - amount * ROW_STEP).max(0.0);
    }
}

/// Actions on the dedicated Studio Settings page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StudioSettingsAction {
    None,
    ToggleSounds,
    ToggleSkins,
    ToggleNames,
    ToggleModsManifest,
    ToggleResourcePack,
    Back,
}

/// Renderer for recording-related Studio settings, separate from shot visuals.
#[derive(Clone, Debug, Default)]
pub struct StudioSettingsUi {
    scroll: f32,
}

impl StudioSettingsUi {
    const ROW_H: f32 = 46.0;
    const SECTION_H: f32 = 20.0;

    fn geometry(size: (f32, f32)) -> ([f32; 4], [f32; 4], [f32; 4]) {
        let (w, h) = size;
        let panel_w = (w - 24.0).clamp(380.0, 560.0).min(w.max(1.0));
        let panel_x = ((w - panel_w) / 2.0).floor();
        let panel_y = 52.0_f32.min((h - 80.0).max(8.0));
        let panel_h = (h - panel_y - 36.0).max(120.0);
        let panel = [panel_x, panel_y, panel_w, panel_h];
        let viewport = [
            panel_x + 8.0,
            panel_y + 8.0,
            panel_w - 16.0,
            (panel_h - 16.0).max(0.0),
        ];
        let done = [panel_x + panel_w - 84.0, h - 28.0, 84.0, 20.0];
        (panel, viewport, done)
    }

    fn rows(
        settings: &crate::studio_preferences::StudioPreferences,
    ) -> [(
        Option<&'static str>,
        &'static str,
        &'static str,
        StudioSettingsAction,
        bool,
    ); 5] {
        [
            (
                Some("horizon.studio.settings.section.privacy"),
                "horizon.studio.settings.skins",
                "horizon.studio.settings.skins.description",
                StudioSettingsAction::ToggleSkins,
                settings.record_player_skins,
            ),
            (
                None,
                "horizon.studio.settings.names",
                "horizon.studio.settings.names.description",
                StudioSettingsAction::ToggleNames,
                settings.record_player_names,
            ),
            (
                Some("horizon.studio.settings.section.recording"),
                "horizon.studio.settings.sounds",
                "horizon.studio.settings.sounds.description",
                StudioSettingsAction::ToggleSounds,
                settings.record_sounds,
            ),
            (
                Some("horizon.studio.settings.section.archive"),
                "horizon.studio.settings.mods",
                "horizon.studio.settings.mods.description",
                StudioSettingsAction::ToggleModsManifest,
                settings.include_mods_manifest,
            ),
            (
                None,
                "horizon.studio.settings.resource_pack",
                "horizon.studio.settings.resource_pack.description",
                StudioSettingsAction::ToggleResourcePack,
                settings.include_resource_pack,
            ),
        ]
    }

    fn content_height() -> f32 {
        3.0 * Self::SECTION_H + 5.0 * Self::ROW_H
    }

    pub fn draw(
        &self,
        batch: &mut HudBatch,
        atlas: &HudAtlas,
        cursor: (f32, f32),
        in_world: bool,
        settings: &crate::studio_preferences::StudioPreferences,
    ) {
        let (w, h) = (batch.width, batch.height);
        ui::menu_background(batch, atlas, 0.0, 0.0, w, h, in_world);
        style::title(
            batch,
            atlas,
            &Text::translate("horizon.studio.settings.title", &[]),
        );
        ui::centered_text(
            batch,
            atlas,
            &Text::translate("horizon.studio.settings.description", &[]),
            12.0,
            24.0,
            w - 24.0,
            style::GREY,
        );
        ui::centered_text(
            batch,
            atlas,
            &Text::translate("horizon.studio.settings.description_secondary", &[]),
            12.0,
            36.0,
            w - 24.0,
            style::GREY,
        );

        let (panel, viewport, done) = Self::geometry((w, h));
        style::panel(batch, atlas, panel);

        let rows = Self::rows(settings);
        let max_scroll = (Self::content_height() - viewport[3]).max(0.0);
        let mut y = viewport[1] - self.scroll.min(max_scroll);
        let mut section = None;
        for (new_section, label, description, _action, enabled) in rows {
            if let Some(key) = new_section
                && section != Some(key)
            {
                section = Some(key);
                if y + 18.0 >= viewport[1] && y <= viewport[1] + viewport[3] {
                    style::fill(
                        batch,
                        atlas,
                        [viewport[0], y, viewport[2], 18.0],
                        style::ROW_RULE,
                    );
                    batch.text_styled(
                        atlas,
                        &Text::translate(key, &[]),
                        viewport[0] + 4.0,
                        y + 2.0,
                        style::SOFT,
                        true,
                    );
                }
                y += Self::SECTION_H;
            }
            let rect = [viewport[0], y, viewport[2], Self::ROW_H - 4.0];
            if rect[1] + rect[3] >= viewport[1] && rect[1] <= viewport[1] + viewport[3] {
                let hovered = ui::hovered(cursor, rect[0], rect[1], rect[2], rect[3]);
                let bg = if hovered { 0x2AFFFFFF } else { 0x12FFFFFF };
                let ring_color = if hovered { style::WHITE } else { 0x22FFFFFF };
                style::fill(batch, atlas, rect, bg);
                style::ring(batch, atlas, rect, ring_color);

                let checkbox_right = rect[0] + rect[2] - 12.0;
                let checkbox_y = rect[1] + 11.0;
                style::checkbox(batch, atlas, checkbox_right, checkbox_y, enabled, true);

                let max_text_w = (rect[2] - 44.0).max(50.0);
                let label_str = crate::lang::get(label);
                let fitted_label = style::fit(atlas, &label_str, max_text_w);
                batch.text_styled(
                    atlas,
                    &Text::literal(fitted_label),
                    rect[0] + 10.0,
                    rect[1] + 7.0,
                    style::WHITE,
                    true,
                );
                let hint_str = crate::lang::get(description);
                let fitted_hint = style::fit(atlas, &hint_str, max_text_w);
                batch.text_styled(
                    atlas,
                    &Text::literal(fitted_hint),
                    rect[0] + 10.0,
                    rect[1] + 24.0,
                    style::MUTED,
                    false,
                );
            }
            y += Self::ROW_H;
        }

        if max_scroll > 0.0 {
            let track = [panel[0] + panel[2] - 5.0, viewport[1], 2.0, viewport[3]];
            let thumb_h = (viewport[3] * viewport[3] / Self::content_height()).max(18.0);
            let thumb_y =
                track[1] + self.scroll.min(max_scroll) / max_scroll * (track[3] - thumb_h);
            style::fill(batch, atlas, track, 0x22FF_FFFF);
            style::fill(
                batch,
                atlas,
                [track[0] - 1.0, thumb_y, 4.0, thumb_h],
                0x88CC_CCCC,
            );
        }

        let done_hover = ui::hovered(cursor, done[0], done[1], done[2], done[3]);
        ui::action_button(
            batch,
            atlas,
            crate::widgets::Action::Done,
            done[0],
            done[1],
            done[2],
            done[3],
            &Text::translate("gui.done", &[]),
            true,
            done_hover,
        );
    }

    pub fn click(
        &self,
        cursor: (f32, f32),
        size: (f32, f32),
        settings: &crate::studio_preferences::StudioPreferences,
    ) -> StudioSettingsAction {
        let (_, viewport, done) = Self::geometry(size);
        if ui::hovered(cursor, done[0], done[1], done[2], done[3]) {
            return StudioSettingsAction::Back;
        }
        if !ui::hovered(cursor, viewport[0], viewport[1], viewport[2], viewport[3]) {
            return StudioSettingsAction::None;
        }
        let max_scroll = (Self::content_height() - viewport[3]).max(0.0);
        let mut y = viewport[1] - self.scroll.min(max_scroll);
        let mut section = None;
        for (new_section, _label, _description, action, _enabled) in Self::rows(settings) {
            if let Some(key) = new_section
                && section != Some(key)
            {
                section = Some(key);
                y += Self::SECTION_H;
            }
            let rect = [viewport[0], y, viewport[2], Self::ROW_H - 4.0];
            if rect[1] + rect[3] >= viewport[1]
                && rect[1] <= viewport[1] + viewport[3]
                && ui::hovered(cursor, rect[0], rect[1], rect[2], rect[3])
            {
                return action;
            }
            y += Self::ROW_H;
        }
        StudioSettingsAction::None
    }

    pub fn wheel(&mut self, amount: f32, size: (f32, f32), cursor: (f32, f32)) -> bool {
        let (_, viewport, _) = Self::geometry(size);
        if !ui::hovered(cursor, viewport[0], viewport[1], viewport[2], viewport[3]) {
            return false;
        }
        let max_scroll = (Self::content_height() - viewport[3]).max(0.0);
        self.scroll = (self.scroll - amount * Self::ROW_H).clamp(0.0, max_scroll);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn formatted_duration_and_size() {
        let item = SavedRecordingItem {
            path: PathBuf::from("recordings/test.mcpr"),
            filename: "test.mcpr".to_string(),
            title: "test".to_string(),
            duration_ms: 125_000,
            date: 1_700_000_000,
            file_size: 4_500_000,
            server_name: "Local World".to_string(),
            singleplayer: true,
        };
        assert_eq!(item.formatted_duration(), "02:05");
        assert_eq!(item.formatted_size(), "4.3 MB");

        let short_item = SavedRecordingItem {
            path: PathBuf::from("recordings/short.mcpr"),
            filename: "short.mcpr".to_string(),
            title: "short".to_string(),
            duration_ms: 45_000,
            date: 1_700_000_000,
            file_size: 512 * 1024,
            server_name: "Hypixel".to_string(),
            singleplayer: false,
        };
        assert_eq!(short_item.formatted_duration(), "00:45");
        assert_eq!(short_item.formatted_size(), "512 KB");
    }

    #[test]
    fn studio_settings_rows_and_done_button_have_distinct_click_targets() {
        let ui = StudioSettingsUi::default();
        let size = (800.0, 600.0);
        let settings = crate::studio_preferences::StudioPreferences::default();
        let (_, _, done) = StudioSettingsUi::geometry(size);
        for (cursor, expected) in [
            ((300.0, 100.0), StudioSettingsAction::ToggleSkins),
            ((300.0, 145.0), StudioSettingsAction::ToggleNames),
            ((300.0, 210.0), StudioSettingsAction::ToggleSounds),
            ((300.0, 275.0), StudioSettingsAction::ToggleModsManifest),
            ((300.0, 325.0), StudioSettingsAction::ToggleResourcePack),
            ((done[0] + 10.0, done[1] + 5.0), StudioSettingsAction::Back),
            ((40.0, 40.0), StudioSettingsAction::None),
        ] {
            assert_eq!(ui.click(cursor, size, &settings), expected);
        }
    }

    #[test]
    fn studio_settings_scroll_reaches_rows_that_do_not_fit_small_windows() {
        let mut ui = StudioSettingsUi::default();
        let size = (400.0, 300.0);
        let settings = crate::studio_preferences::StudioPreferences::default();
        let (_, viewport, _) = StudioSettingsUi::geometry(size);
        assert!(ui.wheel(-3.0, size, (viewport[0] + 2.0, viewport[1] + 2.0)));
        assert_eq!(
            ui.click((viewport[0] + 2.0, viewport[1] + 110.0), size, &settings),
            StudioSettingsAction::ToggleModsManifest
        );
    }

    #[test]
    fn scan_and_select_recordings() {
        let temp_dir = std::env::temp_dir().join(format!("studio_test_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);

        let rec_path = temp_dir.join("sample.mcpr");
        {
            let file = File::create(&rec_path).unwrap();
            let mut zip = zip::ZipWriter::new(file);
            let options = zip::write::SimpleFileOptions::default();
            zip.start_file("metaData.json", options).unwrap();
            let meta = RecordingMetadata {
                server_name: "Test Server".to_string(),
                duration: 65_000,
                date: 1_700_000_500,
                ..Default::default()
            };
            let json_bytes = serde_json::to_vec(&meta).unwrap();
            zip.write_all(&json_bytes).unwrap();
            zip.finish().unwrap();
        }

        let mut ui = StudioUi::new(&temp_dir);
        assert_eq!(ui.recordings.len(), 1);
        assert_eq!(ui.recordings[0].title, "sample");
        assert_eq!(ui.recordings[0].duration_ms, 65_000);
        assert_eq!(ui.recordings[0].server_name, "Test Server");
        assert_eq!(ui.selected, Some(0));

        // Clicking a card selects it for inspection without opening the editor.
        let action = ui.click((200.0, 60.0), (427.0, 720.0));
        assert_eq!(action, StudioAction::None);
        assert_eq!(ui.selected, Some(0));

        // Click the footer's Edit Replay action.
        let card_edit_action = ui.click((260.0, 702.0), (427.0, 720.0));
        assert!(matches!(card_edit_action, StudioAction::LoadAndEdit(_)));

        // Clean up
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn studio_wheel_scrolling() {
        let mut ui = StudioUi::default();
        ui.wheel(2.0);
        assert_eq!(ui.scroll, 0.0); // clamped at 0
        ui.scroll = 100.0;
        ui.wheel(1.0);
        assert_eq!(ui.scroll, 100.0 - ROW_STEP);
    }
}
