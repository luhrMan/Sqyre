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
///
/// Streams are always F32: PipeWire sink defaults are often S24 (`I24`), which
/// `SizedSample` does not expose as `i32` and was previously skipped.
#[cfg(target_os = "linux")]
mod linux_pulse {
    use std::num::{NonZeroU16, NonZeroU32};
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::mpsc::{self, Receiver, SyncSender};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use cpal::{FromSample, SizedSample, StreamConfig};
    use sqyre_capture::{cap_log, mark_site};

    struct Voice {
        samples: Vec<f32>,
        pos: usize,
    }

    struct Mixer {
        tx: SyncSender<Vec<f32>>,
        generation: u64,
        channels: NonZeroU16,
        sample_rate: NonZeroU32,
    }

    /// One Pulse stream for the process. Overlapping cues mix in software;
    /// a second cpal stream was `No_such_entity` (stderr) on pipewire-pulse.
    static MIXER: Mutex<Option<Mixer>> = Mutex::new(None);
    static NEXT_GEN: AtomicU64 = AtomicU64::new(1);

    pub(super) fn play(bytes: &'static [u8], volume: f32) {
        if let Err(stage) = play_inner(bytes, volume) {
            cap_log("SOUND", "fail", &format!("stage={stage}"));
            crate::log::warn(format!("cue sound skipped ({stage})"));
        }
    }

    fn play_inner(bytes: &'static [u8], volume: f32) -> Result<(), &'static str> {
        let mixer = mixer()?;
        mark_site("sound:play:before_decode");
        let decoder = rodio::Decoder::new(std::io::Cursor::new(bytes)).map_err(|_| "decode")?;
        let samples: Vec<f32> =
            rodio::source::UniformSourceIterator::new(decoder, mixer.channels, mixer.sample_rate)
                .map(|s| s * volume)
                .collect();
        if samples.is_empty() {
            return Err("empty-decode");
        }
        let n = samples.len();
        submit(mixer, samples)?;
        cap_log("SOUND", "ok", &format!("mix samples={n}"));
        Ok(())
    }

    fn submit(handle: Mixer, samples: Vec<f32>) -> Result<(), &'static str> {
        if let Err(failed) = handle.tx.send(samples) {
            clear_mixer(handle.generation);
            mixer()?.tx.send(failed.0).map_err(|_| "mixer-gone")?;
        }
        Ok(())
    }

    fn clear_mixer(generation: u64) {
        let mut guard = MIXER.lock().unwrap_or_else(|e| e.into_inner());
        if guard.as_ref().is_some_and(|m| m.generation == generation) {
            *guard = None;
        }
    }

