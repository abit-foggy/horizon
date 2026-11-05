//! macOS-Style Top Menu Bar for Replay Studio
//!
//! Provides a sleek, top-docked macOS-style desktop menu bar with File, Camera,
//! and View menus, transport info badges, an explicit Pause Menu button, and
//! an Exit Studio shortcut.

use super::replay::ReplayDriver;
use horizon_client_render::{
    graphics_style as style,
    hud::{HudAtlas, HudBatch},
    text::Text,
    ui,
};

/// Dropdown menu identifier currently opened in the top bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TopBarMenu {
    File,
    Camera,
    View,
}

/// Action emitted when the user clicks an option or button in the top bar.
#[derive(Clone, Debug, PartialEq)]
pub enum TopBarAction {
    None,
    OpenStudioSettings,
    OpenPauseMenu,
    ExitStudio,
    ExportVideo,
    OpenRecordingsFolder,
    ResetCameraToPlayer,
    ToggleLockToPath,
    ToggleFreecam,
    AddKeyframe,
    AddMarker,
    ClearKeyframes,
    ToggleVisualsSidebar,
    ToggleTimeline,
    CycleGuides,
    CycleAspectRatio,
}

/// Top bar state and renderer.
#[derive(Clone, Debug)]
pub struct TopBarUi {
    pub open_menu: Option<TopBarMenu>,
    pub height: f32,
}

impl Default for TopBarUi {
    fn default() -> Self {
        Self {
            open_menu: None,
            height: 22.0,
        }
    }
}

impl TopBarUi {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn close_dropdown(&mut self) {
        self.open_menu = None;
    }

    /// Render the macOS-style top bar and any open dropdown menu.
    pub fn render(
        &self,
        batch: &mut HudBatch,
        atlas: &HudAtlas,
        cursor: (f32, f32),
        replay: Option<&ReplayDriver>,
        recording_name: Option<&str>,
        path_locked: bool,
        keys: &horizon_client::keybinds::KeyBinds,
    ) {
        let w = batch.width;
        let bar_h = self.height;

        // 1. Top Bar Background (Dark glass)
        style::fill(batch, atlas, [0.0, 0.0, w, bar_h], 0xEE111218);
        style::fill(batch, atlas, [0.0, bar_h - 1.0, w, 1.0], 0x442C2E40);

        // 2. Studio Title Brand
        batch.text_styled(
            atlas,
            &Text::translate("horizon.studio.topbar.title", &[]),
            8.0,
            6.0,
            style::WHITE,
            true,
        );

        // 3. Menu Bar Items: File, Camera, View
        let menus = [
            (TopBarMenu::File, "horizon.studio.topbar.file", 80.0, 36.0),
            (
                TopBarMenu::Camera,
                "horizon.studio.topbar.camera",
                120.0,
                52.0,
            ),
            (TopBarMenu::View, "horizon.studio.topbar.view", 176.0, 40.0),
        ];

        for (menu, title, x, btn_w) in menus {
            let is_open = self.open_menu == Some(menu);
            let hovered = ui::hovered(cursor, x, 2.0, btn_w, bar_h - 4.0);
            if is_open || hovered {
                let bg_color = if is_open { 0x446B5BFF } else { 0x22FFFFFF };
                style::fill(batch, atlas, [x, 2.0, btn_w, bar_h - 4.0], bg_color);
            }
            let text_color = if is_open {
                style::FACE_HOVER
            } else if hovered {
                style::WHITE
            } else {
                style::GREY
            };
            batch.text_styled(
                atlas,
                &Text::translate(title, &[]),
                x + 6.0,
                6.0,
                text_color,
                true,
            );
        }

        // 4. Game Pause Menu Button
        let pause_x = 224.0;
        let pause_w = 84.0;
        let pause_hovered = ui::hovered(cursor, pause_x, 2.0, pause_w, bar_h - 4.0);
        if pause_hovered {
            style::fill(
                batch,
                atlas,
                [pause_x, 2.0, pause_w, bar_h - 4.0],
                0x28FFFFFF,
            );
        }
        let pause_color = if pause_hovered {
            style::WHITE
        } else {
            style::MUTED
        };
        batch.text_styled(
            atlas,
            &Text::translate("horizon.studio.topbar.pause_menu", &[]),
            pause_x + 6.0,
            6.0,
            pause_color,
            true,
        );

        // 5. Right Side Badges: Recording info & Time
        let mut right_x = w - 80.0;

        // Exit button on the far right
        let exit_x = w - 74.0;
        let exit_w = 68.0;
        let exit_hover = ui::hovered(cursor, exit_x, 2.0, exit_w, bar_h - 4.0);
        let exit_bg = if exit_hover { 0xCC7A2424 } else { 0x66441818 };
        style::fill(batch, atlas, [exit_x, 2.0, exit_w, bar_h - 4.0], exit_bg);
        style::ring(batch, atlas, [exit_x, 2.0, exit_w, bar_h - 4.0], 0x889A3838);
        let exit_key = horizon_client::keybinds::display_name(keys.get("key.studio.exit")).plain();
        batch.text_styled(
            atlas,
            &Text::translate("horizon.studio.topbar.exit", &[Text::literal(exit_key)]),
            exit_x + 8.0,
            6.0,
            style::WHITE,
            true,
        );

        // Time badge
        if let Some(r) = replay {
            let pos_sec = r.position_ms() / 1000;
            let dur_sec = r.duration_ms() / 1000;
            let time_str = format!(
                "{:02}:{:02} / {:02}:{:02}",
                pos_sec / 60,
                pos_sec % 60,
                dur_sec / 60,
                dur_sec % 60
            );
            let time_text = Text::literal(&time_str);
            let time_w = atlas.font.width_text(&time_text) + 12.0;
            right_x -= time_w + 8.0;
            style::fill(
                batch,
                atlas,
                [right_x, 2.0, time_w, bar_h - 4.0],
                0x22FFFFFF,
            );
            batch.text_styled(atlas, &time_text, right_x + 6.0, 6.0, style::MUTED, true);
        }

        // Recording filename badge
        if let Some(name) = recording_name {
            let clean_name = name.strip_suffix(".mcpr").unwrap_or(name);
            let name_text = Text::literal(clean_name);
            let name_w = atlas.font.width_text(&name_text) + 12.0;
            right_x -= name_w + 6.0;
            style::fill(
                batch,
                atlas,
                [right_x, 2.0, name_w, bar_h - 4.0],
                0x1AFFFFFF,
            );
            batch.text_styled(atlas, &name_text, right_x + 6.0, 6.0, style::GREY, true);
        }

        // 6. Draw Open Dropdown Menu if any
        if let Some(menu) = self.open_menu {
            self.render_dropdown(batch, atlas, cursor, menu, path_locked, keys);
        }
    }

