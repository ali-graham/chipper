use anyhow::anyhow;
use anyhow::Error;
use anyhow::Result;
use bitvec::prelude::BitVec;
use sdl3::event::EventPollIterator;
use sdl3::pixels::Color;
use sdl3::rect::Rect;
use sdl3::render::Canvas;
use sdl3::render::FRect;
use sdl3::video::Window;
use sdl3::EventPump;
use sdl3::VideoSubsystem;

use crate::audio;
use crate::profile;

const WHITE: Color = Color::RGB(240, 240, 240);
const BLACK: Color = Color::RGB(15, 15, 15);

#[must_use]
pub(super) struct Hardware {
    profile: profile::Profile,
    scale: u8,
    canvas: Canvas<Window>,
    audio: audio::Audio,
    events: EventPump,
}

impl Hardware {
    pub(super) fn new(scale: Option<u8>, profile: profile::Profile) -> Result<Self> {
        let sdl_context = sdl3::init().map_err(Error::msg)?;

        let video = sdl_context.video().map_err(Error::msg)?;
        let scale = scale.unwrap_or_else(|| profile.default_screen_scale());
        let canvas = Self::init_canvas(
            &video,
            u16::from(profile.screen_width()) * u16::from(scale),
            u16::from(profile.screen_height()) * u16::from(scale),
        )?;

        let audio = audio::Audio::new(&sdl_context)?;

        let events = sdl_context.event_pump().map_err(Error::msg)?;

        Ok(Self {
            profile,
            scale,
            canvas,
            audio,
            events,
        })
    }

    fn init_canvas(
        video_subsys: &VideoSubsystem,
        width: u16,
        height: u16,
    ) -> Result<Canvas<Window>> {
        // TODO iterate through all displays, work out which one mouse is on,
        // use that to determine window size limit
        let dm = video_subsys.desktop_display_mode(0).map_err(Error::msg)?;

        if i32::from(width) > dm.w || i32::from(height) > dm.h {
            return Err(anyhow!("Window too large"));
        }

        let window = video_subsys
            .window("chipper", u32::from(width), u32::from(height))
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

    pub(super) fn refresh_graphics(&mut self, gfx: &BitVec, res_scale: u8) -> Result<()> {
        let rect_scale = u32::from(self.scale * res_scale);
        let mut rect = Rect::new(0, 0, rect_scale, rect_scale);
        let sw = u16::from(self.profile.screen_width());
        let s = i32::from(self.scale * res_scale);

        for yline in 0..u16::from(self.profile.screen_height()) {
            for xline in 0..sw {
                self.canvas
                    .set_draw_color(if gfx[usize::from((yline * sw) + xline)] {
                        WHITE
                    } else {
                        BLACK
                    });
                rect.set_x(i32::from(xline) * s);
                rect.set_y(i32::from(yline) * s);
                self.canvas
                    .fill_rect(FRect::from(rect))
                    .map_err(Error::msg)?;
            }
        }
        self.canvas.present();

        Ok(())
    }

    pub(super) fn sound_stop(&mut self) {
        if self.audio.playing() {
            self.audio.pause();
        }
    }

    pub(super) fn sound_start(&mut self) {
        if self.audio.paused() {
            self.audio.play();
        }
    }

    pub(super) fn event_iter(&mut self) -> EventPollIterator {
        self.events.poll_iter()
    }
}