    fn mixer() -> Result<Mixer, &'static str> {
        let mut guard = MIXER.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(existing) = guard.as_ref() {
            return Ok(Mixer {
                tx: existing.tx.clone(),
                generation: existing.generation,
                channels: existing.channels,
                sample_rate: existing.sample_rate,
            });
        }
        let started = start_mixer()?;
        *guard = Some(Mixer {
            tx: started.tx.clone(),
            generation: started.generation,
            channels: started.channels,
            sample_rate: started.sample_rate,
        });
        Ok(started)
    }

    fn start_mixer() -> Result<Mixer, &'static str> {
        let generation = NEXT_GEN.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = mpsc::sync_channel::<Vec<f32>>(16);
        let (ready_tx, ready_rx) = mpsc::channel();
        std::thread::Builder::new()
            .name("sqyre-sound".into())
            .spawn(move || mixer_thread(generation, rx, ready_tx))
            .map_err(|_| "spawn")?;
        match ready_rx.recv_timeout(Duration::from_secs(5)) {
            Ok(Ok((channels, sample_rate))) => Ok(Mixer {
                tx,
                generation,
                channels,
                sample_rate,
            }),
            Ok(Err(stage)) => Err(stage),
            Err(_) => Err("mixer-timeout"),
        }
    }

    fn mixer_thread(
        generation: u64,
        rx: Receiver<Vec<f32>>,
        ready_tx: mpsc::Sender<Result<(NonZeroU16, NonZeroU32), &'static str>>,
    ) {
        if let Err(stage) = mixer_thread_inner(rx, &ready_tx) {
            cap_log("SOUND", "fail", &format!("stage={stage}"));
            let _ = ready_tx.send(Err(stage));
        }
        clear_mixer(generation);
    }

    fn mixer_thread_inner(
        rx: Receiver<Vec<f32>>,
        ready_tx: &mpsc::Sender<Result<(NonZeroU16, NonZeroU32), &'static str>>,
    ) -> Result<(), &'static str> {
        mark_site("sound:play:before_host");
        let host = cpal::host_from_id(cpal::HostId::PulseAudio).map_err(|e| {
            cap_log(
                "SOUND",
                "fail",
                &format!("stage=host error={}", slug(&e.to_string())),
            );
            "pulse-host"
        })?;
        let device = output_device(&host).ok_or("no-output-device")?;
        let (channels, sample_rate) = output_layout(&device)?;
        // `BufferSize::Default` is `u32::MAX` on the Pulse wire protocol;
        // pipewire-pulse then asks for ~2s in one callback (cpal #1190).
        let period_frames = (sample_rate.get() / 10).max(256);
        let config = StreamConfig {
            channels: channels.get(),
            sample_rate: sample_rate.get(),
            buffer_size: cpal::BufferSize::Fixed(period_frames),
        };
        let voices = Arc::new(Mutex::new(Vec::<Voice>::new()));
        let failed = Arc::new(AtomicBool::new(false));
        let voices_cb = Arc::clone(&voices);
        mark_site("sound:play:before_stream");
        let stream = device
            .build_output_stream(
                config,
                move |out: &mut [f32], _| {
                    let Ok(mut voices) = voices_cb.lock() else {
                        return;
                    };
                    mix_into(out, &mut voices);
                },
                {
                    let failed = Arc::clone(&failed);
                    move |err| {
                        cap_log(
                            "SOUND",
                            "fail",
                            &format!("stage=callback error={}", slug(&err.to_string())),
                        );
                        failed.store(true, Ordering::Relaxed);
                    }
                },
                None,
            )
            .map_err(|e| {
                cap_log(
                    "SOUND",
                    "fail",
                    &format!("stage=stream error={}", slug(&e.to_string())),
                );
                "stream"
            })?;
        stream.play().map_err(|_| "play")?;
        cap_log(
            "SOUND",
            "ok",
            &format!(
                "host=pulse mixer=start rate={} ch={} period={period_frames}",
                config.sample_rate, config.channels
            ),
        );
        ready_tx
            .send(Ok((channels, sample_rate)))
            .map_err(|_| "ready")?;

        loop {
            if failed.load(Ordering::Relaxed) {
                break;
            }
            match rx.recv_timeout(Duration::from_millis(200)) {
                Ok(samples) => {
                    if let Ok(mut list) = voices.lock() {
                        list.push(Voice { samples, pos: 0 });
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        drop(stream);
        mark_site("sound:play:done");
        Ok(())
    }

    fn mix_into<T: SizedSample + FromSample<f32>>(out: &mut [T], voices: &mut Vec<Voice>) {
        for slot in out.iter_mut() {
            let mut mixed = 0.0;
            for voice in voices.iter_mut() {
                if let Some(&sample) = voice.samples.get(voice.pos) {
                    mixed += sample;
                    voice.pos += 1;
                }
            }
            *slot = T::from_sample(mixed.clamp(-1.0, 1.0));
        }
        voices.retain(|voice| voice.pos < voice.samples.len());
    }

    fn output_device(host: &cpal::Host) -> Option<cpal::Device> {
        if let Some(device) = host.default_output_device() {
            return Some(device);
        }
        host.devices()
            .ok()?
            .find(|d| d.default_output_config().is_ok())
    }

    fn output_layout(device: &cpal::Device) -> Result<(NonZeroU16, NonZeroU32), &'static str> {
        if let Ok(supported) = device.default_output_config() {
            let channels = NonZeroU16::new(supported.channels()).ok_or("bad-channels")?;
            let sample_rate = NonZeroU32::new(supported.sample_rate()).ok_or("bad-rate")?;
            return Ok((channels, sample_rate));
        }
        Ok((
            NonZeroU16::new(2).expect("2"),
            NonZeroU32::new(48_000).expect("48000"),
        ))
    }

    fn slug(s: &str) -> String {
        s.chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '.' {
                    c
                } else {
                    '_'
                }
            })
            .collect()
    }

    #[cfg(test)]
    mod mix_tests {
        use super::*;

        #[test]
        fn overlapping_voices_sum() {
            let mut voices = vec![
                Voice {
                    samples: vec![0.2, 0.2],
                    pos: 0,
                },
                Voice {
                    samples: vec![0.3],
                    pos: 0,
                },
            ];
            let mut out = [0.0f32; 2];
            mix_into(&mut out, &mut voices);
            assert!((out[0] - 0.5).abs() < f32::EPSILON);
            assert!((out[1] - 0.2).abs() < f32::EPSILON);
            assert!(voices.is_empty());
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

    #[test]
    fn finish_sound_decodes_samples() {
        let n = rodio::Decoder::new(std::io::Cursor::new(FINISH_SOUND_MP3))
            .expect("mp3 decode")
            .count();
        assert!(n > 1000, "decoded {n} samples");
    }
}