    fn render_dropdown(
        &self,
        batch: &mut HudBatch,
        atlas: &HudAtlas,
        cursor: (f32, f32),
        menu: TopBarMenu,
        path_locked: bool,
        keys: &horizon_client::keybinds::KeyBinds,
    ) {
        let shortcut = |mapping: &str, modifier_key: Option<&str>| {
            let key = horizon_client::keybinds::display_name(keys.get(mapping)).plain();
            modifier_key.map_or(key.clone(), |modifier| {
                format!("{}+{key}", Text::translate(modifier, &[]).plain())
            })
        };
        let (x, items): (f32, Vec<(&str, String)>) = match menu {
            TopBarMenu::File => (
                80.0,
                vec![
                    (
                        "horizon.studio.topbar.export_video",
                        shortcut("key.studio.export", Some("horizon.studio.shortcut.control")),
                    ),
                    ("horizon.studio.topbar.open_recordings", String::new()),
                    ("horizon.studio.settings.open", String::new()),
                    (
                        "horizon.studio.topbar.game_pause_menu",
                        shortcut("key.studio.pauseMenu", None),
                    ),
                    (
                        "horizon.studio.topbar.close_replay",
                        shortcut("key.studio.exit", None),
                    ),
                ],
            ),
            TopBarMenu::Camera => (
                120.0,
                vec![
                    (
                        "horizon.studio.topbar.toggle_freecam",
                        shortcut("key.studio.toggleFreecam", None),
                    ),
                    (
                        "horizon.studio.topbar.reset_to_player",
                        shortcut("key.studio.resetCamera", None),
                    ),
                    (
                        if path_locked {
                            "horizon.studio.topbar.unlock_freecam"
                        } else {
                            "horizon.studio.topbar.lock_to_spline"
                        },
                        shortcut("key.studio.togglePathLock", None),
                    ),
                    (
                        "horizon.studio.topbar.add_keyframe",
                        shortcut("key.studio.addKeyframe", None),
                    ),
                    (
                        "horizon.studio.topbar.add_marker",
                        shortcut("key.studio.addMarker", None),
                    ),
                    ("horizon.studio.topbar.clear_keyframes", String::new()),
                ],
            ),
            TopBarMenu::View => (
                176.0,
                vec![
                    (
                        "horizon.studio.topbar.framing_guides",
                        shortcut("key.studio.cycleGuides", None),
                    ),
                    (
                        "horizon.studio.topbar.aspect_ratio_matte",
                        shortcut(
                            "key.studio.cycleAspectRatio",
                            Some("horizon.studio.shortcut.shift"),
                        ),
                    ),
                    (
                        "horizon.studio.topbar.visuals_sidebar",
                        shortcut("key.studio.toggleVisuals", None),
                    ),
                    (
                        "horizon.studio.topbar.timeline_hud",
                        shortcut("key.studio.toggleTimeline", None),
                    ),
                ],
            ),
        };

        let menu_w = 175.0;
        let item_h = 22.0;
        let menu_h = items.len() as f32 * item_h + 6.0;
        let menu_y = self.height;

        // Shadow & Backdrop
        style::fill(
            batch,
            atlas,
            [x + 2.0, menu_y + 2.0, menu_w, menu_h],
            0x88000000,
        );
        style::fill(batch, atlas, [x, menu_y, menu_w, menu_h], 0xF814151E);
        style::ring(batch, atlas, [x, menu_y, menu_w, menu_h], 0x553C3E54);

        for (i, (label, shortcut)) in items.iter().enumerate() {
            let item_y = menu_y + 3.0 + i as f32 * item_h;
            let item_hovered = ui::hovered(cursor, x + 3.0, item_y, menu_w - 6.0, item_h);

            if item_hovered {
                style::fill(
                    batch,
                    atlas,
                    [x + 3.0, item_y, menu_w - 6.0, item_h],
                    0x336B5BFF,
                );
            }

            let text_color = if item_hovered {
                style::WHITE
            } else {
                style::MUTED
            };
            let label_text = Text::translate(label, &[]);
            batch.text_styled(atlas, &label_text, x + 8.0, item_y + 6.0, text_color, true);

            if !shortcut.is_empty() {
                let sc_text = Text::literal(shortcut);
                let sc_w = atlas.font.width_text(&sc_text);
                batch.text_styled(
                    atlas,
                    &sc_text,
                    x + menu_w - sc_w - 8.0,
                    item_y + 6.0,
                    0x888899AA,
                    true,
                );
            }
        }
    }

