//! Embedded cue sounds for the desktop shell.

/// Age of Empires I “under attack” sting — macro finish cue.
const FINISH_SOUND_MP3: &[u8] = include_bytes!("../assets/sounds/aoe1-under-attack.mp3");

/// Fire-and-forget playback of the macro finish sound on a background thread.
///
/// Failures (no audio device, decode errors) are ignored so run completion is never blocked.
pub fn play_finish_sound() {
    let _ = std::thread::Builder::new()
        .name("sqyre-finish-sound".into())
        .spawn(|| {
            let Ok(mut handle) = rodio::DeviceSinkBuilder::open_default_sink() else {
                return;
            };
            handle.log_on_drop(false);
            let cursor = std::io::Cursor::new(FINISH_SOUND_MP3);
            let Ok(player) = rodio::play(handle.mixer(), cursor) else {
                return;
            };
            player.sleep_until_end();
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finish_sound_embedded() {
        assert!(!FINISH_SOUND_MP3.is_empty());
        // MPEG frame sync / ID3 — either is a valid MP3 container start.
        assert!(
            FINISH_SOUND_MP3.starts_with(b"ID3")
                || (FINISH_SOUND_MP3.len() >= 2
                    && FINISH_SOUND_MP3[0] == 0xff
                    && FINISH_SOUND_MP3[1] & 0xe0 == 0xe0)
        );
    }
}
