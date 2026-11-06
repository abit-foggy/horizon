//! Studio Replay Playback Driver
//!
//! Feeds recorded `.mcpr` packet streams into the client's packet processing
//! loop at user-controlled playback speeds (slow-motion, 1x, fast-forward, pause).

use super::mcpr::{Marker, McprReader, McprWriter, Metadata, TimestampedPacket};
use super::spline::CameraTrack;
use std::{
    io,
    path::{Path, PathBuf},
    time::Duration,
};

/// Playback controller driving recorded packets according to a virtual timeline.
pub struct ReplayDriver {
    pub metadata: Metadata,
    reader: McprReader,
    path: PathBuf,
    cursor_ms: f64,
    speed: f64,
    paused: bool,
    looping: bool,
    finished: bool,
    next_packet_cache: Option<TimestampedPacket>,
}

impl ReplayDriver {
    /// Open an `.mcpr` recording archive for replay playback.
    pub fn open(path: &Path) -> io::Result<Self> {
        let mut reader = McprReader::open(path)?;
        validate_metadata(&reader.metadata)?;
        let metadata = reader.metadata.clone();
        let next_packet_cache = reader.next_packet()?;
        let first = next_packet_cache
            .as_ref()
            .ok_or_else(|| invalid_replay("recording contains no play-state packets"))?;
        validate_play_stream_start(first)?;

        Ok(Self {
            metadata,
            reader,
            path: path.to_path_buf(),
            cursor_ms: 0.0,
            speed: 1.0,
            paused: false,
            looping: false,
            finished: false,
            next_packet_cache,
        })
    }

    /// Current timeline position in milliseconds.
    pub fn position_ms(&self) -> u32 {
        self.cursor_ms as u32
    }

    /// Total recording duration in milliseconds.
    pub fn duration_ms(&self) -> u32 {
        self.metadata.duration
    }

    /// Total number of indexed packets in the recording stream.
    pub fn packet_count(&self) -> usize {
        self.reader.total_packets()
    }

    pub fn markers(&self) -> &[Marker] {
        &self.reader.markers
    }

    pub fn camera_track(&self) -> Option<&CameraTrack> {
        self.reader.camera_track.as_ref()
    }

    pub fn save_camera_track(&mut self, track: &CameraTrack) -> io::Result<()> {
        McprWriter::replace_camera_track(&self.path, track)?;
        self.reader.camera_track = (!track.is_empty()).then(|| track.clone());
        Ok(())
    }

    /// Add a bookmark at the replay playhead and persist it in markers.json.
    pub fn add_marker(&mut self, label: String) -> io::Result<()> {
        let marker = Marker {
            time_ms: self.position_ms(),
            label,
        };
        let mut markers = self.reader.markers.clone();
        markers.push(marker);
        markers.sort_by_key(|marker| marker.time_ms);
        McprWriter::replace_markers(&self.path, &markers)?;
        self.reader.markers = markers;
        Ok(())
    }

    /// Current playback progress as a fraction from 0.0 to 1.0.
    pub fn progress(&self) -> f32 {
        if self.metadata.duration == 0 {
            0.0
        } else {
            (self.cursor_ms / self.metadata.duration as f64).clamp(0.0, 1.0) as f32
        }
    }

    /// Current playback speed multiplier (e.g. 0.25, 1.0, 2.0).
    pub fn speed(&self) -> f64 {
        self.speed
    }

    /// Set playback speed multiplier. Clamped between 0.01x and 20.0x.
    pub fn set_speed(&mut self, speed: f64) {
        self.speed = speed.clamp(0.01, 20.0);
    }

