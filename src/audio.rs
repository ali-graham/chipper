use anyhow::Result;
use sdl3::audio::AudioCallback;
use sdl3::audio::AudioDevice;
use sdl3::audio::AudioSpec;
use sdl3::audio::AudioStream;
use sdl3::audio::AudioStreamWithCallback;
use sdl3::AudioSubsystem;

#[must_use]
struct SquareWave {
    phase_inc: f32,
    phase: f32,
    volume: f32,
    buffer: Vec<f32>,
}

impl AudioCallback<f32> for SquareWave {
    fn callback(&mut self, stream: &mut AudioStream, requested: i32) {
        self.buffer
            .resize(usize::try_from(requested).expect("bad data"), 0.0);

        // Generate a square wave
        for x in &mut self.buffer {
            *x = if self.phase <= 0.5 {
                self.volume
            } else {
                -self.volume
            };
            self.phase = (self.phase + self.phase_inc) % 1.0;
        }

        stream.put_data_f32(&self.buffer).unwrap();
    }
}

#[must_use]
pub(super) struct Audio {
    _subsystem: AudioSubsystem, // see https://github.com/vhspace/sdl3-rs/issues/79
    _device: AudioDevice,
    stream: AudioStreamWithCallback<SquareWave>,
    is_playing: bool,
}

// impl Drop for Audio {
//     fn drop(&mut self) {
//         eprintln!("{0:?}", self._subsystem);
//     }
// }

impl Audio {
    pub(super) fn new(context: &sdl3::Sdl) -> Result<Self> {
        let subsystem = context.audio()?;

        let desired_spec = AudioSpec {
            freq: Some(44_100),
            channels: Some(1), // mono
            format: Some(sdl3::audio::AudioFormat::F32LE),
        };

        let device = subsystem.open_playback_device(&desired_spec)?;
        let stream = device.open_playback_stream_with_callback(
            &desired_spec,
            SquareWave {
                phase_inc: 440.0 / 44_100.0,
                phase: 0.0,
                volume: 0.25,
                buffer: Vec::new(),
            },
        )?;

        Ok(Audio {
            _subsystem: subsystem,
            _device: device,
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
