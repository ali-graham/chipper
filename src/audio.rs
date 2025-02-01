use anyhow::Error;
use anyhow::Result;
use sdl3::audio::AudioCallback;
use sdl3::audio::AudioSpec;
use sdl3::audio::AudioStreamWithCallback;
use sdl3::AudioSubsystem;

#[must_use]
struct SquareWave {
    phase_inc: f32,
    phase: f32,
    volume: f32,
}

impl AudioCallback<f32> for SquareWave {
    fn callback(&mut self, out: &mut [f32]) {
        // Generate a square wave
        for x in out.iter_mut() {
            *x = if self.phase <= 0.5 {
                self.volume
            } else {
                -self.volume
            };
            self.phase = (self.phase + self.phase_inc) % 1.0;
        }
    }
}

#[must_use]
pub(super) struct Audio {
    _audio_subsystem: AudioSubsystem, // see https://github.com/vhspace/sdl3-rs/issues/79
    stream: AudioStreamWithCallback<SquareWave>,
    is_playing: bool,
}

impl Audio {
    pub(super) fn new(context: &sdl3::Sdl) -> Result<Self> {
        let audio_subsystem = context.audio()?;

        let desired_spec = AudioSpec {
            freq: Some(44_100),
            channels: Some(1), // mono
            format: Some(sdl3::audio::AudioFormat::F32LE),
        };

        let stream = audio_subsystem
            .open_playback_stream(
                &desired_spec,
                // initialize the audio callback
                SquareWave {
                    phase_inc: 440.0 / 44_100.0,
                    phase: 0.0,
                    volume: 0.25,
                },
            )
            .map_err(Error::msg)?;

        Ok(Audio {
            _audio_subsystem: audio_subsystem,
            stream,
            is_playing: false,
        })
    }

    pub(super) fn play(&mut self) {
        if !self.is_playing {
            self.stream.resume().expect("resume");
            self.is_playing = true;
        }
    }

    pub(super) fn pause(&mut self) {
        if self.is_playing {
            self.stream.pause().expect("pause");
            self.is_playing = false;
        }
    }

    // pub(super) fn playing(&mut self) -> bool {
    //     self...pause() == AudioStatus::Playing
    // }

    // pub(super) fn paused(&mut self) -> bool {
    //     self.stream.status() == AudioStatus::Paused
    // }
}
