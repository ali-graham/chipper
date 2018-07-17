extern crate sdl2;

use sdl2::AudioSubsystem;
use sdl2::audio::{AudioCallback, AudioSpecDesired};

struct SquareWave {
    phase_inc: f32,
    phase: f32,
    volume: f32
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
    subsystem: Result<AudioSubsystem, String>,
    desired_spec: AudioSpecDesired
}

impl Default for Audio {
    fn default() -> Audio {
        Audio {
            subsystem: Err("Not initialized".to_string()),
            desired_spec: AudioSpecDesired {
                freq: Some(44100),
                channels: Some(1),  // mono
                samples: None       // default sample size
            }
        }
    }
}

impl Audio {
    pub fn initialize(&mut self, context: &sdl2::Sdl) {
        self.subsystem = context.audio();

        match self.subsystem {
            Ok(ref _ss) => {

            },
            Err(ref s) => println!("{}", s)
        }
    }
}