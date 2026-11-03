//! In-game Video Studio: Recording & Replay Subsystem
//!
//! Provides in-game and control-socket commands to record network packet streams
//! into standard `.mcpr` containers and drive timeline-based replay playback.

use super::Game;
use horizon_client::{
    Result,
    studio::{Metadata, ReplayDriver},
    text::Text,
};
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub const HELP: &[&str] = &[
    "studio record start [NAME]     start recording clientbound packets into .mcpr archive",
    "studio record stop             stop recording and finalize .mcpr archive",
    "studio record status           query recording status (active, elapsed, packets, bytes)",
    "studio replay open PATH        open .mcpr recording archive for playback",
    "studio replay pause            pause replay playback",
    "studio replay resume           resume replay playback",
    "studio replay speed MULTIPLIER set replay playback speed (0.01..20.0)",
    "studio replay seek MS          seek replay playback to timestamp in milliseconds",
    "studio replay status           query replay playback status",
    "studio replay close            close active replay session",
    "studio freecam [on|off]        toggle or set 6-DOF spectator freecam flight",
    "studio keyframe add|remove|list add or modify camera spline keyframes",
    "studio timeline [show|hide]    toggle studio timeline HUD overlay",
    "studio visuals                 toggle visuals & cinematic overrides sidebar",
    "studio export                  open offline video export dialog",
];