    /// Check if playback is paused.
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Set paused state.
    pub fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
    }

    /// Toggle paused state.
    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }

    /// Set whether replay should loop when reaching the end.
    pub fn set_looping(&mut self, looping: bool) {
        self.looping = looping;
    }

    /// Check if playback reached the end of the recording.
    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// Advance virtual time by `delta` and collect all packets scheduled up to the new time.
    pub fn advance(&mut self, delta: Duration) -> io::Result<Vec<TimestampedPacket>> {
        if self.paused || self.finished {
            return Ok(Vec::new());
        }

        let delta_ms = delta.as_secs_f64() * 1000.0 * self.speed;
        self.cursor_ms += delta_ms;

        let mut ready = Vec::new();
        let target_ms = self.cursor_ms as u32;

        loop {
            let Some(pkt) = &self.next_packet_cache else {
                if self.cursor_ms >= self.metadata.duration as f64 {
                    if self.looping {
                        self.seek(0)?;
                    } else {
                        self.finished = true;
                    }
                }
                break;
            };

            if pkt.timestamp_ms <= target_ms {
                ready.push(self.next_packet_cache.take().unwrap());
                self.next_packet_cache = self.reader.next_packet()?;
            } else {
                break;
            }
        }

        Ok(ready)
    }

    /// Seek directly to a specific timestamp in milliseconds.
    pub fn seek(&mut self, target_ms: u32) -> io::Result<()> {
        let clamped_ms = target_ms.min(self.metadata.duration);
        self.cursor_ms = clamped_ms as f64;
        self.finished = false;

        self.reader.seek_to_timestamp(clamped_ms)?;
        self.next_packet_cache = self.reader.next_packet()?;
        Ok(())
    }

    /// Rewind playback to the beginning.
    pub fn restart(&mut self) -> io::Result<()> {
        self.seek(0)
    }
}

