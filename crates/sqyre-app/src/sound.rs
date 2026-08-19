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
        .spawn(move || play_mp3_blocking(bytes, volume));
}

#[cfg(target_os = "linux")]
fn play_mp3_blocking(bytes: &'static [u8], volume: f32) {
    linux_pulse::play(bytes, volume);
}

#[cfg(not(target_os = "linux"))]
fn play_mp3_blocking(bytes: &'static [u8], volume: f32) {
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

/// PulseAudio (including PipeWire's pulse socket) instead of ALSA.
///
/// cpal's ALSA host loads `libasound_module_pcm_pipewire.so` from the library's
/// compile-time plugin dir (Debian path in Ubuntu-built AppImages) and then
/// hits `get_htstamp` / `get_trigger_htstamp` errors on that plugin.
#[cfg(target_os = "linux")]
mod linux_pulse {
    use std::num::{NonZeroU16, NonZeroU32};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use cpal::{FromSample, SampleFormat, SizedSample, StreamConfig};

    pub(super) fn play(bytes: &'static [u8], volume: f32) {
        let Ok(host) = cpal::host_from_id(cpal::HostId::PulseAudio) else {
            return;
        };
        let Some(device) = host.default_output_device() else {
            return;
        };
        let Ok(supported) = device.default_output_config() else {
            return;
        };
        let sample_format = supported.sample_format();
        let config: StreamConfig = supported.into();

        let Some(channels) = NonZeroU16::new(config.channels) else {
            return;
        };
        let Some(sample_rate) = NonZeroU32::new(config.sample_rate) else {
            return;
        };

        let cursor = std::io::Cursor::new(bytes);
        let Ok(decoder) = rodio::Decoder::new(cursor) else {
            return;
        };
        let samples: Vec<f32> =
            rodio::source::UniformSourceIterator::new(decoder, channels, sample_rate)
                .map(|s| s * volume)
                .collect();
        if samples.is_empty() {
            return;
        }

        match sample_format {
            SampleFormat::F32 => run::<f32>(&device, &config, samples),
            SampleFormat::I16 => run::<i16>(&device, &config, samples),
            SampleFormat::I32 => run::<i32>(&device, &config, samples),
            SampleFormat::U8 => run::<u8>(&device, &config, samples),
            _ => {}
        }
    }

    fn run<T: SizedSample + FromSample<f32>>(
        device: &cpal::Device,
        config: &StreamConfig,
        samples: Vec<f32>,
    ) {
        let n = samples.len();
        let pos = Arc::new(AtomicUsize::new(0));
        let (done_tx, done_rx) = mpsc::channel::<()>();
        let done = Arc::new(Mutex::new(Some(done_tx)));
        let samples = Arc::new(samples);

        let pos_cb = Arc::clone(&pos);
        let samples_cb = Arc::clone(&samples);
        let done_cb = Arc::clone(&done);
        let Ok(stream) = device.build_output_stream(
            *config,
            move |out: &mut [T], _| {
                for slot in out.iter_mut() {
                    let i = pos_cb.fetch_add(1, Ordering::Relaxed);
                    *slot = if i < samples_cb.len() {
                        T::from_sample(samples_cb[i])
                    } else {
                        T::EQUILIBRIUM
                    };
                }
                if pos_cb.load(Ordering::Relaxed) >= n {
                    signal_done(&done_cb);
                }
            },
            {
                let done_err = Arc::clone(&done);
                move |_| signal_done(&done_err)
            },
            None,
        ) else {
            return;
        };

        if stream.play().is_err() {
            return;
        }

        let frames =
            (n as f64) / (f64::from(config.sample_rate) * f64::from(config.channels).max(1.0));
        let _ = done_rx.recv_timeout(Duration::from_secs_f64(frames + 1.0));
        drop(stream);
    }

    fn signal_done(done: &Mutex<Option<mpsc::Sender<()>>>) {
        if let Ok(mut guard) = done.lock() {
            if let Some(tx) = guard.take() {
                let _ = tx.send(());
            }
        }
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
