use std::fs::File;
use std::io::Read;
use std::thread;
use std::time::Duration;
use std::time::Instant;

use anyhow::Context;
use anyhow::Result;

use crate::chip8;
use crate::hardware::Hardware;
use crate::profile::profiles;
use crate::profile::Profile;
use crate::Action;
use crate::ProcessType;
use crate::Target;

const TICK: Duration = Duration::from_millis(1_000 / 60);

type Ticker = Box<dyn FnMut(&mut Emulator) -> Option<Action>>;

pub(super) struct Emulator {
    profile: Profile,
    hardware: Hardware,
    chip8: chip8::Chip8,
}

impl Emulator {
    pub(super) fn new(scale: Option<u8>, target: Target) -> Result<Self> {
        let profile: Profile = *profiles()
            .get(&target)
            .context("Unknown target architecture")?;

        let hardware = Hardware::new(scale, profile)?;

        let chip8 = chip8::Chip8::new(target, profile, Box::new(rand::rng()));

        Ok(Emulator {
            profile,
            hardware,
            chip8,
        })
    }

    pub(super) fn process(&mut self, process_type: ProcessType, filename: &str) -> Result<()> {
        let rom_data = Self::load_file(filename)?;

        self.chip8.load_rom(&rom_data);

        let mut ticker: Ticker = match process_type {
            ProcessType::Run => Box::new(Self::tick_run),
            // ProcessType::Step => Box::new(Self::tick_step),
        };

        loop {
            // we always want a refresh after a tick, even if about to quit
            if matches!(ticker(self), Some(Action::Quit))
                | matches!(self.refresh()?, Some(Action::Quit))
            {
                break;
            }
        }

        Ok(())
    }

    fn load_file(filename: &str) -> Result<Vec<u8>> {
        let mut rom_data = Vec::new();
        {
            let mut f = File::open(filename)?;
            f.read_to_end(&mut rom_data)?;
        }
        Ok(rom_data)
    }

    fn tick_run(&mut self) -> Option<Action> {
        let start = Instant::now();
        let mut remaining = Duration::new(0, 0);

        for _cycles in 0u8..20u8 {
            self.chip8.emulate_cycle();

            remaining = TICK.saturating_sub(start.elapsed());

            if ((self.profile.lores_display_wait() && !self.chip8.hires_mode())
                && self.chip8.graphics_needs_refresh())
                || remaining.is_zero()
            {
                break;
            }
        }

        if !remaining.is_zero() {
            thread::sleep(remaining);
        }

        None
    }

    // fn tick_step(&mut self) -> Option<Action> {
    //     for _cycles in 0u8..8u8 {
    //         // actually 83 cycles / 10 ticks
    //         self.chip8.emulate_cycle();
    //         // TODO block waiting for ADVANCE event
    //     }
    //
    //     None
    // }

    fn refresh(&mut self) -> Result<Option<Action>> {
        self.chip8.update_timers();

        if self.chip8.graphics_needs_refresh() {
            self.hardware
                .refresh_graphics(self.chip8.graphics(), self.chip8.resolution_scale())?;
            self.chip8.graphics_clear_refresh();
        }

        if self.chip8.audio_sound() {
            self.hardware.sound_start();
        } else {
            self.hardware.sound_stop();
        }

        self.hardware
            .event_iter()
            .find_map(|event| self.chip8.handle_key(&event).transpose())
            .transpose()
    }
}
