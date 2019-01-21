extern crate sdl2;

use sdl2::audio::{AudioCallback, AudioDevice, AudioSpecDesired, AudioStatus};

struct SquareWave {
    phase_inc: f32,
    phase: f32,
    volume: f32,
}

impl AudioCallback for SquareWave {
    type Channel = f32;

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

pub struct Audio {
    device: AudioDevice<SquareWave>,
}

impl Audio {
    pub fn new(context: &sdl2::Sdl) -> Audio {
        let desired_spec = AudioSpecDesired {
            freq: Some(44100),
            channels: Some(1), // mono
            samples: None,     // default sample size
        };

        Audio {
            device: context
                .audio()
                .unwrap()
                .open_playback(None, &desired_spec, |spec| {
                    // initialize the audio callback
                    SquareWave {
                        phase_inc: 440.0 / spec.freq as f32,
                        phase: 0.0,
                        volume: 0.25,
                    }
                })
                .unwrap(),
        }
    }

    pub fn play(&mut self) {
        self.device.resume();
    }

    pub fn pause(&mut self) {
        self.device.pause();
    }

    pub fn playing(&mut self) -> bool {
        self.device.status() == AudioStatus::Playing
    }

    pub fn paused(&mut self) -> bool {
        self.device.status() == AudioStatus::Paused
    }
}
