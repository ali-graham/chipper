use anyhow::Error;
use anyhow::Result;
use sdl3::audio::AudioCallback;
use sdl3::audio::AudioDevice;
use sdl3::audio::AudioSpecDesired;
use sdl3::audio::AudioStatus;

#[must_use]
struct SquareWave {
    phase_inc: f64,
    phase: f64,
    volume: f32,
}

impl AudioCallback for SquareWave {
    type Channel = f32;

    fn callback(&mut self, out: &mut [f32]) {
        // Generate a square wave
        for x in out {
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
    device: AudioDevice<SquareWave>,
}

impl Audio {
    pub(super) fn new(context: &sdl3::Sdl) -> Result<Self> {
        let desired_spec = AudioSpecDesired {
            freq: Some(44_100),
            channels: Some(1), // mono
            samples: None,     // default sample size
        };

        Ok(Audio {
            device: context
                .audio()
                .map_err(Error::msg)?
                .open_playback(None, &desired_spec, |spec| -> SquareWave {
                    // initialize the audio callback
                    SquareWave {
                        phase_inc: 440.0 / f64::from(spec.freq),
                        phase: 0.0,
                        volume: 0.25,
                    }
                })
                .map_err(Error::msg)?,
        })
    }

    pub(super) fn play(&mut self) {
        self.device.resume();
    }

    pub(super) fn pause(&mut self) {
        self.device.pause();
    }

    pub(super) fn playing(&mut self) -> bool {
        self.device.status() == AudioStatus::Playing
    }

    pub(super) fn paused(&mut self) -> bool {
        self.device.status() == AudioStatus::Paused
    }
}
