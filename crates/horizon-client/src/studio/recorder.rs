//! Studio Live Session Packet Recorder
//!
//! Captures clientbound network packets in real-time onto a background writer
//! thread so gameplay frame rates and latency are unaffected.

use super::mcpr::{McprWriter, Metadata, TimestampedPacket};
use std::{
    collections::HashMap,
    io,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{Receiver, SyncSender, TrySendError, sync_channel},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

static RECORDING_NAME_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Commands sent to the background recording thread.
enum RecordCommand {
    Packet(TimestampedPacket),
    Marker(u32, String),
    Stop,
}

const RECORD_QUEUE_PACKETS: usize = 2048;

/// Status summary of the live recording.
#[derive(Clone, Debug, PartialEq)]
pub struct RecorderStatus {
    pub active: bool,
    pub path: PathBuf,
    pub elapsed: Duration,
    pub packet_count: u64,
    pub bytes_written: u64,
    pub error: Option<String>,
}

/// Asynchronous packet recorder that writes `.mcpr` sessions on a worker thread.
pub struct StudioRecorder {
    active: Arc<AtomicBool>,
    packet_count: Arc<AtomicU64>,
    bytes_written: Arc<AtomicU64>,
    sender: SyncSender<RecordCommand>,
    failure: Arc<std::sync::Mutex<Option<String>>>,
    worker: Option<JoinHandle<io::Result<PathBuf>>>,
    start_time: Instant,
    dest_path: PathBuf,
    appearance_capture: (bool, bool),
    anonymous_ids: std::sync::Mutex<HashMap<u128, u128>>,
    appearances: std::sync::Mutex<HashMap<u128, StoredAppearance>>,
}

impl StudioRecorder {
    /// Start a new recording session saving to `dest_path`.
    pub fn start(dest_path: PathBuf, metadata: Metadata) -> io::Result<Self> {
        Self::start_with_appearances(dest_path, metadata, false, false)
    }

    /// Start a recording with optional local-only appearance capture.
    pub fn start_with_appearances(
        dest_path: PathBuf,
        metadata: Metadata,
        capture_skins: bool,
        capture_names: bool,
    ) -> io::Result<Self> {
        Self::start_with_archive_options(
            dest_path,
            metadata,
            capture_skins,
            capture_names,
            None,
            None,
        )
    }

    pub fn start_with_archive_options(
        dest_path: PathBuf,
        mut metadata: Metadata,
        capture_skins: bool,
        capture_names: bool,
        mods_json: Option<Vec<u8>>,
        resource_pack: Option<PathBuf>,
    ) -> io::Result<Self> {
        let active = Arc::new(AtomicBool::new(true));
        let packet_count = Arc::new(AtomicU64::new(0));
        let bytes_written = Arc::new(AtomicU64::new(0));

        let start_time = Instant::now();
        let start_unix_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        metadata.date = start_unix_ms;

        let (sender, receiver): (SyncSender<RecordCommand>, Receiver<RecordCommand>) =
            sync_channel(RECORD_QUEUE_PACKETS);
        let mut writer = McprWriter::new(dest_path.clone(), metadata)?;
        writer.set_mods_json(mods_json);
        writer.set_resource_pack(resource_pack);

        let worker_active = active.clone();
        let worker_count = packet_count.clone();
        let worker_bytes = bytes_written.clone();
        let failure = Arc::new(std::sync::Mutex::new(None));
        let worker_failure = failure.clone();
        let thread_dest = dest_path.clone();

        let worker = thread::Builder::new()
            .name("Studio Recorder".into())
            .spawn(move || -> io::Result<PathBuf> {
                let mut duration_ms = 0;
                while let Ok(cmd) = receiver.recv() {
                    match cmd {
                        RecordCommand::Packet(pkt) => {
                            duration_ms = duration_ms.max(pkt.timestamp_ms);
                            if let Err(error) =
                                writer.write_packet(pkt.timestamp_ms, pkt.id, &pkt.payload)
                            {
                                *worker_failure.lock().unwrap() = Some(error.to_string());
                                worker_active.store(false, Ordering::SeqCst);
                                break;
                            }
                            worker_count.fetch_add(1, Ordering::Relaxed);
                            worker_bytes.store(writer.bytes_written(), Ordering::Relaxed);
                        }
                        RecordCommand::Marker(time_ms, label) => writer.add_marker(time_ms, label),
                        RecordCommand::Stop => break,
                    }
                }
                // Drain any lingering packets in queue
                while let Ok(cmd) = receiver.try_recv() {
                    if let RecordCommand::Packet(pkt) = cmd {
                        duration_ms = duration_ms.max(pkt.timestamp_ms);
                        if let Err(error) =
                            writer.write_packet(pkt.timestamp_ms, pkt.id, &pkt.payload)
                        {
                            *worker_failure.lock().unwrap() = Some(error.to_string());
                            break;
                        }
                        worker_count.fetch_add(1, Ordering::Relaxed);
                        worker_bytes.store(writer.bytes_written(), Ordering::Relaxed);
                    }
                }
                worker_active.store(false, Ordering::SeqCst);
                writer.finish(duration_ms)?;
                if let Some(error) = worker_failure.lock().unwrap().take() {
                    return Err(io::Error::other(format!(
                        "recording stopped with an error: {error}"
                    )));
                }
                Ok(thread_dest)
            })?;

        Ok(Self {
            active,
            packet_count,
            bytes_written,
            sender,
            failure,
            worker: Some(worker),
            start_time,
            dest_path,
            appearance_capture: (capture_skins, capture_names),
            anonymous_ids: std::sync::Mutex::new(HashMap::new()),
            appearances: std::sync::Mutex::new(HashMap::new()),
        })
    }

    /// A stable random UUID for a real profile, used only in the `.mcpr` stream.
    pub fn anonymous_uuid(&self, real: u128) -> u128 {
        let mut anonymous_ids = self.anonymous_ids.lock().unwrap();
        if let Some(id) = anonymous_ids.get(&real) {
            return *id;
        }
        let mut bytes = [0; 16];
        if horizon_protocol::crypto::random_bytes(&mut bytes).is_err() {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let mut seed = real.to_be_bytes().to_vec();
            seed.extend_from_slice(&nonce.to_be_bytes());
            bytes.copy_from_slice(&horizon_client::skins::sha1(&seed)[..16]);
        }
        let mut id = u128::from_be_bytes(bytes);
        while anonymous_ids.values().any(|known| *known == id) || id == real {
            id = id.wrapping_add(1);
        }
        anonymous_ids.insert(real, id);
        id
    }

    /// Retain the first resolved local appearance for an entity; updates are
    /// accepted when its skin or cape changes during a session.
    pub fn record_appearance(
        &self,
        real: u128,
        name: Option<&str>,
        slim: bool,
        skin_rgba: Option<&[u8]>,
        cape_rgba: Option<&[u8]>,
    ) {
        if !self.appearance_capture.0 && !self.appearance_capture.1 {
            return;
        }
        let uuid = self.anonymous_uuid(real);
        let mut appearances = self.appearances.lock().unwrap();
        let previous = appearances.get(&uuid);
        if previous.is_some_and(|old| {
            old.name.as_deref() == self.appearance_capture.1.then_some(name).flatten()
                && old.slim == (self.appearance_capture.0 && slim)
                && old.skin_rgba.as_deref()
                    == self.appearance_capture.0.then_some(skin_rgba).flatten()
                && old.cape_rgba.as_deref()
                    == self.appearance_capture.0.then_some(cape_rgba).flatten()
        }) {
            return;
        }
        let name = self
            .appearance_capture
            .1
            .then_some(name)
            .flatten()
            .map(str::to_owned);
        let skin_rgba = self
            .appearance_capture
            .0
            .then_some(skin_rgba)
            .flatten()
            .map(<[u8]>::to_vec);
        let cape_rgba = self
            .appearance_capture
            .0
            .then_some(cape_rgba)
            .flatten()
            .map(<[u8]>::to_vec);
        appearances.insert(
            uuid,
            StoredAppearance {
                uuid,
                name,
                slim: self.appearance_capture.0 && slim,
                skin_rgba,
                cape_rgba,
            },
        );
    }

    /// Record a clientbound packet if recording is active.
    pub fn record_packet(&self, id: i32, payload: &[u8]) {
        let Some((id, payload)) = self.sanitized_packet(id, payload) else {
            return;
        }
        let timestamp_ms = self.start_time.elapsed().as_millis() as u32;
        let pkt = TimestampedPacket::new(timestamp_ms, id, payload.to_vec());
        if let Err(error) = self.sender.try_send(RecordCommand::Packet(pkt)) {
            let message = match error {
                TrySendError::Full(_) => "recorder queue is full; capture stopped",
                TrySendError::Disconnected(_) => "recorder worker stopped unexpectedly",
            };
            *self.failure.lock().unwrap() = Some(message.into());
            self.active.store(false, Ordering::SeqCst);
        }
    }

    /// Record a clientbound packet at an explicit timestamp (e.g. 0ms for initial world baseline).
    pub fn record_packet_at(&self, timestamp_ms: u32, id: i32, payload: &[u8]) {
        if !self.active.load(Ordering::Relaxed) {
            return;
        }
        let pkt = TimestampedPacket::new(timestamp_ms, id, payload.to_vec());
        if let Err(error) = self.sender.try_send(RecordCommand::Packet(pkt)) {
            let message = match error {
                TrySendError::Full(_) => "recorder queue is full; capture stopped",
                TrySendError::Disconnected(_) => "recorder worker stopped unexpectedly",
            };
            *self.failure.lock().unwrap() = Some(message.into());
            self.active.store(false, Ordering::SeqCst);
        }
    }

    /// Record a packet at a supplied timestamp after applying the privacy filter.
    pub fn record_packet_at(&self, timestamp_ms: u32, id: i32, payload: &[u8]) {
        let Some((id, payload)) = self.sanitized_packet(id, payload) else {
            return;
        };
        self.queue_packet(id, &payload, Some(timestamp_ms));
    }

    /// Add a user-created timeline marker at the current recording time.
    pub fn record_marker(&self, label: String) {
        if self.is_active() {
            let time_ms = self.start_time.elapsed().as_millis().min(u32::MAX as u128) as u32;
            let _ = self.sender.try_send(RecordCommand::Marker(time_ms, label));
        }
    }

    fn sanitized_packet(&self, id: i32, payload: &[u8]) -> Option<(i32, Vec<u8>)> {
        use horizon_protocol::packets::play::Clientbound as C;
        let Ok(mut packet) = C::decode(id, payload) else {
            // The live network path records only packets that decoded above;
            // retain opaque packets from explicit snapshot callers unchanged.
            return Some((id, payload.to_vec()));
        };
        if matches!(
            &packet,
            C::CookieRequest(_)
                | C::CustomPayload(_)
                | C::DisguisedChat(_)
                | C::PlayerChat(_)
                | C::PlayerInfoRemove(_)
                | C::PlayerInfoUpdate(_)
                | C::PlayerCombatKill(_)
                | C::ResourcePackPush(_)
                | C::ResetScore(_)
                | C::ServerLinks(_)
                | C::SetPlayerTeam(_)
                | C::SetScore(_)
                | C::StoreCookie(_)
                | C::SystemChat(_)
                | C::TabList(_)
                | C::Transfer(_)
        ) {
            return None;
        }
        if let C::SetEntityData(data) = &mut packet {
            // Keep only the shared entity flags and pose. These carry visible
            // movement (including crouching and swimming) without recording
            // custom names, profiles, items, or other identifying metadata.
            data.values.retain(|(index, serializer, value)| {
                matches!(
                    (*index, *serializer, value),
                    (
                        0,
                        horizon_protocol::entity_data::serializer::BYTE,
                        horizon_protocol::entity_data::Value::Byte(_)
                    ) | (
                        6,
                        horizon_protocol::entity_data::serializer::POSE,
                        horizon_protocol::entity_data::Value::Id(_)
                    )
                )
            });
            if data.values.is_empty() {
                return None;
            }
        }
        if let C::AddEntity(entity) = &mut packet {
            entity.uuid = self.anonymous_uuid(entity.uuid);
        }
        let encoded = packet.encode();
        let mut reader = horizon_protocol::buf::Reader::new(&encoded);
        let packet_id = reader.varint().ok()?;
        Some((packet_id, reader.rest().to_vec()))
    }

    fn queue_packet(&self, id: i32, payload: &[u8], at: Option<u32>) {
        if !self.active.load(Ordering::Relaxed) {
            return;
        }
        let timestamp_ms = at.unwrap_or_else(|| self.start_time.elapsed().as_millis() as u32);
        let pkt = TimestampedPacket::new(timestamp_ms, id, payload.to_vec());
        if let Err(error) = self.sender.try_send(RecordCommand::Packet(pkt)) {
            let message = match error {
                TrySendError::Full(_) => "recorder queue is full; capture stopped",
                TrySendError::Disconnected(_) => "recorder worker stopped unexpectedly",
            };
            *self.failure.lock().unwrap() = Some(message.into());
            self.active.store(false, Ordering::SeqCst);
        }
    }

    /// Check if currently recording.
    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Relaxed)
    }

    /// Query current recording status.
    pub fn status(&self) -> RecorderStatus {
        RecorderStatus {
            active: self.is_active(),
            path: self.dest_path.clone(),
            elapsed: self.start_time.elapsed(),
            packet_count: self.packet_count.load(Ordering::Relaxed),
            bytes_written: self.bytes_written.load(Ordering::Relaxed),
            error: self.failure.lock().unwrap().clone(),
        }
    }

    /// Stop recording and finalize the `.mcpr` archive.
    pub fn stop(mut self) -> io::Result<PathBuf> {
        self.active.store(false, Ordering::SeqCst);
        let _ = self.sender.send(RecordCommand::Stop);
        let result = if let Some(worker) = self.worker.take() {
            match worker.join() {
                Ok(res) => res,
                Err(_) => Err(io::Error::other("studio recorder worker thread panicked")),
            }
        } else {
            Ok(self.dest_path.clone())
        }?;
        let appearances = self.appearances.lock().unwrap();
        if !appearances.is_empty() {
            let local_root = horizon_client::logging::player_dir().join("studio-appearances");
            std::fs::create_dir_all(&local_root)?;
            let stem = self.dest_path.file_name().unwrap_or_default();
            let sidecar = local_root.join(format!("{}.json", stem.to_string_lossy()));
            let bytes = serde_json::to_vec(&appearances.values().collect::<Vec<_>>())
                .map_err(io::Error::other)?;
            std::fs::write(sidecar, bytes)?;
        }
        Ok(result)
    }
}