impl Game<'_> {
    /// In-game chat command handler: `/studio <record|replay> ...`.
    /// Returns true if the command was recognized and handled.
    pub(super) fn studio_command(&mut self, command: &str) -> bool {
        let Some(rest) = command.strip_prefix("studio") else {
            return false;
        };
        let words: Vec<&str> = rest.split_whitespace().collect();
        if words.is_empty() {
            self.system_message(Text::literal("Studio commands: /studio record <start|stop|status> or /studio replay <open|pause|resume|speed|seek|status|close>"));
            return true;
        }

        match words[0] {
            "record" => match words.get(1).copied() {
                Some("start") => {
                    let name = words.get(2).copied();
                    match self.start_studio_recording(name) {
                        Ok(v) if v["ok"].as_bool() == Some(true) => {
                            let path = v["path"].as_str().unwrap_or("recording");
                            self.system_message(Text::literal(format!(
                                "Studio: Started recording to {path}"
                            )));
                        }
                        Ok(v) => {
                            let err = v["error"].as_str().unwrap_or("failed to start");
                            self.system_message(Text::literal(format!("Studio error: {err}")));
                        }
                        Err(e) => {
                            self.system_message(Text::literal(format!("Studio error: {e}")));
                        }
                    }
                }
                Some("stop") => match self.stop_studio_recording() {
                    Ok(v) if v["ok"].as_bool() == Some(true) => {
                        let path = v["path"].as_str().unwrap_or("recording");
                        self.system_message(Text::literal(format!(
                            "Studio: Recording saved to {path}"
                        )));
                    }
                    Ok(v) => {
                        let err = v["error"].as_str().unwrap_or("failed to stop");
                        self.system_message(Text::literal(format!("Studio error: {err}")));
                    }
                    Err(e) => {
                        self.system_message(Text::literal(format!("Studio error: {e}")));
                    }
                },
                Some("status") => {
                    let v = self.studio_recording_status();
                    if v["active"].as_bool() == Some(true) {
                        let path = v["path"].as_str().unwrap_or("recording");
                        let elapsed_s = v["elapsed_ms"].as_u64().unwrap_or(0) / 1000;
                        let packets = v["packet_count"].as_u64().unwrap_or(0);
                        let bytes = v["bytes_written"].as_u64().unwrap_or(0);
                        self.system_message(Text::literal(format!(
                            "Studio Recording: Active, Time: {elapsed_s}s, Packets: {packets}, Size: {} KB ({path})",
                            bytes / 1024
                        )));
                    } else {
                        self.system_message(Text::literal("Studio Recording: Inactive"));
                    }
                }
                _ => {
                    self.system_message(Text::literal(
                        "Usage: /studio record <start [name]|stop|status>",
                    ));
                }
            },
            "replay" => match words.get(1).copied() {
                Some("open") => {
                    let Some(path) = words.get(2) else {
                        self.system_message(Text::literal("Usage: /studio replay open <PATH>"));
                        return true;
                    };
                    match self.open_studio_replay(path) {
                        Ok(v) if v["ok"].as_bool() == Some(true) => {
                            let duration_s = v["duration_ms"].as_f64().unwrap_or(0.0) / 1000.0;
                            let packets = v["packet_count"].as_u64().unwrap_or(0);
                            self.system_message(Text::literal(format!(
                                "Studio Replay: Opened {path} (Duration: {duration_s:.1}s, Packets: {packets})"
                            )));
                        }
                        Ok(v) => {
                            let err = v["error"].as_str().unwrap_or("failed to open replay");
                            self.system_message(Text::literal(format!(
                                "Studio Replay error: {err}"
                            )));
                        }
                        Err(e) => {
                            self.system_message(Text::literal(format!("Studio Replay error: {e}")));
                        }
                    }
                }
                Some("pause") => {
                    if let Some(r) = self.studio_replay.as_mut() {
                        r.set_paused(true);
                        self.system_message(Text::literal("Studio Replay: Paused"));
                    } else {
                        self.system_message(Text::literal("Studio Replay: No active replay"));
                    }
                }
                Some("resume") => {
                    if let Some(r) = self.studio_replay.as_mut() {
                        r.set_paused(false);
                        self.system_message(Text::literal("Studio Replay: Resumed"));
                    } else {
                        self.system_message(Text::literal("Studio Replay: No active replay"));
                    }
                }
                Some("speed") => {
                    let Some(spd_str) = words.get(2) else {
                        self.system_message(Text::literal(
                            "Usage: /studio replay speed <MULTIPLIER>",
                        ));
                        return true;
                    };
                    if let Ok(spd) = spd_str.parse::<f64>() {
                        let new_speed = if let Some(r) = self.studio_replay.as_mut() {
                            r.set_speed(spd);
                            Some(r.speed())
                        } else {
                            None
                        };
                        if let Some(speed) = new_speed {
                            self.system_message(Text::literal(format!(
                                "Studio Replay: Speed set to {:.2}x",
                                speed
                            )));
                        } else {
                            self.system_message(Text::literal("Studio Replay: No active replay"));
                        }
                    } else {
                        self.system_message(Text::literal("Studio Replay: Invalid speed number"));
                    }
                }
                Some("seek") => {
                    let Some(ms_str) = words.get(2) else {
                        self.system_message(Text::literal(
                            "Usage: /studio replay seek <MILLISECONDS>",
                        ));
                        return true;
                    };
                    if let Ok(ms) = ms_str.parse::<u32>() {
                        if let Some(r) = self.studio_replay.as_mut() {
                            match r.seek(ms) {
                                Ok(()) => {
                                    self.system_message(Text::literal(format!(
                                        "Studio Replay: Seek to {}ms ({:.1}s)",
                                        ms,
                                        ms as f32 / 1000.0
                                    )));
                                }
                                Err(e) => {
                                    self.system_message(Text::literal(format!(
                                        "Studio Replay seek error: {e}"
                                    )));
                                }
                            }
                        } else {
                            self.system_message(Text::literal("Studio Replay: No active replay"));
                        }
                    } else {
                        self.system_message(Text::literal("Studio Replay: Invalid milliseconds"));
                    }
                }
                Some("status") => {
                    if let Some(r) = self.studio_replay.as_ref() {
                        self.system_message(Text::literal(format!(
                            "Studio Replay: Pos: {}ms / {}ms ({:.1}%), Speed: {:.2}x, Paused: {}, Finished: {}",
                            r.position_ms(),
                            r.duration_ms(),
                            r.progress() * 100.0,
                            r.speed(),
                            r.is_paused(),
                            r.is_finished()
                        )));
                    } else {
                        self.system_message(Text::literal("Studio Replay: Inactive"));
                    }
                }
                Some("close") => {
                    if self.studio_replay.take().is_some() {
                        self.system_message(Text::literal("Studio Replay: Closed"));
                    } else {
                        self.system_message(Text::literal("Studio Replay: No active replay"));
                    }
                }
                _ => {
                    self.system_message(Text::literal(
                        "Usage: /studio replay <open|pause|resume|speed|seek|status|close>",
                    ));
                }
            },
            "freecam" => {
                let active = match words.get(1).copied() {
                    Some("on") => true,
                    Some("off") => false,
                    _ => !self.studio_freecam.active,
                };
                if active {
                    let (pos, yaw, pitch) =
                        self.camera_view(0.0)
                            .unwrap_or((glam::DVec3::ZERO, 0.0, 0.0));
                    self.studio_freecam
                        .activate(pos, yaw, pitch, self.settings.fov);
                    self.capture(true);
                    self.system_message(Text::literal(
                        "Studio Freecam: Activated (WASD fly, Space/Shift vertical, Ctrl boost, Mouse wheel speed)",
                    ));
                } else {
                    self.studio_freecam.deactivate();
                    self.system_message(Text::literal("Studio Freecam: Deactivated"));
                }
            }
            "keyframe" => match words.get(1).copied() {
                Some("add") => {
                    let time_ms = self.studio_timeline.current_time_ms.min(u32::MAX as u64) as u32;
                    let kf = horizon_client::studio::CameraKeyframe::new(
                        time_ms,
                        [
                            self.studio_freecam.position.x,
                            self.studio_freecam.position.y,
                            self.studio_freecam.position.z,
                        ],
                        self.studio_freecam.yaw,
                        self.studio_freecam.pitch,
                        self.studio_freecam.fov,
                    );
                    self.studio_freecam.track.insert(kf);
                    self.system_message(Text::literal(format!(
                        "Studio: Added camera keyframe at {}ms (total: {})",
                        time_ms,
                        self.studio_freecam.track.keyframes.len()
                    )));
                }
                Some("clear") => {
                    self.studio_freecam.track.clear();
                    self.studio_timeline.selected_keyframe = None;
                    self.system_message(Text::literal("Studio: Cleared all keyframes"));
                }
                Some("list") => {
                    let count = self.studio_freecam.track.keyframes.len();
                    self.system_message(Text::literal(format!("Studio Keyframes ({}):", count)));
                    let lines: Vec<String> = self
                        .studio_freecam
                        .track
                        .keyframes
                        .iter()
                        .enumerate()
                        .map(|(i, k)| {
                            format!(
                                "  #{i} @ {}ms: pos=({:.1}, {:.1}, {:.1}), yaw={:.1}, pitch={:.1}",
                                k.time_ms,
                                k.position[0],
                                k.position[1],
                                k.position[2],
                                k.yaw,
                                k.pitch
                            )
                        })
                        .collect();
                    for line in lines {
                        self.system_message(Text::literal(line));
                    }
                }
                _ => {
                    self.system_message(Text::literal("Usage: /studio keyframe <add|clear|list>"));
                }
            },
            "timeline" | "ui" => {
                let vis = match words.get(1).copied() {
                    Some("show") => true,
                    Some("hide") => false,
                    _ => !self.studio_timeline.visible,
                };
                self.studio_timeline.visible = vis;
                self.system_message(Text::literal(format!(
                    "Studio Timeline: {}",
                    if vis { "Visible" } else { "Hidden" }
                )));
            }
            "visuals" => {
                self.studio_visuals.open = !self.studio_visuals.open;
                self.system_message(Text::literal(format!(
                    "Studio Visuals sidebar: {}",
                    if self.studio_visuals.open {
                        "Open"
                    } else {
                        "Closed"
                    }
                )));
            }
            "export" => {
                self.studio_export.open = true;
                self.system_message(Text::literal("Studio: Opened video export dialog"));
            }
            _ => {
                self.system_message(Text::literal(
                    "Unknown studio subcommand. Usage: /studio <record|replay|freecam|keyframe|timeline|visuals|export> ...",
                ));
            }
        }
        true
    }

    /// Advance active replay playback by elapsed frame time and dispatch ready packets.
    pub(super) fn advance_studio_replay(&mut self) -> Result<()> {
        let now = std::time::Instant::now();
        let delta = match self.last_replay_tick {
            Some(last) => now
                .saturating_duration_since(last)
                .min(Duration::from_millis(250)),
            None => Duration::from_millis(16),
        };
        self.last_replay_tick = Some(now);

        let (ready_packets, pos, dur, paused) = {
            let Some(driver) = self.studio_replay.as_mut() else {
                self.last_replay_tick = None;
                return Ok(());
            };
            let pkts = match driver.advance(delta) {
                Ok(pkts) => pkts,
                Err(e) => {
                    log::warn!("Studio replay advance error: {e}");
                    return Ok(());
                }
            };
            (
                pkts,
                driver.position_ms(),
                driver.duration_ms(),
                driver.is_paused(),
            )
        };

        for pkt in ready_packets {
            match horizon_protocol::packets::play::Clientbound::decode(pkt.id, &pkt.payload) {
                Ok(packet) => {
                    let _ = self.handle_packet(packet);
                }
                Err(e) => {
                    let name = horizon_protocol::ids::name(
                        horizon_protocol::State::Play,
                        horizon_protocol::Direction::Clientbound,
                        pkt.id,
                    )
                    .unwrap_or("?");
                    log::debug!(
                        "Studio replay skipped packet {name} (id {:#x}): {e}",
                        pkt.id
                    );
                }
            }
        }

        self.studio_timeline.current_time_ms = pos as u64;
        self.studio_timeline.duration_ms = dur as u64;
        self.studio_timeline.playing = !paused;
        if self.studio_freecam.locked_to_path {
            self.studio_freecam.update_spline_playback(pos);
        }
        Ok(())
    }

    /// Control socket command handler for `studio`.
    pub(super) fn studio_control(&mut self, words: &[&str]) -> Result<Option<Value>> {
        if words.first().copied() != Some("studio") {
            return Ok(None);
        }
        match words.get(1).copied() {
            Some("record") => match words.get(2).copied() {
                Some("start") => {
                    let name = words.get(3).copied();
                    let res = self.start_studio_recording(name)?;
                    Ok(Some(res))
                }
                Some("stop") => {
                    let res = self.stop_studio_recording()?;
                    Ok(Some(res))
                }
                Some("status") => {
                    let res = self.studio_recording_status();
                    Ok(Some(res))
                }
                _ => Ok(Some(json!({ "error": "unknown studio record subcommand" }))),
            },
            Some("replay") => match words.get(2).copied() {
                Some("open") => {
                    let path = words.get(3).ok_or("missing path for studio replay open")?;
                    let res = self.open_studio_replay(path)?;
                    Ok(Some(res))
                }
                Some("pause") => {
                    if let Some(r) = self.studio_replay.as_mut() {
                        r.set_paused(true);
                        Ok(Some(json!({ "ok": true, "paused": true })))
                    } else {
                        Ok(Some(json!({ "error": "no active replay" })))
                    }
                }
                Some("resume") => {
                    if let Some(r) = self.studio_replay.as_mut() {
                        r.set_paused(false);
                        Ok(Some(json!({ "ok": true, "paused": false })))
                    } else {
                        Ok(Some(json!({ "error": "no active replay" })))
                    }
                }
                Some("speed") => {
                    let spd_str = words.get(3).ok_or("missing speed value")?;
                    let spd: f64 = spd_str.parse().map_err(|_| "invalid speed")?;
                    if let Some(r) = self.studio_replay.as_mut() {
                        r.set_speed(spd);
                        Ok(Some(json!({ "ok": true, "speed": r.speed() })))
                    } else {
                        Ok(Some(json!({ "error": "no active replay" })))
                    }
                }
                Some("seek") => {
                    let ms_str = words.get(3).ok_or("missing seek timestamp")?;
                    let ms: u32 = ms_str.parse().map_err(|_| "invalid timestamp")?;
                    if let Some(r) = self.studio_replay.as_mut() {
                        r.seek(ms).map_err(|e| format!("{e}"))?;
                        Ok(Some(json!({ "ok": true, "position_ms": r.position_ms() })))
                    } else {
                        Ok(Some(json!({ "error": "no active replay" })))
                    }
                }
                Some("status") => {
                    if let Some(r) = self.studio_replay.as_ref() {
                        Ok(Some(json!({
                            "ok": true,
                            "active": true,
                            "position_ms": r.position_ms(),
                            "duration_ms": r.duration_ms(),
                            "progress": r.progress(),
                            "speed": r.speed(),
                            "paused": r.is_paused(),
                            "finished": r.is_finished(),
                        })))
                    } else {
                        Ok(Some(json!({ "ok": true, "active": false })))
                    }
                }
                Some("close") => {
                    let closed = self.studio_replay.take().is_some();
                    Ok(Some(json!({ "ok": true, "closed": closed })))
                }
                _ => Ok(Some(json!({ "error": "unknown studio replay subcommand" }))),
            },
            _ => Ok(Some(json!({ "error": "unknown studio command" }))),
        }
    }

    pub(super) fn start_studio_recording(&mut self, name: Option<&str>) -> Result<Value> {
        let Some(r) = self.remote.as_ref() else {
            return Ok(json!({ "ok": false, "error": "no active world session" }));
        };
        if r.session.is_recording() {
            return Ok(json!({ "ok": false, "error": "recording already active" }));
        }
        let _ = std::fs::create_dir_all("recordings");
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let filename = match name {
            Some(n) if !n.is_empty() => format!("{}_{}.mcpr", n, ts),
            _ => format!("recording_{}.mcpr", ts),
        };
        let path = PathBuf::from("recordings").join(filename);
        let server_address = if r.session.address.is_empty() {
            "local".to_owned()
        } else {
            r.session.address.clone()
        };
        let is_singleplayer = server_address == "local";
        let meta = Metadata::new(
            server_address,
            is_singleplayer,
            horizon_core::release::VERSION.into(),
        );
        match r.session.start_recording(&path, meta) {
            Ok(()) => Ok(json!({ "ok": true, "path": path.display().to_string() })),
            Err(e) => Ok(json!({ "ok": false, "error": e.to_string() })),
        }
    }

    pub(super) fn stop_studio_recording(&mut self) -> Result<Value> {
        let Some(r) = self.remote.as_ref() else {
            return Ok(json!({ "ok": false, "error": "no active world session" }));
        };
        match r.session.stop_recording() {
            Some(Ok(path)) => Ok(json!({ "ok": true, "path": path.display().to_string() })),
            Some(Err(e)) => Ok(json!({ "ok": false, "error": e.to_string() })),
            None => Ok(json!({ "ok": false, "error": "no active recording" })),
        }
    }

    pub(super) fn studio_recording_status(&self) -> Value {
        if let Some(r) = self.remote.as_ref()
            && let Some(status) = r.session.recorder_status()
        {
            json!({
                "ok": true,
                "active": status.active,
                "path": status.path.display().to_string(),
                "elapsed_ms": status.elapsed.as_millis(),
                "packet_count": status.packet_count,
                "bytes_written": status.bytes_written,
            })
        } else {
            json!({ "ok": true, "active": false })
        }
    }

    pub(super) fn open_studio_replay(&mut self, path: &str) -> Result<Value> {
        match ReplayDriver::open(Path::new(path)) {
            Ok(driver) => {
                let duration_ms = driver.duration_ms();
                let packets = driver.packet_count();
                self.studio_timeline.duration_ms = duration_ms as u64;
                self.studio_timeline.current_time_ms = 0;
                self.studio_timeline.visible = true;
                self.studio_replay = Some(driver);
                Ok(json!({
                    "ok": true,
                    "duration_ms": duration_ms,
                    "packet_count": packets,
                }))
            }
            Err(e) => Ok(json!({ "ok": false, "error": e.to_string() })),
        }
    }
}