fn invalid_replay(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

fn validate_metadata(metadata: &Metadata) -> io::Result<()> {
    if metadata.file_format != "MCPR" {
        return Err(invalid_replay(format!(
            "unsupported replay file format {:?}",
            metadata.file_format
        )));
    }
    if metadata.file_format_version != 14 {
        return Err(invalid_replay(format!(
            "unsupported MCPR format version {}; this build supports 14",
            metadata.file_format_version
        )));
    }
    match metadata.protocol {
        Some(protocol) if protocol == horizon_protocol::PROTOCOL_VERSION => {}
        Some(protocol) => {
            return Err(invalid_replay(format!(
                "unsupported Minecraft protocol {protocol}; this build supports {}",
                horizon_protocol::PROTOCOL_VERSION
            )));
        }
        None => {
            return Err(invalid_replay(
                "replay does not declare a Minecraft protocol version",
            ));
        }
    }
    Ok(())
}

/// This player consumes only Play-state packets. A version-14 recording must
/// begin with the Play `Login` snapshot packet; refuse streams from a different
/// connection phase before packet IDs are interpreted against Play's table.
fn validate_play_stream_start(first: &TimestampedPacket) -> io::Result<()> {
    let packet = horizon_protocol::packets::play::Clientbound::decode(first.id, &first.payload)
        .map_err(|error| invalid_replay(format!("invalid initial play packet: {error}")))?;
    if !matches!(
        packet,
        horizon_protocol::packets::play::Clientbound::Login(_)
    ) {
        return Err(invalid_replay(
            "recording does not start with the Play Login packet",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::studio::mcpr::McprWriter;
    use std::fs;

    fn play_login_packet() -> (i32, Vec<u8>) {
        use horizon_protocol::{buf::Reader, packets::play};

        let encoded = play::Clientbound::Login(play::Login {
            player_id: 1,
            hardcore: false,
            levels: vec!["minecraft:overworld".into()],
            max_players: 20,
            chunk_radius: 8,
            simulation_distance: 8,
            reduced_debug_info: false,
            show_death_screen: true,
            limited_crafting: false,
            spawn: play::SpawnInfo {
                dimension_type: 0,
                dimension: "minecraft:overworld".into(),
                seed: 0,
                game_type: 0,
                previous_game_type: None,
                debug: false,
                flat: false,
                last_death: None,
                portal_cooldown: 0,
                sea_level: 63,
            },
            online_mode: false,
            enforces_secure_chat: false,
        })
        .encode();
        let mut reader = Reader::new(&encoded);
        let id = reader.varint().unwrap();
        (id, reader.rest().to_vec())
    }

    fn write_play_login(writer: &mut McprWriter, timestamp_ms: u32) {
        let (id, payload) = play_login_packet();
        writer.write_packet(timestamp_ms, id, &payload).unwrap();
    }

    #[test]
    fn playback_refuses_unknown_format_or_protocol_metadata() {
        let mut metadata = Metadata::default();
        assert!(validate_metadata(&metadata).is_ok());

        metadata.file_format = "OTHER".into();
        assert_eq!(
            validate_metadata(&metadata).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        metadata = Metadata::default();
        metadata.file_format_version = 13;
        assert_eq!(
            validate_metadata(&metadata).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        metadata = Metadata::default();
        metadata.protocol = None;
        assert_eq!(
            validate_metadata(&metadata).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        metadata.protocol = Some(horizon_protocol::PROTOCOL_VERSION + 1);
        assert_eq!(
            validate_metadata(&metadata).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn playback_requires_a_play_login_phase_start() {
        let (id, payload) = play_login_packet();
        assert!(validate_play_stream_start(&TimestampedPacket::new(0, id, payload)).is_ok());

        use horizon_protocol::packets::play;
        let encoded =
            play::Clientbound::SetChunkCacheCenter(play::SetChunkCacheCenter { x: 0, z: 0 })
                .encode();
        let mut reader = horizon_protocol::buf::Reader::new(&encoded);
        let id = reader.varint().unwrap();
        assert_eq!(
            validate_play_stream_start(&TimestampedPacket::new(0, id, reader.rest().to_vec()))
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn test_replay_driver_advance_and_speed() {
        let temp_dir =
            std::env::temp_dir().join(format!("replay_driver_test_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);
        let mcpr_path = temp_dir.join("driver_test.mcpr");

        let meta = Metadata {
            duration: 1000,
            ..Default::default()
        };
        let mut writer = McprWriter::new(mcpr_path.clone(), meta).unwrap();
        write_play_login(&mut writer, 0);
        writer.write_packet(0, 0x01, &[1]).unwrap();
        writer.write_packet(100, 0x02, &[2]).unwrap();
        writer.write_packet(200, 0x03, &[3]).unwrap();
        writer.write_packet(500, 0x04, &[4]).unwrap();
        writer.finish(1000).unwrap();

        let mut driver = ReplayDriver::open(&mcpr_path).unwrap();
        assert_eq!(driver.position_ms(), 0);
        assert_eq!(driver.duration_ms(), 1000);

        // Advance 50ms at 1.0x -> receives packet at 0ms
        let batch1 = driver.advance(Duration::from_millis(50)).unwrap();
        assert_eq!(batch1.len(), 2);
        assert_eq!(batch1[0].id, play_login_packet().0);
        assert_eq!(batch1[1].id, 0x01);

        // Advance another 100ms (now 150ms) -> receives packet at 100ms
        let batch2 = driver.advance(Duration::from_millis(100)).unwrap();
        assert_eq!(batch2.len(), 1);
        assert_eq!(batch2[0].id, 0x02);

        // Paused -> advance produces nothing
        driver.set_paused(true);
        let batch_paused = driver.advance(Duration::from_millis(200)).unwrap();
        assert!(batch_paused.is_empty());
        driver.set_paused(false);

        // Speed 2.0x -> advance 100ms real time advances 200ms virtual time (150ms -> 350ms)
        driver.set_speed(2.0);
        let batch3 = driver.advance(Duration::from_millis(100)).unwrap();
        assert_eq!(batch3.len(), 1);
        assert_eq!(batch3[0].id, 0x03);

        // Seek back to 0
        driver.seek(0).unwrap();
        assert_eq!(driver.position_ms(), 0);

        let _ = fs::remove_file(mcpr_path);
        let _ = fs::remove_dir(temp_dir);
    }

    #[test]
    fn replay_marker_is_saved_and_loaded_from_the_archive() {
        let temp_dir = std::env::temp_dir().join(format!(
            "replay_marker_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&temp_dir).unwrap();
        let mcpr_path = temp_dir.join("markers.mcpr");
        let mut writer = McprWriter::new(
            mcpr_path.clone(),
            Metadata {
                duration: 1_000,
                ..Default::default()
            },
        )
        .unwrap();
        write_play_login(&mut writer, 0);
        writer.finish(1_000).unwrap();

        let mut replay = ReplayDriver::open(&mcpr_path).unwrap();
        replay.seek(250).unwrap();
        replay.add_marker("Bookmark".into()).unwrap();
        drop(replay);

        let replay = ReplayDriver::open(&mcpr_path).unwrap();
        assert_eq!(
            replay.markers(),
            &[Marker {
                time_ms: 250,
                label: "Bookmark".into(),
            }]
        );

        fs::remove_dir_all(temp_dir).unwrap();
    }
}
