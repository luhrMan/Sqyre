//! Audible cue preferences.

use serde::{Deserialize, Serialize};

pub const DEFAULT_PLAY_FINISH_SOUND: bool = true;
pub const DEFAULT_SOUND_VOLUME: f32 = 0.25;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SoundSettings {
    /// Play an audible cue when a top-level macro run finishes successfully.
    #[serde(default = "default_play_finish_sound")]
    pub play_finish_sound: bool,

    /// Play an audible cue when the user adds macros, actions, or catalog entities.
    #[serde(default = "default_play_add_sound")]
    pub play_add_sound: bool,

    /// Play an audible cue when the user deletes macros, actions, or catalog entities.
    #[serde(default = "default_play_delete_sound")]
    pub play_delete_sound: bool,

    /// Playback volume for app cue sounds (`0.0`–`1.0`).
    #[serde(default = "default_sound_volume")]
    pub sound_volume: f32,

    /// Custom macro-finished cue file (empty = built-in).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub finish_sound_file: String,

    /// Custom add-item cue file (empty = built-in).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub add_sound_file: String,

    /// Custom delete-item cue file (empty = built-in).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub delete_sound_file: String,
}

fn default_play_finish_sound() -> bool {
    DEFAULT_PLAY_FINISH_SOUND
}
fn default_play_add_sound() -> bool {
    true
}
fn default_play_delete_sound() -> bool {
    true
}
fn default_sound_volume() -> f32 {
    DEFAULT_SOUND_VOLUME
}

impl Default for SoundSettings {
    fn default() -> Self {
        Self {
            play_finish_sound: DEFAULT_PLAY_FINISH_SOUND,
            play_add_sound: true,
            play_delete_sound: true,
            sound_volume: DEFAULT_SOUND_VOLUME,
            finish_sound_file: String::new(),
            add_sound_file: String::new(),
            delete_sound_file: String::new(),
        }
    }
}

impl SoundSettings {
    /// Clamp numeric ranges to settings UI bounds.
    pub fn clamp(&mut self) {
        if !self.sound_volume.is_finite() {
            self.sound_volume = DEFAULT_SOUND_VOLUME;
        }
        self.sound_volume = self.sound_volume.clamp(0.0, 1.0);
    }
}
