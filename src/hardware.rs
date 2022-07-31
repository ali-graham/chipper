use anyhow::{Error, Result};
use sdl2::event::EventPollIterator;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;
use sdl2::{EventPump, VideoSubsystem};

use crate::audio::Audio;
use crate::chip8;

pub const DEFAULT_DISPLAY_SCALE: u8 = 12;

const WHITE: Color = Color::RGB(255, 255, 255);
const BLACK: Color = Color::RGB(0, 0, 0);

pub struct Hardware {
    canvas: Canvas<Window>,
    scale: u8,
    audio: Audio,
    pub events: EventPump,
}

impl Hardware {
    pub fn new(scale: u8) -> Result<Self> {
        let sdl_context = sdl2::init().map_err(Error::msg)?;

        let video = sdl_context.video().map_err(Error::msg)?;

        Ok(Self {
            scale,
            canvas: Self::init_video(&video, scale)?,
            audio: Audio::new(&sdl_context),
            events: sdl_context.event_pump().map_err(Error::msg)?,
        })
    }

    fn init_video(video_subsys: &VideoSubsystem, scale: u8) -> Result<Canvas<Window>> {
        let window = video_subsys
            .window(
                "chipper",
                u32::from(chip8::SCREEN_WIDTH) * u32::from(scale),
                u32::from(chip8::SCREEN_HEIGHT) * u32::from(scale),
            )
            .position_centered()
            .build()
            .map_err(Error::new)?;

        let mut canvas = window
            .into_canvas()
            .present_vsync()
            .build()
            .map_err(Error::new)?;

        canvas.set_draw_color(BLACK);
        canvas.clear();
        canvas.present();
        Ok(canvas)
    }

    pub fn refresh_graphics(&mut self, gfx: &[u8]) -> Result<()> {
        let mut color: Color;
        let mut rect = Rect::new(0, 0, u32::from(self.scale), u32::from(self.scale));
        let sw = u16::from(chip8::SCREEN_WIDTH);
        let s = i32::from(self.scale);

        for yline in 0..u16::from(chip8::SCREEN_HEIGHT) {
            for xline in 0..sw {
                color = if gfx[((yline * sw) + xline) as usize] == 1 {
                    WHITE
                } else {
                    BLACK
                };
                self.canvas.set_draw_color(color);
                rect.set_x(i32::from(xline) * s);
                rect.set_y(i32::from(yline) * s);
                self.canvas.fill_rect(rect).map_err(anyhow::Error::msg)?;
            }
        }
        self.canvas.present();

        Ok(())
    }

    pub fn do_sound(&mut self, sound: bool) {
        if sound {
            if self.audio.paused() {
                self.audio.play();
            }
            return;
        }

        if self.audio.playing() {
            self.audio.pause();
        }
    }

    pub fn event_iter(&mut self) -> EventPollIterator {
        self.events.poll_iter()
    }
}
