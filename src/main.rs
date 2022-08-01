#![forbid(unsafe_code)]
#![deny(clippy::pedantic)]

use anyhow::Result;
use clap::Parser;

use std::fs::File;
use std::io::Read;
use std::thread;
use std::time::{Duration, Instant};

use crate::chip8::Chip8;
use crate::hardware::Hardware;

mod audio;
mod chip8;
mod hardware;

const TICK: Duration = Duration::from_millis(1000 / 60);

/// Simple CHIP-8 emulator
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// ROM filename to load
    #[clap(short, long, value_parser)]
    file: String,

    /// Scale factor for the window
    #[clap(short, long, value_parser, default_value_t = hardware::DEFAULT_DISPLAY_SCALE)]
    scale: u8,

    /// Use older shift opcodes
    #[clap(short, long, value_parser, default_value_t = false)]
    legacy: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let rom_data = load_file(&args.file)?;

    let mut chip8: Chip8 = Chip8::new(args.legacy);

    chip8.load_rom(&rom_data);

    let mut hardware = Hardware::new(args.scale)?;

    main_loop(&mut hardware, &mut chip8)?;

    Ok(())
}

fn load_file(filename: &str) -> Result<Vec<u8>> {
    let mut f = File::open(filename)?;
    let mut rom_data = Vec::new();
    f.read_to_end(&mut rom_data)?;
    Ok(rom_data)
}

fn main_loop(hardware: &mut hardware::Hardware, chip8: &mut chip8::Chip8) -> Result<()> {
    let mut start: Instant;
    let mut cycles: i32;

    'outer: loop {
        start = Instant::now();
        cycles = 0;

        'inner: loop {
            chip8.emulate_cycle();
            cycles += 1;

            let remaining = TICK.saturating_sub(start.elapsed());

            if remaining.is_zero() {
                break 'inner;
            } else if cycles >= 8 {
                thread::sleep(remaining);
                break 'inner;
            }
        }

        chip8.update_timers();

        if chip8.graphics_needs_refresh() {
            hardware.refresh_graphics(&chip8.gfx)?;
            chip8.graphics_clear_refresh();
        }

        hardware.do_sound(chip8.audio_sound());

        for event in hardware.event_iter() {
            if chip8.handle_key(&event)? {
                break 'outer;
            }
        }
    }

    Ok(())
}
