#![forbid(unsafe_code)]

#[macro_use]
extern crate clap;

extern crate sdl2;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels;
use sdl2::rect::Rect;
use std::time::{Duration, Instant};
use std::{process, thread};

use std::fs::File;
use std::io::prelude::*;

mod audio;
mod chip8;

const DEFAULT_DISPLAY_SCALE: u8 = 12;

fn main() {
    let matches = clap::App::new("chipper")
        .version("0.1.0")
        .author("Ali Graham <ali.graham@gmail.com>")
        .about("Simple CHIP-8 emulator")
        .args_from_usage(
            "-f, --file=[file]   'ROM filename to load'
             -s, --scale=[scale] 'Scale factor for the window'
             -l, --legacy        'Use older shift opcodes'",
        )
        .get_matches();

    let rom_filename = matches.value_of("file").expect("No ROM filename provided");
    let scale = value_t!(matches, "scale", u8).unwrap_or(DEFAULT_DISPLAY_SCALE);
    let legacy_mode = matches.is_present("legacy");

    let sdl_context = sdl2::init().unwrap();

    let video_subsys = sdl_context.video().unwrap();
    let window = video_subsys
        .window(
            "chipper",
            (chip8::SCREEN_WIDTH as u32) * (scale as u32),
            (chip8::SCREEN_HEIGHT as u32) * (scale as u32),
        )
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();

    canvas.set_draw_color(pixels::Color::RGB(0, 0, 0));
    canvas.clear();
    canvas.present();

    let mut events = sdl_context.event_pump().unwrap();

    let mut chip8: chip8::Chip8 = Default::default();

    let mut f = File::open(rom_filename).unwrap();
    let mut rom_data = Vec::new();
    f.read_to_end(&mut rom_data).unwrap();

    chip8.initialize();
    let mut audio = audio::Audio::new(&sdl_context);

    chip8.load_rom(&rom_data);

    // unnecessary init, but compiler complains
    let mut start = Instant::now();
    let tick = Duration::from_millis(1000 / 60);

    let mut main_loop = || {
        start = Instant::now();

        chip8.emulate_cycle(legacy_mode);

        if chip8.graphics_needs_refresh() {
            for yline in 0..chip8::SCREEN_HEIGHT {
                for xline in 0..chip8::SCREEN_WIDTH {
                    if chip8.gfx
                        [((yline as u16 * chip8::SCREEN_WIDTH as u16) + xline as u16) as usize]
                        == 1
                    {
                        canvas.set_draw_color(pixels::Color::RGB(255, 255, 255));
                    } else {
                        canvas.set_draw_color(pixels::Color::RGB(0, 0, 0));
                    }
                    let r = Rect::new(
                        xline as i32 * scale as i32,
                        yline as i32 * scale as i32,
                        scale as u32,
                        scale as u32,
                    );
                    canvas.fill_rect(r).unwrap();
                }
            }
            canvas.present();
            chip8.graphics_clear_refresh();
        }

        if chip8.audio_sound() {
            if audio.paused() {
                audio.play();
            }
        } else if audio.playing() {
            audio.pause();
        }

        for event in events.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    process::exit(1);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Num1),
                    repeat: false,
                    ..
                } => {
                    chip8.key_down(0x1);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Num2),
                    repeat: false,
                    ..
                } => {
                    chip8.key_down(0x2);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Num3),
                    repeat: false,
                    ..
                } => {
                    chip8.key_down(0x3);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Num4),
                    repeat: false,
                    ..
                } => {
                    chip8.key_down(0xc);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Q),
                    repeat: false,
                    ..
                } => {
                    chip8.key_down(0x4);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::W),
                    repeat: false,
                    ..
                } => {
                    chip8.key_down(0x5);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::E),
                    repeat: false,
                    ..
                } => {
                    chip8.key_down(0x6);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::R),
                    repeat: false,
                    ..
                } => {
                    chip8.key_down(0xd);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::A),
                    repeat: false,
                    ..
                } => {
                    chip8.key_down(0x7);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::S),
                    repeat: false,
                    ..
                } => {
                    chip8.key_down(0x8);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::D),
                    repeat: false,
                    ..
                } => {
                    chip8.key_down(0x9);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::F),
                    repeat: false,
                    ..
                } => {
                    chip8.key_down(0xe);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Z),
                    repeat: false,
                    ..
                } => {
                    chip8.key_down(0xa);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::X),
                    repeat: false,
                    ..
                } => {
                    chip8.key_down(0x0);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::C),
                    repeat: false,
                    ..
                } => {
                    chip8.key_down(0xb);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::V),
                    repeat: false,
                    ..
                } => {
                    chip8.key_down(0xf);
                }
                Event::KeyUp {
                    keycode: Some(Keycode::Num1),
                    ..
                } => {
                    chip8.key_up(0x1);
                }
                Event::KeyUp {
                    keycode: Some(Keycode::Num2),
                    ..
                } => {
                    chip8.key_up(0x2);
                }
                Event::KeyUp {
                    keycode: Some(Keycode::Num3),
                    ..
                } => {
                    chip8.key_up(0x3);
                }
                Event::KeyUp {
                    keycode: Some(Keycode::Num4),
                    ..
                } => {
                    chip8.key_up(0xc);
                }
                Event::KeyUp {
                    keycode: Some(Keycode::Q),
                    ..
                } => {
                    chip8.key_up(0x4);
                }
                Event::KeyUp {
                    keycode: Some(Keycode::W),
                    ..
                } => {
                    chip8.key_up(0x5);
                }
                Event::KeyUp {
                    keycode: Some(Keycode::E),
                    ..
                } => {
                    chip8.key_up(0x6);
                }
                Event::KeyUp {
                    keycode: Some(Keycode::R),
                    ..
                } => {
                    chip8.key_up(0xd);
                }
                Event::KeyUp {
                    keycode: Some(Keycode::A),
                    ..
                } => {
                    chip8.key_up(0x7);
                }
                Event::KeyUp {
                    keycode: Some(Keycode::S),
                    ..
                } => {
                    chip8.key_up(0x8);
                }
                Event::KeyUp {
                    keycode: Some(Keycode::D),
                    ..
                } => {
                    chip8.key_up(0x9);
                }
                Event::KeyUp {
                    keycode: Some(Keycode::F),
                    ..
                } => {
                    chip8.key_up(0xe);
                }
                Event::KeyUp {
                    keycode: Some(Keycode::Z),
                    ..
                } => {
                    chip8.key_up(0xa);
                }
                Event::KeyUp {
                    keycode: Some(Keycode::X),
                    ..
                } => {
                    chip8.key_up(0x0);
                }
                Event::KeyUp {
                    keycode: Some(Keycode::C),
                    ..
                } => {
                    chip8.key_up(0xb);
                }
                Event::KeyUp {
                    keycode: Some(Keycode::V),
                    ..
                } => {
                    chip8.key_up(0xf);
                }
                _ => {}
            }
        }

        match tick.checked_sub(start.elapsed()) {
            Some(remaining) => thread::sleep(remaining),
            None => {}
        }
    };

    loop {
        main_loop();
    }
}
