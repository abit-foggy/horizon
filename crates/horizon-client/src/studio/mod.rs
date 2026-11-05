//! Horizon In-Game Video Studio.
//!
//! Provides native session packet recording (`.mcpr` format), replay simulation,
//! 6-DOF spectator freecam flight, 3D camera spline keyframing, timeline sequencer HUD,
//! visuals & overrides inspector, and offline cinematic video export.

pub mod camera;
pub mod export_ui;
pub mod mcpr;
pub mod path_render;
pub mod recorder;
pub mod replay;
pub mod spline;
pub mod timeline_ui;
pub mod topbar;
pub mod visuals_ui;

pub use camera::StudioFreecam;
pub use export_ui::{ExportResolution, ExportSettings, ExportUi};
pub use mcpr::{
    McprReader, McprWriter, Metadata, RecoveryReport, TimestampedPacket, recover_crashed_recordings,
};
pub use path_render::render_camera_path;
pub use recorder::{RecorderStatus, StoredAppearance, StudioRecorder, generate_recording_filename};
pub use replay::ReplayDriver;
pub use spline::{CameraKeyframe, CameraTrack, EvaluatedPose};
pub use timeline_ui::{TimelineAction, TimelineTrackKind, TimelineUi};
pub use topbar::{TopBarAction, TopBarMenu, TopBarUi};
pub use visuals_ui::{AspectRatio, VisualsSettings, VisualsUi};
