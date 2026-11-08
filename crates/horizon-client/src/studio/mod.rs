//! Horizon In-Game Video Studio.
//!
//! Provides native session packet recording (`.mcpr` format).

pub mod camera;
pub mod editor_preferences;
pub mod mcpr;
pub mod path_render;
pub mod recorder;
pub mod replay;
pub mod spline;
pub mod tabs;
pub mod timeline_ui;
pub mod topbar;
pub mod visuals_ui;

pub use camera::StudioFreecam;
pub use mcpr::{
    McprReader, McprWriter, Metadata, RecoveryReport, TimestampedPacket, recover_crashed_recordings,
};
pub use path_render::render_camera_path;
pub use recorder::{RecorderStatus, StoredAppearance, StudioRecorder, generate_recording_filename};
pub use replay::ReplayDriver;
pub use spline::{CameraKeyframe, CameraTrack, EvaluatedPose};
pub use tabs::{StudioTab, StudioTabSystem};
pub use timeline_ui::{TimelineAction, TimelineUi};
pub use topbar::{TopBarAction, TopBarUi};
pub use visuals_ui::{VisualsSettings, VisualsUi};