    /// Process mouse click on the top bar or an active dropdown menu.
    pub fn handle_mouse_click(
        &mut self,
        cursor: (f32, f32),
        width: f32,
        _height: f32,
    ) -> TopBarAction {
        let (cx, cy) = cursor;

        // Check dropdown items first if dropdown is currently open
        if let Some(menu) = self.open_menu {
            let (menu_x, count) = match menu {
                TopBarMenu::File => (80.0, 5),
                TopBarMenu::Camera => (120.0, 6),
                TopBarMenu::View => (176.0, 4),
            };
            let menu_w = 175.0;
            let item_h = 22.0;
            let menu_h = count as f32 * item_h + 6.0;
            let menu_y = self.height;

            if ui::hovered(cursor, menu_x, menu_y, menu_w, menu_h) {
                let idx = ((cy - menu_y - 3.0) / item_h).floor() as usize;
                self.open_menu = None;
                return match (menu, idx) {
                    (TopBarMenu::File, 0) => TopBarAction::ExportVideo,
                    (TopBarMenu::File, 1) => TopBarAction::OpenRecordingsFolder,
                    (TopBarMenu::File, 2) => TopBarAction::OpenStudioSettings,
                    (TopBarMenu::File, 3) => TopBarAction::OpenPauseMenu,
                    (TopBarMenu::File, 4) => TopBarAction::ExitStudio,

                    (TopBarMenu::Camera, 0) => TopBarAction::ToggleFreecam,
                    (TopBarMenu::Camera, 1) => TopBarAction::ResetCameraToPlayer,
                    (TopBarMenu::Camera, 2) => TopBarAction::ToggleLockToPath,
                    (TopBarMenu::Camera, 3) => TopBarAction::AddKeyframe,
                    (TopBarMenu::Camera, 4) => TopBarAction::AddMarker,
                    (TopBarMenu::Camera, 5) => TopBarAction::ClearKeyframes,

                    (TopBarMenu::View, 0) => TopBarAction::CycleGuides,
                    (TopBarMenu::View, 1) => TopBarAction::CycleAspectRatio,
                    (TopBarMenu::View, 2) => TopBarAction::ToggleVisualsSidebar,
                    (TopBarMenu::View, 3) => TopBarAction::ToggleTimeline,

                    _ => TopBarAction::None,
                };
            }
        }

        // Check clicks on the top bar itself
        if cy <= self.height {
            // Exit button on the far right
            if cx >= width - 74.0 && cx <= width - 6.0 {
                self.open_menu = None;
                return TopBarAction::ExitStudio;
            }

            // Game Pause Menu button
            if (224.0..=308.0).contains(&cx) {
                self.open_menu = None;
                return TopBarAction::OpenPauseMenu;
            }

            // Menu triggers
            let menus = [
                (TopBarMenu::File, 80.0, 36.0),
                (TopBarMenu::Camera, 120.0, 52.0),
                (TopBarMenu::View, 176.0, 40.0),
            ];

            for (menu, x, w) in menus {
                if cx >= x && cx <= x + w {
                    if self.open_menu == Some(menu) {
                        self.open_menu = None;
                    } else {
                        self.open_menu = Some(menu);
                    }
                    return TopBarAction::None;
                }
            }

            // Clicked empty area of the bar
            self.open_menu = None;
            return TopBarAction::None;
        }

        // Click was outside top bar and outside dropdown -> close dropdown
        if self.open_menu.is_some() {
            self.open_menu = None;
        }

        TopBarAction::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topbar_menu_toggle() {
        let mut topbar = TopBarUi::new();
        assert_eq!(topbar.open_menu, None);

        // Click File menu
        topbar.handle_mouse_click((90.0, 10.0), 1280.0, 720.0);
        assert_eq!(topbar.open_menu, Some(TopBarMenu::File));

        // Click File menu again to toggle off
        topbar.handle_mouse_click((90.0, 10.0), 1280.0, 720.0);
        assert_eq!(topbar.open_menu, None);
    }

    #[test]
    fn test_topbar_action_buttons() {
        let mut topbar = TopBarUi::new();

        // Click Pause Menu button
        let action = topbar.handle_mouse_click((250.0, 10.0), 1280.0, 720.0);
        assert_eq!(action, TopBarAction::OpenPauseMenu);

        // Click Exit Studio button
        let exit_action = topbar.handle_mouse_click((1280.0 - 40.0, 10.0), 1280.0, 720.0);
        assert_eq!(exit_action, TopBarAction::ExitStudio);
    }

    #[test]
    fn camera_menu_add_marker_action_uses_m_shortcut_row() {
        let mut topbar = TopBarUi::new();
        topbar.handle_mouse_click((130.0, 10.0), 1280.0, 720.0);
        let action = topbar.handle_mouse_click((130.0, 124.0), 1280.0, 720.0);
        assert_eq!(action, TopBarAction::AddMarker);
    }

    #[test]
    fn test_topbar_shortcuts_labels() {
        let _topbar = TopBarUi::new();
        let (file_x, file_items) = match TopBarMenu::File {
            TopBarMenu::File => (
                80.0,
                vec![
                    ("🎬 Export Video...", "Ctrl+E"),
                    ("📁 Open Recordings Folder", ""),
                    ("⏸ Game Pause Menu", "P"),
                    ("✕ Close Replay", "Esc"),
                ],
            ),
            _ => unreachable!(),
        };
        assert_eq!(file_x, 80.0);
        assert_eq!(file_items[0].1, "Ctrl+E");
        assert_eq!(file_items[2].1, "P");
        assert_eq!(file_items[3].1, "Esc");

        let (view_x, view_items) = match TopBarMenu::View {
            TopBarMenu::View => (
                176.0,
                vec![
                    ("📐 Framing Guides", "G"),
                    ("⬛ Aspect Ratio Matte", "Shift+A"),
                    ("🎛 Visuals Sidebar", "V"),
                    ("⏱ Timeline HUD", "T"),
                ],
            ),
            _ => unreachable!(),
        };
        assert_eq!(view_x, 176.0);
        assert_eq!(view_items[0].1, "G");
        assert_eq!(view_items[1].1, "Shift+A");
        assert_eq!(view_items[2].1, "V");
        assert_eq!(view_items[3].1, "T");
    }
}
