#![forbid(unsafe_code)]
#![deny(clippy::pedantic)]

use anyhow::Result;
use clap::value_parser;
use clap::Parser;
use clap::ValueEnum;

mod audio;
mod chip8;
mod emulator;
mod hardware;
mod profile;
mod util;

#[derive(ValueEnum, Debug, Copy, Clone)]
pub(crate) enum ProcessType {
    // Step,
    Run,
}

#[derive(ValueEnum, Debug, Eq, PartialEq, Hash, Copy, Clone)]
pub(crate) enum Target {
    Chip8,
    SuperChip,
    XoChip,
}

#[derive(ValueEnum, Debug, Eq, PartialEq, Hash, Copy, Clone)]
pub(crate) enum KeyMapping {
    Qwerty,
    Colemak,
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub(crate) enum Action {
    Quit,
}

/// Simple CHIP-8 emulator
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Scale factor for the window
    #[clap(short, long, value_parser = value_parser!(u8).range(1..64))]
    scale: Option<u8>,

    /// Keyboard layout to use
    #[clap(short, long, value_enum, default_value_t = KeyMapping::Qwerty)]
    key_mapping: KeyMapping,

    /// Target architecture to emulate
    #[clap(short, long, value_enum, default_value_t = Target::Chip8)]
    target: Target,

    /// How emulator cycles will be executed
    #[clap(short, long, value_enum, default_value_t = ProcessType::Run)]
    process_type: ProcessType,

    /// ROM filename to load
    #[clap(short, long, value_parser)]
    file: String,
}

fn main() -> Result<()> {
    let args = Args::parse();

    emulator::Emulator::new(args.scale, args.key_mapping, args.target)
        .and_then(|mut e| e.process(args.process_type, &args.file))
}