impl Drop for StudioRecorder {
    fn drop(&mut self) {
        self.active.store(false, Ordering::SeqCst);
        let _ = self.sender.send(RecordCommand::Stop);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

/// Default storage directory for video studio recordings (`recordings/`).
pub fn default_recordings_dir() -> PathBuf {
    Path::new("recordings").to_path_buf()
}

/// Generate a timestamped file name for a recording.
pub fn generate_recording_filename(name: Option<&str>) -> String {
    let now = chrono::Local::now();
    let sequence = RECORDING_NAME_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let stamp = format!("{}_{}", now.format("%Y-%m-%d_%H-%M-%S_%3f"), sequence);
    match name {
        Some(custom) if !custom.trim().is_empty() => {
            let stem: String = custom
                .trim()
                .chars()
                .filter_map(|c| match c {
                    'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => Some(c),
                    _ if c.is_whitespace() => Some('_'),
                    _ => None,
                })
                .take(80)
                .collect();
            let stem = stem.trim_matches(['-', '_']);
            if stem.is_empty() {
                format!("recording_{stamp}.mcpr")
            } else {
                format!("{stem}_{stamp}.mcpr")
            }
        }
        _ => format!("recording_{stamp}.mcpr"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_studio_recorder_lifecycle() {
        let temp_dir = std::env::temp_dir().join(format!(
            "studio_rec_{}_{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::create_dir_all(&temp_dir);
        let mcpr_path = temp_dir.join("live_test.mcpr");

        let meta = Metadata::default();
        let recorder = StudioRecorder::start(mcpr_path.clone(), meta).unwrap();
        assert!(recorder.is_active());

        // Send a few packets
        recorder.record_packet(0x01, &[1, 2, 3]);
        recorder.record_packet(0x02, &[4, 5]);

        let st = recorder.status();
        assert_eq!(st.path, mcpr_path);

        let finished = recorder.stop().unwrap();
        assert_eq!(finished, mcpr_path);
        assert!(mcpr_path.exists());

        // Verify with reader
        let mut reader = super::super::mcpr::McprReader::open(&mcpr_path).unwrap();
        assert_eq!(reader.total_packets(), 2);
        let p0 = reader.next_packet().unwrap().unwrap();
        assert_eq!(p0.id, 0x01);
        let p1 = reader.next_packet().unwrap().unwrap();
        assert_eq!(p1.id, 0x02);

        let _ = fs::remove_file(mcpr_path);
        let _ = fs::remove_dir(temp_dir);
    }
}
