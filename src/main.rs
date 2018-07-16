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

mod chip8;

const DEFAULT_DISPLAY_SCALE: u32 = 12;

fn main() {
    let matches = clap::App::new("chipper")
        .version("0.1.0")
        .author("Ali Graham <ali.graham@gmail.com>")
        .about("Simple CHIP-8 emulator")
        .args_from_usage(
            "-f, --file=[file]   'ROM filename to load'
             -s, --scale=[scale] 'Scale factor for the window'",
        )
        .get_matches();

    let rom_filename = matches.value_of("file").expect("No ROM filename provided");
    let scale = value_t!(matches, "scale", u32).unwrap_or(DEFAULT_DISPLAY_SCALE);

    let display_width = chip8::SCREEN_WIDTH * scale;
    let display_height = chip8::SCREEN_HEIGHT * scale;

    let sdl_context = sdl2::init().unwrap();

    let video_subsys = sdl_context.video().unwrap();
    let window = video_subsys
        .window("chipper", display_width, display_height)
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
    chip8.load_rom(&rom_data);

    // unnecessary init, but compiler complains
    let mut start = Instant::now();
    let tick = Duration::from_millis(1000 / 60);

    let mut main_loop = || {
        start = Instant::now();

        chip8.emulate_cycle();

        if chip8.graphics_needs_refresh() {
            for yline in 0..chip8::SCREEN_HEIGHT {
                for xline in 0..chip8::SCREEN_WIDTH {
                    if chip8.gfx[((yline * chip8::SCREEN_WIDTH) + xline) as usize] == 1 {
                        canvas.set_draw_color(pixels::Color::RGB(255, 255, 255));
                    } else {
                        canvas.set_draw_color(pixels::Color::RGB(0, 0, 0));
                    }
                    let r = Rect::new((xline * scale) as i32, (yline * scale) as i32, scale, scale);
                    canvas.fill_rect(r).unwrap();
                }
            }
            canvas.present();
            chip8.graphics_clear_refresh();
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
                // Event::KeyDown { keycode: Some(Keycode::Left), ..} => {

                // },
                // Event::KeyDown { keycode: Some(Keycode::Right), ..} => {

                // },
                // Event::KeyDown { keycode: Some(Keycode::Up), ..} => {

                // },
                // Event::KeyDown { keycode: Some(Keycode::Down), ..} => {

                // },
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
