//! Embedded cue sounds for the desktop shell.

/// Age of Empires I “under attack” sting — macro finish cue.
const FINISH_SOUND_MP3: &[u8] = include_bytes!("../assets/sounds/aoe1-under-attack.mp3");

/// Cue when the user adds a macro, action, or catalog entity.
const ADD_SOUND_MP3: &[u8] = include_bytes!("../assets/sounds/shhh-ho.mp3");

/// Cue when the user deletes a macro, action, or catalog entity.
const DELETE_SOUND_MP3: &[u8] = include_bytes!("../assets/sounds/death.mp3");

fn play_mp3(bytes: &'static [u8], thread_name: &str, volume: f32) {
    let volume = volume.clamp(0.0, 1.0);
    if volume <= 0.0 {
        return;
    }
    let _ = std::thread::Builder::new()
        .name(thread_name.into())
        .spawn(move || {
            let Ok(mut handle) = rodio::DeviceSinkBuilder::open_default_sink() else {
                return;
            };
            handle.log_on_drop(false);
            let cursor = std::io::Cursor::new(bytes);
            let Ok(player) = rodio::play(handle.mixer(), cursor) else {
                return;
            };
            player.set_volume(volume);
            player.sleep_until_end();
        });
}

/// Fire-and-forget playback of the macro finish sound on a background thread.
///
/// Failures (no audio device, decode errors) are ignored so run completion is never blocked.
pub fn play_finish_sound(volume: f32) {
    play_mp3(FINISH_SOUND_MP3, "sqyre-finish-sound", volume);
}

/// Fire-and-forget playback of the UI “added” cue.
pub fn play_add_sound(volume: f32) {
    play_mp3(ADD_SOUND_MP3, "sqyre-add-sound", volume);
}

/// Fire-and-forget playback of the UI “deleted” cue.
pub fn play_delete_sound(volume: f32) {
    play_mp3(DELETE_SOUND_MP3, "sqyre-delete-sound", volume);
}

/// Play the add cue when UI sounds are enabled in settings.
pub fn play_add_sound_if(enabled: bool, volume: f32) {
    if enabled {
        play_add_sound(volume);
    }
}

/// Play the delete cue when UI sounds are enabled in settings.
pub fn play_delete_sound_if(enabled: bool, volume: f32) {
    if enabled {
        play_delete_sound(volume);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_mp3(bytes: &[u8]) {
        assert!(!bytes.is_empty());
        // MPEG frame sync / ID3 — either is a valid MP3 container start.
        assert!(
            bytes.starts_with(b"ID3")
                || (bytes.len() >= 2 && bytes[0] == 0xff && bytes[1] & 0xe0 == 0xe0)
        );
    }

    #[test]
    fn finish_sound_embedded() {
        assert_mp3(FINISH_SOUND_MP3);
    }

    #[test]
    fn add_sound_embedded() {
        assert_mp3(ADD_SOUND_MP3);
    }

    #[test]
    fn delete_sound_embedded() {
        assert_mp3(DELETE_SOUND_MP3);
    }
}
