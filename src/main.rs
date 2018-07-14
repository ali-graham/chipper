extern crate sdl2;

// use std::process;
use sdl2::event::{Event};
use sdl2::pixels;
use sdl2::keyboard::Keycode;

use std::io::prelude::*;
use std::fs::File;

const SCREEN_WIDTH: u32 = 64;
const SCREEN_HEIGHT: u32 = 32;

mod chip8;

fn main() {

    // get ROM filename from command line

    // set up graphics (SDL)

    // set up input (SDL)

    let sdl_context = sdl2::init().unwrap();

    let video_subsys = sdl_context.video().unwrap();
    let window = video_subsys.window("chipper", SCREEN_WIDTH, SCREEN_HEIGHT)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();

    canvas.set_draw_color(pixels::Color::RGB(0, 0, 0));
    canvas.clear();
    canvas.present();

    let mut events = sdl_context.event_pump().unwrap();

    let mut chip8: chip8::Chip8 = Default::default();

    // FIXME: get filename from CLI argument

    let mut f = File::open("programs/Chip8 Picture.ch8").unwrap();
    let mut rom_data = Vec::new();
    f.read_to_end(&mut rom_data).unwrap();

    chip8.initialize();
    chip8.load_rom(&rom_data);

    let mut main_loop = || {

        chip8.emulate_cycle();

        for event in events.poll_iter() {
            match event {
                Event::Quit {..} | Event::KeyDown {keycode: Some(Keycode::Escape), ..} => {
                    // process::exit(1);
                },
                Event::KeyDown { keycode: Some(Keycode::Left), ..} => {
                    // rect.x -= 10;
                },
                Event::KeyDown { keycode: Some(Keycode::Right), ..} => {
                    // rect.x += 10;
                },
                Event::KeyDown { keycode: Some(Keycode::Up), ..} => {
                    // rect.y -= 10;
                },
                Event::KeyDown { keycode: Some(Keycode::Down), ..} => {
                    // rect.y += 10;
                },
                _ => {}
            }
        }

    };

    loop { main_loop(); }
}
