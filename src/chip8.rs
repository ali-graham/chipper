use std::collections::HashMap;

use anyhow::Context;
use anyhow::Result;
use once_cell::sync::Lazy;
use rand::Rng;
use rand::RngCore;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;

use crate::profile;
use crate::Action;
use crate::KeyMapping;
use crate::Target;

const CHIP8_FONTSET: [u8; 80] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];

const NUMBER_OF_KEYS: usize = 16;
const NUMBER_OF_REGISTERS: usize = 16;
const STACK_SIZE: usize = 16;

static KEY_MAPPINGS: Lazy<HashMap<KeyMapping, HashMap<Keycode, u8>>> = Lazy::new(|| {
    HashMap::from([
        (
            KeyMapping::Qwerty,
            HashMap::from([
                (Keycode::Num1, 0x1),
                (Keycode::Num2, 0x2),
                (Keycode::Num3, 0x3),
                (Keycode::Num4, 0xc),
                (Keycode::Q, 0x4),
                (Keycode::W, 0x5),
                (Keycode::E, 0x6),
                (Keycode::R, 0xd),
                (Keycode::A, 0x7),
                (Keycode::S, 0x8),
                (Keycode::D, 0x9),
                (Keycode::F, 0xe),
                (Keycode::Z, 0xa),
                (Keycode::X, 0x0),
                (Keycode::C, 0xb),
                (Keycode::V, 0xf),
            ]),
        ),
        (
            KeyMapping::Colemak,
            HashMap::from([
                (Keycode::Num1, 0x1),
                (Keycode::Num2, 0x2),
                (Keycode::Num3, 0x3),
                (Keycode::Num4, 0xc),
                (Keycode::Q, 0x4),
                (Keycode::W, 0x5),
                (Keycode::F, 0x6),
                (Keycode::P, 0xd),
                (Keycode::A, 0x7),
                (Keycode::R, 0x8),
                (Keycode::S, 0x9),
                (Keycode::T, 0xe),
                (Keycode::Z, 0xa),
                (Keycode::X, 0x0),
                (Keycode::C, 0xb),
                (Keycode::V, 0xf),
            ]),
        ),
    ])
});

pub(super) struct Chip8 {
    v: [u8; NUMBER_OF_REGISTERS], // registers
    uv: Option<Box<[u8]>>,

    i: u16, // can only be loaded with a 12-bit address value

    pc: u16, // program counter

    // 0x000-0x1FF - Chip 8 interpreter (contains font set in emu)
    // 0x050-0x0A0 - Used for the built in 4x5 pixel font set (0-F)
    // 0x200-0xE8F - Program ROM and work RAM
    // 0xEA0-0xEFF - 'internal usage and variables'
    // 0xF00-0xFFF - 'display refresh'
    memory: Box<[u8]>,

    gfx: Box<[bool]>,
    profile: profile::Profile,

    delay_timer: u8,

    // The system’s buzzer sounds whenever the sound timer reaches zero
    sound_timer: u8,

    stack: [u16; STACK_SIZE],
    sp: u8,

    key: [bool; NUMBER_OF_KEYS],

    draw: bool,
    hires: bool,

    target: Target,

    rng: Box<dyn RngCore>,

    keymap: &'static HashMap<Keycode, u8>,
    key_wait: (Option<bool>, Option<u8>),
}

impl Chip8 {
    pub(super) fn new(
        target: Target,
        profile: profile::Profile,
        key_mapping: KeyMapping,
        rng: Box<dyn RngCore>,
    ) -> Result<Self> {
        let keymap = KEY_MAPPINGS.get(&key_mapping).context("Unknown keymap")?;

        let user_registers = match profile.user_register_count() {
            ur if ur > 0 => Some(crate::util::boxed_array::<u8>(usize::from(ur))),
            _ => None,
        };

        let mut memory = crate::util::boxed_array::<u8>(profile.memory_capacity());
        memory[..CHIP8_FONTSET.len()].copy_from_slice(&CHIP8_FONTSET[..]); // load fontset

        Ok(Self {
            target,
            rng,
            keymap,
            memory,

            v: [0u8; 16],
            uv: user_registers,

            i: 0,

            pc: 0x200,

            delay_timer: 0,
            sound_timer: 0,
            stack: [0_u16; 16],
            sp: 0,
            gfx: crate::util::boxed_array::<bool>(
                usize::from(profile.screen_width()) * usize::from(profile.screen_height()),
            ),
            profile,
            key: [false; NUMBER_OF_KEYS],
            key_wait: (None, None),

            draw: true,
            hires: false,
        })
    }

    // FIXME error if the rom_data is too large for the memory space ( 0x200-0xE8F )
    pub(super) fn load_rom(&mut self, rom_data: &[u8]) {
        self.memory[0x200..(0x200 + rom_data.len())].copy_from_slice(rom_data);
    }

    pub(super) fn graphics(&self) -> &[bool] {
        &self.gfx
    }

    pub(super) fn graphics_needs_refresh(&self) -> bool {
        self.draw
    }

    pub(super) fn graphics_clear_refresh(&mut self) {
        self.draw = false;
    }

    pub(super) fn resolution_scale(&self) -> u8 {
        if self.hires || (self.target == Target::Chip8) {
            return 1;
        }

        2
    }

    pub(super) fn audio_sound(&self) -> bool {
        self.sound_timer > 0
    }

    pub(super) fn update_timers(&mut self) {
        self.delay_timer = self.delay_timer.saturating_sub(1);
        self.sound_timer = self.sound_timer.saturating_sub(1);
    }

    fn set_key(&mut self, code: Keycode, pressed: bool) -> Result<()> {
        let key_num = self.keymap.get(&code).context("invalid key code")?;

        match self.key_wait {
            (Some(false), Some(num)) if !pressed && num == *key_num => {
                self.key_wait = (Some(true), Some(num));
            }
            (Some(false), None) if pressed => self.key_wait = (Some(false), Some(*key_num)),
            _ => {}
        }

        self.key[usize::from(*key_num)] = pressed;

        Ok(())
    }

    pub(super) fn handle_key(&mut self, event: &Event) -> Result<Option<Action>> {
        match event {
            Event::Quit { .. }
            | Event::KeyDown {
                keycode: Some(Keycode::Escape),
                ..
            } => Ok(Some(Action::Quit)),
            Event::KeyDown {
                repeat: false,
                keycode: Some(code),
                ..
            }
            | Event::KeyUp {
                repeat: false,
                keycode: Some(code),
                ..
            } if self.keymap.contains_key(code) => self
                .set_key(*code, matches!(event, Event::KeyDown { .. }))
                .and(Ok(None)),
            _ => Ok(None),
        }
    }

    pub(super) fn emulate_cycle(&mut self) -> Option<Action> {
        let opcode = (u16::from(self.memory[usize::from(self.pc)]) << 8)
            | u16::from(self.memory[usize::from(self.pc + 1)]);

        let sc = matches!(self.target, Target::SuperChip);
        let xo = matches!(self.target, Target::XoChip);

        if (opcode == 0x00FD) && (sc || xo) {
            return Some(Action::Quit);
        }

        // these operations manipulate the program counter value directly
        if match opcode {
            0x00EE => self.c8_flow_return(),
            o if o & 0xF000 == 0x1000 => self.c8_flow_goto(o),
            o if o & 0xF000 == 0x2000 => self.c8_flow_gosub(o),
            o if o & 0xF000 == 0xB000 => self.c8_flow_jump(o),
            _ => false,
        } {
            return None;
        }

        self.pc += match opcode {
            0x00E0 => self.c8_display_clear(),

            o if o & 0xFFF0 == 0x00C0 && (sc || xo) => self.sc_scroll_down(o),
            o if o & 0xFFF0 == 0x00D0 && xo => self.xo_scroll_up(o),

            0x00FB if sc || xo => self.sc_scroll_right(),
            0x00FC if sc || xo => self.sc_scroll_left(),
            0x00FE if sc || xo => self.sc_display_low(),
            0x00FF if sc || xo => self.sc_display_high(),

            // 0NNN is ignored in original Chip8
            o if o & 0xF000 == 0x0000 => 2,

            o if o & 0xF000 == 0x3000 => self.c8_cond_skip_eq_num(o),
            o if o & 0xF000 == 0x4000 => self.c8_cond_skip_neq_num(o),

            o if o & 0xF00F == 0x5000 => self.c8_cond_skip_eq_reg(o),

            o if o & 0xF000 == 0x6000 => self.c8_const_set_num(o),
            o if o & 0xF000 == 0x7000 => self.c8_const_add_num(o),

            o if o & 0xF00F == 0x8000 => self.c8_assign_set_reg(o),
            o if o & 0xF00F == 0x8001 => self.c8_bitop_or_reg(o),
            o if o & 0xF00F == 0x8002 => self.c8_bitop_and_reg(o),
            o if o & 0xF00F == 0x8003 => self.c8_bitop_xor_reg(o),
            o if o & 0xF00F == 0x8004 => self.c8_math_add_reg(o),
            o if o & 0xF00F == 0x8005 => self.c8_math_sub_reg(o),
            o if o & 0xF00F == 0x8006 => self.c8_bitop_shr_reg(o),
            o if o & 0xF00F == 0x8007 => self.c8_math_neg_reg(o),
            o if o & 0xF00F == 0x800E => self.c8_bitop_shl_reg(o),

            o if o & 0xF00F == 0x9000 => self.c8_cond_skipifneq_reg(o),
            o if o & 0xF000 == 0xA000 => self.c8_mem_store(o),

            o if o & 0xF000 == 0xC000 => self.c8_rand_and_reg(o),

            o if o & 0xF000 == 0xD000 => self.c8_draw_sprite(o),

            o if o & 0xF0FF == 0xE09E => self.c8_key_pressedskip(o),
            o if o & 0xF0FF == 0xE0A1 => self.c8_key_notpressedskip(o),

            o if o & 0xF0FF == 0xF00A => self.c8_key_wait(o),

            o if o & 0xF0FF == 0xF007 => self.c8_timer_delay_store(o),
            o if o & 0xF0FF == 0xF015 => self.c8_timer_delay_set(o),
            o if o & 0xF0FF == 0xF018 => self.c8_timer_sound_set(o),

            o if o & 0xF0FF == 0xF01E => self.c8_mem_addi(o),

            o if o & 0xF0FF == 0xF029 => self.c8_mem_spriteaddr(o),

            o if (o & 0xF0FF == 0xF030) && (sc || xo) => self.sc_hires_font(o),

            o if o & 0xF0FF == 0xF033 => self.c8_bcd_store(o),

            o if o & 0xF0FF == 0xF055 => self.c8_mem_reg_dump(o),
            o if o & 0xF0FF == 0xF065 => self.c8_mem_reg_load(o),

            o if (o & 0xF0FF == 0xF075) && (sc || xo) => self.sc_flag_store(o),
            o if (o & 0xF0FF == 0xF085) && (sc || xo) => self.sc_flag_load(o),

            o => panic!("unknown opcode {o:x?}"),
        };

        None
    }

    fn c8_flow_return(&mut self) -> bool {
        // 00EE - return from a subroutine
        self.sp = self.sp.checked_sub(1).expect("stack pointer too low");
        self.pc = self.stack[usize::from(self.sp)] + 2;
        true
    }

    fn c8_flow_goto(&mut self, o: u16) -> bool {
        // 1NNN - goto
        self.pc = o & 0x0FFF;
        true
    }

    fn c8_flow_gosub(&mut self, o: u16) -> bool {
        // 2NNN - subroutine
        self.stack[usize::from(self.sp)] = self.pc;
        self.sp += 1;
        assert!(usize::from(self.sp) < STACK_SIZE, "stack pointer too high");

        self.pc = o & 0x0FFF;
        true
    }

    fn c8_display_clear(&mut self) -> u16 {
        // 00E0 - clear the screen
        for i in 0..self.gfx.len() {
            self.gfx[i] = false;
        }
        self.draw = true;

        2
    }

    fn c8_cond_skip_eq_num(&self, o: u16) -> u16 {
        // 3XNN - Skip the following instruction if the value of register VX equals NN
        let reg = Self::register_x(o);
        let val = Self::opcode_value(o);

        if self.v[reg] == val {
            4
        } else {
            2
        }
    }

    fn c8_cond_skip_neq_num(&self, o: u16) -> u16 {
        // 4XNN - Skip the following instruction if the value of register VX is not equal to NN
        let reg = Self::register_x(o);
        let val = Self::opcode_value(o);

        if self.v[reg] == val {
            2
        } else {
            4
        }
    }

    fn c8_cond_skip_eq_reg(&self, o: u16) -> u16 {
        // 5XY0 - Skip the following instruction if the value of register VX equals value of register VY
        let (reg_x, reg_y) = Self::register_xy(o);

        if self.v[reg_x] == self.v[reg_y] {
            4
        } else {
            2
        }
    }

    fn c8_const_set_num(&mut self, o: u16) -> u16 {
        // 6XNN - store NN in register X
        let reg = Self::register_x(o);

        self.v[reg] = Self::opcode_value(o);
        2
    }

    fn c8_const_add_num(&mut self, o: u16) -> u16 {
        // 7XNN - Add the value NN to register VX (carry flag is not changed)
        let reg = Self::register_x(o);

        let result_carry = self.v[reg].overflowing_add(Self::opcode_value(o));
        self.v[reg] = result_carry.0;
        2
    }

    fn c8_assign_set_reg(&mut self, o: u16) -> u16 {
        // 8XY0 - Assign the value of register VX to the value of register VY
        let (reg_x, reg_y) = Self::register_xy(o);

        self.v[reg_x] = self.v[reg_y];
        2
    }

    fn c8_bitop_or_reg(&mut self, o: u16) -> u16 {
        // 8XY1 - Bitwise OR the values of registers VX and register VY, result to VX
        let (reg_x, reg_y) = Self::register_xy(o);

        self.v[reg_x] |= self.v[reg_y];
        match self.target {
            Target::Chip8 => self.v[15] = 0,
            Target::SuperChip | Target::XoChip => {}
        }

        2
    }

    fn c8_bitop_and_reg(&mut self, o: u16) -> u16 {
        // 8XY2 - Bitwise AND the values of registers VX and register VY, result to VX
        let (reg_x, reg_y) = Self::register_xy(o);

        self.v[reg_x] &= self.v[reg_y];
        match self.target {
            Target::Chip8 => self.v[15] = 0,
            Target::SuperChip | Target::XoChip => {}
        }

        2
    }

    fn c8_bitop_xor_reg(&mut self, o: u16) -> u16 {
        // 8XY3 - Bitwise XOR the values of registers VX and register VY, result to VX
        let (reg_x, reg_y) = Self::register_xy(o);

        self.v[reg_x] ^= self.v[reg_y];
        match self.target {
            Target::Chip8 => self.v[15] = 0,
            Target::SuperChip | Target::XoChip => {}
        }

        2
    }

    fn c8_math_add_reg(&mut self, o: u16) -> u16 {
        // 8XY4 - Add the values of registers VX and register VY, result to VX
        // VF = 1 if overflow
        let (reg_x, reg_y) = Self::register_xy(o);

        let result_carry = self.v[reg_x].overflowing_add(self.v[reg_y]);
        self.v[reg_x] = result_carry.0;
        self.v[15] = u8::from(result_carry.1);
        2
    }

    fn c8_math_sub_reg(&mut self, o: u16) -> u16 {
        // 8XY5 - Subtract value of register VY from value of register VX, result to VX
        // VF = 1 if no borrow, 0 if there is
        let (reg_x, reg_y) = Self::register_xy(o);

        let result_borrow = self.v[reg_x].overflowing_sub(self.v[reg_y]);
        self.v[reg_x] = result_borrow.0;
        self.v[15] = u8::from(!result_borrow.1);
        2
    }

    fn c8_bitop_shr_reg(&mut self, o: u16) -> u16 {
        // 8XY6 - Store the value of register VY shifted right one bit in register VX
        // Set register VF to the least significant bit prior to the shift
        // NB: modern interpreters seem to operate on reg_x only
        let reg_x = Self::register_x(o);

        let val = match self.target {
            Target::Chip8 | Target::XoChip => self.v[Self::register_y(o)],
            Target::SuperChip => self.v[reg_x],
        };

        self.v[reg_x] = val.checked_shr(1).unwrap_or(0);
        self.v[15] = val & 0x1;
        2
    }

    fn c8_math_neg_reg(&mut self, o: u16) -> u16 {
        // 8XY7 - Subtract value of register VX from value of register VY, result to VX
        // VF = 1 if no borrow, 0 if there is
        let (reg_x, reg_y) = Self::register_xy(o);

        let result_borrow = self.v[reg_y].overflowing_sub(self.v[reg_x]);
        self.v[reg_x] = result_borrow.0;
        self.v[15] = u8::from(!result_borrow.1);
        2
    }

    fn c8_bitop_shl_reg(&mut self, o: u16) -> u16 {
        // 8XYE
        let reg_x = Self::register_x(o);

        let val = match self.target {
            Target::Chip8 | Target::XoChip => self.v[Self::register_y(o)],
            Target::SuperChip => self.v[reg_x],
        };

        self.v[reg_x] = val.checked_shl(1).unwrap_or(u8::max_value());
        self.v[15] = val.checked_shr(7).unwrap_or(0) & 0x1;
        2
    }

    fn c8_cond_skipifneq_reg(&self, o: u16) -> u16 {
        // 9XY0 - Skip the following instruction if the value of register VX does not equal value of register VY
        let (reg_x, reg_y) = Self::register_xy(o);

        if self.v[reg_x] == self.v[reg_y] {
            2
        } else {
            4
        }
    }

    fn c8_mem_store(&mut self, o: u16) -> u16 {
        // ANNN - store NNN in I
        self.i = o & 0x0FFF;
        2
    }

    fn c8_flow_jump(&mut self, o: u16) -> bool {
        // BNNN - goto NNN + V0
        // BXNN - goto XNN + VX
        self.pc = (o & 0x0FFF)
            + match self.target {
                Target::Chip8 | Target::XoChip => u16::from(self.v[0]),
                Target::SuperChip => u16::from(self.v[Self::register_x(o)]),
            };

        true
    }

    fn c8_rand_and_reg(&mut self, o: u16) -> u16 {
        // CXNN - Sets VX to the result of a bitwise and operation on a random number (Typically: 0 to 255) and NN.
        let reg = Self::register_x(o);
        let val = Self::opcode_value(o);

        self.v[reg] = val & self.rng.gen::<u8>();
        2
    }

    // TODO fix sprite wrapping for Xochip in lores/hires modes
    fn c8_draw_sprite(&mut self, o: u16) -> u16 {
        // DXYN - Draw a sprite at position VX, VY with N bytes of sprite data starting at the address stored in I
        // Set VF to 01 if any set pixels are changed to unset, and 00 otherwise
        let (reg_x, reg_y) = Self::register_xy(o);
        let data_count = o & 0x000F;

        let (w, h) =
            if (self.target == Target::SuperChip || self.target == Target::XoChip) && !self.hires {
                (
                    self.profile.screen_width() / 2,
                    self.profile.screen_height() / 2,
                )
            } else {
                (self.profile.screen_width(), self.profile.screen_height())
            };

        let unset = if (self.target == Target::SuperChip || self.target == Target::XoChip)
            && self.hires
            && data_count == 0
        {
            // self.sprite_draw_16(self.v[reg_x], self.v[reg_y], w, h)
            todo!()
        } else {
            self.sprite_draw_8(self.v[reg_x], self.v[reg_y], w, h, data_count)
        };

        self.draw = true;
        self.v[15] = u8::from(unset);
        2
    }

    // fn sprite_draw_16(&mut self, _x: u8, _y: u8, _w: u8, _h: u8q) -> bool {
    //     todo!("16 x 16 sprite");
    // }

    fn sprite_draw_8(&mut self, x: u8, y: u8, w: u8, h: u8, data_count: u16) -> bool {
        let mut offset: usize;
        let mut x_off: usize;
        let mut y_off: usize;
        let mut unset: bool = false;

        // wrap starting draw positions if outside screen boundaries
        let (x_pos, y_pos) = (usize::from(x % w), usize::from(y % h));

        for yline in 0..usize::from(data_count) {
            y_off = y_pos + yline;
            if self.target == Target::XoChip {
                y_off %= usize::from(h);
            } else if y_off >= usize::from(h) {
                // clip if going past bottom of screen
                break;
            }
            let y_temp = y_off * usize::from(self.profile.screen_width());
            let mem_value = self.memory[usize::from(self.i) + yline];
            for xline in 0..8u8 {
                x_off = x_pos + usize::from(xline);
                if self.target == Target::XoChip {
                    x_off %= usize::from(w);
                } else if x_off >= usize::from(w) {
                    // clip if going past bottom of screen
                    break;
                }
                if (mem_value & 0x80u8.rotate_right(xline.into())) != 0 {
                    offset = y_temp + x_off;
                    unset |= self.gfx[offset];
                    self.gfx[offset] ^= true;
                }
            }
        }

        unset
    }

    fn c8_key_pressedskip(&self, o: u16) -> u16 {
        // EX9E - Skip the following instruction if the key corresponding to the
        // value currently stored in register VX is pressed
        let reg = Self::register_x(o);

        if self.key[usize::from(self.v[reg])] {
            4
        } else {
            2
        }
    }

    fn c8_key_notpressedskip(&self, o: u16) -> u16 {
        // EXA1 - Skip the following instruction if the key corresponding to the
        // value currently stored in register VX is not pressed
        let reg = Self::register_x(o);

        if self.key[usize::from(self.v[reg])] {
            2
        } else {
            4
        }
    }

    fn c8_key_wait(&mut self, o: u16) -> u16 {
        // FX0A - Wait for a keypress and store the result in register VX
        let reg = Self::register_x(o);

        match self.key_wait {
            (Some(true), Some(key_num)) => {
                self.key_wait = (None, None);
                self.v[reg] = key_num;
                2
            }
            (None, None) => {
                self.key_wait = (Some(false), None);
                0
            }
            _ => 0,
        }
    }

    fn c8_timer_delay_store(&mut self, o: u16) -> u16 {
        // FX07 - Store the current value of the delay timer in register VX
        let reg_x = Self::register_x(o);

        self.v[reg_x] = self.delay_timer;
        2
    }

    fn c8_timer_delay_set(&mut self, o: u16) -> u16 {
        // FX15 - Set the delay timer to the value of register VX
        let reg_x = Self::register_x(o);

        self.delay_timer = self.v[reg_x];
        2
    }

    fn c8_timer_sound_set(&mut self, o: u16) -> u16 {
        // FX18 - Set the sound timer to the value of register VX
        let reg_x = Self::register_x(o);

        self.sound_timer = self.v[reg_x];
        2
    }

    fn c8_mem_addi(&mut self, o: u16) -> u16 {
        // FX1E - Add the value stored in register VX to register I
        // Sets carry flag if 12-bit limit exceeded for I
        let reg_x = Self::register_x(o);

        self.i += u16::from(self.v[reg_x]);

        if self.i > 0xFFF {
            self.i -= 0x1000;
            self.v[15] = 1;
        }

        2
    }

    fn c8_mem_spriteaddr(&mut self, o: u16) -> u16 {
        // FX29 - Set I to the memory address of the sprite data corresponding to the
        // hexadecimal digit stored in register VX
        let reg_x = Self::register_x(o);

        self.i = u16::from(5 * (self.v[reg_x] & 0xF));
        2
    }

    fn sc_hires_font(&mut self, o: u16) -> u16 {
        // FX30 - Set I to the memory address of the (10-bit) sprite data corresponding to the
        // hexadecimal digit stored in register VX (0-9)
        let reg_x = Self::register_x(o);
        let value = self.v[reg_x];

        if value <= 9 {
            self.i = u16::from(10 * value);
        }

        2
    }

    fn c8_bcd_store(&mut self, o: u16) -> u16 {
        // FX33 - Store the binary-coded decimal equivalent of the value stored in
        // register VX at addresses I, I + 1, and I + 2
        let reg_x = Self::register_x(o);

        let val = self.v[reg_x];
        let ones = val % 10;
        let tens = ((val % 100) - ones) / 10;
        let hundreds = (val - (tens + ones)) / 100;

        self.memory[usize::from(self.i)] = hundreds;
        self.memory[usize::from(self.i + 1)] = tens;
        self.memory[usize::from(self.i + 2)] = ones;

        2
    }

    fn c8_mem_reg_dump(&mut self, o: u16) -> u16 {
        // FX55 - Store the values of registers V0 to VX inclusive in memory starting at address I
        // I is set to I + X + 1 after operation
        let reg_num = (o & 0x0F00) >> 8;
        self.memory[usize::from(self.i)..=(usize::from(self.i) + usize::from(reg_num))]
            .copy_from_slice(&self.v[0..=usize::from(reg_num)]);

        match self.target {
            Target::Chip8 | Target::XoChip => self.i += reg_num + 1,
            Target::SuperChip => {}
        }

        2
    }

    fn c8_mem_reg_load(&mut self, o: u16) -> u16 {
        // FX65 - Fill registers V0 to VX inclusive with the values stored in memory starting at address I
        // I is set to I + X + 1 after operation
        let reg_num = (o & 0x0F00) >> 8;
        self.v[0..=usize::from(reg_num)]
            .copy_from_slice(&self.memory[usize::from(self.i)..=usize::from(self.i + reg_num)]);

        match self.target {
            Target::Chip8 | Target::XoChip => self.i += reg_num + 1,
            Target::SuperChip => {}
        }

        2
    }

    fn sc_display_low(&mut self) -> u16 {
        // 00FE: Disable high-resolution mode
        if self.hires {
            if self.target == Target::XoChip {
                // clear sceen on display mode change
                for i in 0..self.gfx.len() {
                    self.gfx[i] = false;
                }
            }

            self.hires = false;
            self.draw = true;
        }

        2
    }

    fn sc_display_high(&mut self) -> u16 {
        // 00FF: Enable high-resolution mode
        if !self.hires {
            if self.target == Target::XoChip {
                // clear sceen on display mode change
                for i in 0..self.gfx.len() {
                    self.gfx[i] = false;
                }
            }

            self.hires = true;
            self.draw = true;
        }

        2
    }

    fn sc_flag_load(&mut self, o: u16) -> u16 {
        // FX75 - Store V0..VX in RPL user flags (X <= 7 for SuperChip)
        let c = usize::from((o & 0x0F00) >> 8);

        match self.uv.as_mut() {
            Some(reg) if c < usize::from(self.profile.user_register_count()) => {
                reg[0..=c].copy_from_slice(&self.v[0..=c]);
            }
            Some(_) | None => {}
        }

        2
    }

    fn sc_flag_store(&mut self, o: u16) -> u16 {
        // FX85 - Read V0..VX from RPL user flags (X <= 7 for SuperChip)
        let c = usize::from((o & 0x0F00) >> 8);

        match self.uv.as_mut() {
            Some(reg) if c < usize::from(self.profile.user_register_count()) => {
                self.v[0..=c].copy_from_slice(&reg[0..=c]);
            }
            Some(_) | None => {}
        }

        2
    }

    fn sc_scroll_down(&mut self, o: u16) -> u16 {
        // 00CN

        let n = usize::from(o & 0xF);
        let w = usize::from(self.profile.screen_width());
        let h = usize::from(self.profile.screen_height());

        for y in (n..h).rev() {
            for x in 0..w {
                self.gfx[(y * w) + x] = self.gfx[((y - n) * w) + x];
            }
        }

        for y in 0..n {
            for x in 0..w {
                self.gfx[(y * w) + x] = false;
            }
        }

        self.draw = true;

        2
    }

    fn xo_scroll_up(&mut self, o: u16) -> u16 {
        // 00DN

        let n = usize::from(o & 0xF);
        let w = usize::from(self.profile.screen_width());
        let h = usize::from(self.profile.screen_height());

        for y in n..h {
            for x in 0..w {
                self.gfx[(y * w) + x] = self.gfx[((y + n) * w) + x];
            }
        }

        for y in (h - n)..h {
            for x in 0..w {
                self.gfx[(y * w) + x] = false;
            }
        }

        self.draw = true;

        2
    }

    fn sc_scroll_right(&mut self) -> u16 {
        // 00FB

        let w = usize::from(self.profile.screen_width());
        let h = usize::from(self.profile.screen_height());

        for x in (4..w).rev() {
            for y in 0..h {
                self.gfx[(y * w) + x] = self.gfx[(y * w) + (x - 4)];
            }
        }

        for x in 0usize..3usize {
            for y in 0..h {
                self.gfx[(y * w) + x] = false;
            }
        }

        self.draw = true;

        2
    }

    fn sc_scroll_left(&mut self) -> u16 {
        // 00FC

        let w = usize::from(self.profile.screen_width());
        let h = usize::from(self.profile.screen_height());

        for x in 0..(w - 4) {
            for y in 0..h {
                self.gfx[(y * w) + x] = self.gfx[(y * w) + (x + 4)];
            }
        }

        for x in (w - 4)..w {
            for y in 0..h {
                self.gfx[(y * w) + x] = false;
            }
        }

        self.draw = true;

        2
    }

    fn register_x(o: u16) -> usize {
        usize::from((o & 0x0F00).wrapping_shr(8))
    }

    fn register_y(o: u16) -> usize {
        usize::from((o & 0x00F0).wrapping_shr(4))
    }

    fn register_xy(o: u16) -> (usize, usize) {
        (
            usize::from((o & 0x0F00).wrapping_shr(8)),
            usize::from((o & 0x00F0).wrapping_shr(4)),
        )
    }

    fn opcode_value(o: u16) -> u8 {
        (o & 0x00FF) as u8
    }
}

#[cfg(test)]
mod tests {
    use std::panic;

    use anyhow::Result;
    use rand::rngs::mock::StepRng;
    use sdl2::keyboard::Keycode;
    use sdl2::keyboard::Mod;

    use super::Chip8;
    use crate::profile;
    use crate::util::boxed_array;
    use crate::{Action, KeyMapping, Target};

    #[test]
    fn test_graphics_needs_refresh() {
        // when
        let mut chip8 = Chip8::new(
            Target::Chip8,
            *profile::PROFILES.get(&Target::Chip8).unwrap(),
            KeyMapping::Qwerty,
            Box::new(rand::thread_rng()),
        )
        .expect("Could not initialise Chip8");
        chip8.draw = true;

        // then
        let needs_refresh = chip8.graphics_needs_refresh();

        // verify
        assert!(needs_refresh);
    }

    #[test]
    fn test_graphics_does_not_need_refresh() {
        // when
        let mut chip8 = Chip8::new(
            Target::Chip8,
            *profile::PROFILES.get(&Target::Chip8).unwrap(),
            KeyMapping::Qwerty,
            Box::new(rand::thread_rng()),
        )
        .expect("Could not initialise Chip8");
        chip8.draw = false;

        // then
        let needs_refresh = chip8.graphics_needs_refresh();

        // verify
        assert!(!needs_refresh);
    }

    #[test]
    fn test_graphics_clear_refresh() {
        // when
        let mut chip8 = Chip8::new(
            Target::Chip8,
            *profile::PROFILES.get(&Target::Chip8).unwrap(),
            KeyMapping::Qwerty,
            Box::new(rand::thread_rng()),
        )
        .expect("Could not initialise Chip8");
        chip8.draw = true;

        // then
        chip8.graphics_clear_refresh();

        // verify
        assert!(!chip8.draw);
    }

    #[test]
    fn test_sound_should_play() {
        // when
        let mut chip8 = Chip8::new(
            Target::Chip8,
            *profile::PROFILES.get(&Target::Chip8).unwrap(),
            KeyMapping::Qwerty,
            Box::new(rand::thread_rng()),
        )
        .expect("Could not initialise Chip8");
        chip8.sound_timer = 1;

        // then
        let should_play = chip8.audio_sound();

        // verify
        assert!(should_play);
    }

    #[test]
    fn test_sound_should_not_play() {
        // when
        let mut chip8 = Chip8::new(
            Target::Chip8,
            *profile::PROFILES.get(&Target::Chip8).unwrap(),
            KeyMapping::Qwerty,
            Box::new(rand::thread_rng()),
        )
        .expect("Could not initialise Chip8");
        chip8.sound_timer = 0;

        // then
        let should_play = chip8.audio_sound();

        // verify
        assert!(!should_play);
    }

    #[test]
    fn test_timers_count_down() {
        // when
        let mut chip8 = Chip8::new(
            Target::Chip8,
            *profile::PROFILES.get(&Target::Chip8).unwrap(),
            KeyMapping::Qwerty,
            Box::new(rand::thread_rng()),
        )
        .expect("Could not initialise Chip8");
        chip8.delay_timer = 15;
        chip8.sound_timer = 10;

        // then
        chip8.update_timers();

        // verify
        assert_eq!(chip8.delay_timer, 14);
        assert_eq!(chip8.sound_timer, 9);
    }

    #[test]
    fn test_timers_stop_at_zero() {
        // when
        let mut chip8 = Chip8::new(
            Target::Chip8,
            *profile::PROFILES.get(&Target::Chip8).unwrap(),
            KeyMapping::Qwerty,
            Box::new(rand::thread_rng()),
        )
        .expect("Could not initialise Chip8");
        chip8.delay_timer = 0;
        chip8.sound_timer = 0;

        // then
        chip8.update_timers();

        // verify
        assert_eq!(chip8.delay_timer, 0);
        assert_eq!(chip8.sound_timer, 0);
    }

    #[test]
    fn test_quit_event_returns_quit_action() {
        // when
        let mut chip8 = Chip8::new(
            Target::Chip8,
            *profile::PROFILES.get(&Target::Chip8).unwrap(),
            KeyMapping::Qwerty,
            Box::new(rand::thread_rng()),
        )
        .expect("Could not initialise Chip8");

        // then
        let result = chip8.handle_key(&sdl2::event::Event::Quit { timestamp: 0 });

        // verify
        assert_eq!(result.unwrap(), Some(Action::Quit));
    }

    #[test]
    fn test_keydown_escape_returns_quit_action() {
        // when
        let mut chip8 = Chip8::new(
            Target::Chip8,
            *profile::PROFILES.get(&Target::Chip8).unwrap(),
            KeyMapping::Qwerty,
            Box::new(rand::thread_rng()),
        )
        .expect("Could not initialise Chip8");

        // then
        let result = chip8.handle_key(&sdl2::event::Event::KeyDown {
            timestamp: 0,
            window_id: 0,
            keycode: Some(Keycode::Escape),
            scancode: None,
            keymod: Mod::empty(),
            repeat: false,
        });

        // verify
        assert_eq!(result.unwrap(), Some(Action::Quit));
    }

    #[test]
    fn test_other_key_events_store_pressed_state() {
        // when
        let mut chip8 = Chip8::new(
            Target::Chip8,
            *profile::PROFILES.get(&Target::Chip8).unwrap(),
            KeyMapping::Qwerty,
            Box::new(rand::thread_rng()),
        )
        .expect("Could not initialise Chip8");
        let mut results = Vec::<Result<Option<Action>>>::with_capacity(chip8.keymap.len());

        // then
        for keycode in chip8.keymap.keys() {
            let event = &sdl2::event::Event::KeyDown {
                timestamp: 0,
                window_id: 0,
                keycode: Some(*keycode),
                scancode: None,
                keymod: Mod::empty(),
                repeat: false,
            };
            results.push(chip8.handle_key(event));
        }

        // verify
        assert_eq!(chip8.key, [true; 16]);
        assert!(results.into_iter().all(|r| r.unwrap().is_none()));
    }

    #[test]
    fn test_other_key_events_store_released_state() {
        // when
        let mut chip8 = Chip8::new(
            Target::Chip8,
            *profile::PROFILES.get(&Target::Chip8).unwrap(),
            KeyMapping::Qwerty,
            Box::new(rand::thread_rng()),
        )
        .expect("Could not initialise Chip8");
        chip8.key = [true; 16];
        let mut results = Vec::<Result<Option<Action>>>::with_capacity(chip8.keymap.len());

        // then
        for keycode in chip8.keymap.keys() {
            let event = &sdl2::event::Event::KeyUp {
                timestamp: 0,
                window_id: 0,
                keycode: Some(*keycode),
                scancode: None,
                keymod: Mod::empty(),
                repeat: false,
            };
            results.push(chip8.handle_key(event));
        }

        // verify
        assert_eq!(chip8.key, [false; 16]);
        assert!(results.into_iter().all(|r| r.unwrap().is_none()));
    }

    #[test]
    fn test_c8_display_clear() {
        // when
        let mut chip8 = Chip8::new(
            Target::Chip8,
            *profile::PROFILES.get(&Target::Chip8).unwrap(),
            KeyMapping::Qwerty,
            Box::new(rand::thread_rng()),
        )
        .expect("Could not initialise Chip8");
        //
        chip8.gfx = Box::new([true; 64 * 32]);
        chip8.draw = false;
        chip8.memory[0x200] = 0x0;
        chip8.memory[0x201] = 0xe0;

        // then
        chip8.emulate_cycle();

        // verify
        assert_eq!(chip8.gfx, boxed_array::<bool>(64 * 32));
        assert!(chip8.draw);
        assert_eq!(chip8.pc, 0x202);
    }

    #[test]
    fn test_ignored_opcode() {
        // TODO all ignored values

        // when
        let mut chip8 = Chip8::new(
            Target::Chip8,
            *profile::PROFILES.get(&Target::Chip8).unwrap(),
            KeyMapping::Qwerty,
            Box::new(rand::thread_rng()),
        )
        .expect("Could not initialise Chip8");
        chip8.memory[0x200] = 0x3;
        chip8.memory[0x201] = 0x21;

        // then
        chip8.emulate_cycle();

        // verify
        assert_eq!(chip8.pc, 0x202);
    }

    #[test]
    fn test_c8_flow_return_ok() {
        // when
        let mut chip8 = Chip8::new(
            Target::Chip8,
            *profile::PROFILES.get(&Target::Chip8).unwrap(),
            KeyMapping::Qwerty,
            Box::new(rand::thread_rng()),
        )
        .expect("Could not initialise Chip8");
        chip8.memory[0x400] = 0x00;
        chip8.memory[0x401] = 0xEE;
        chip8.sp = 1;
        chip8.pc = 0x400;
        chip8.stack[0] = 0x600;

        // then
        chip8.emulate_cycle();

        // verify
        assert_eq!(chip8.pc, 0x602);
        assert_eq!(chip8.sp, 0);
    }

    #[test]
    fn test_c8_flow_return_badstack() {
        let result = catch_unwind_silent(|| {
            // when
            let mut chip8 = Chip8::new(
                Target::Chip8,
                *profile::PROFILES.get(&Target::Chip8).unwrap(),
                KeyMapping::Qwerty,
                Box::new(rand::thread_rng()),
            )
            .expect("Could not initialise Chip8");
            chip8.memory[0x400] = 0x00;
            chip8.memory[0x401] = 0xEE;
            chip8.sp = 0;
            chip8.pc = 0x400;
            chip8.stack[0] = 0x600;

            // then
            chip8.emulate_cycle();
        });

        // verify
        assert!(result.is_err());
    }

    #[test]
    fn test_c8_flow_goto() {
        // when
        let mut chip8 = Chip8::new(
            Target::Chip8,
            *profile::PROFILES.get(&Target::Chip8).unwrap(),
            KeyMapping::Qwerty,
            Box::new(rand::thread_rng()),
        )
        .expect("Could not initialise Chip8");
        chip8.memory[0x200] = 0x13;
        chip8.memory[0x201] = 0x21;

        // then
        chip8.emulate_cycle();

        // verify
        assert_eq!(chip8.pc, 0x321);
    }

    #[test]
    fn test_c8_flow_gosub_ok() {
        // when
        let mut chip8 = Chip8::new(
            Target::Chip8,
            *profile::PROFILES.get(&Target::Chip8).unwrap(),
            KeyMapping::Qwerty,
            Box::new(rand::thread_rng()),
        )
        .expect("Could not initialise Chip8");
        chip8.memory[0x200] = 0x25;
        chip8.memory[0x201] = 0x73;
        chip8.pc = 0x200;
        chip8.sp = 0;

        // then
        chip8.emulate_cycle();

        // verify
        assert_eq!(chip8.pc, 0x573);
        assert_eq!(chip8.sp, 0x1);
        assert_eq!(chip8.stack[0], 0x200);
    }

    #[test]
    fn test_c8_flow_gosub_badstack() {
        let result = catch_unwind_silent(|| {
            // when
            let mut chip8 = Chip8::new(
                Target::Chip8,
                *profile::PROFILES.get(&Target::Chip8).unwrap(),
                KeyMapping::Qwerty,
                Box::new(rand::thread_rng()),
            )
            .expect("Could not initialise Chip8");
            chip8.memory[0x200] = 0x25;
            chip8.memory[0x201] = 0x73;
            chip8.pc = 0x200;
            chip8.sp = 16;

            // then
            chip8.emulate_cycle();
        });

        // verify
        assert!(result.is_err());
    }

    #[test]
    fn test_c8_cond_skipifeq_reg_skips() {
        // when
        let mut chip8 = Chip8::new(
            Target::Chip8,
            *profile::PROFILES.get(&Target::Chip8).unwrap(),
            KeyMapping::Qwerty,
            Box::new(rand::thread_rng()),
        )
        .expect("Could not initialise Chip8");
        chip8.memory[0x200] = 0x53;
        chip8.memory[0x201] = 0x10;
        chip8.v[3] = 2;
        chip8.v[1] = 2;

        // then
        chip8.emulate_cycle();

        // verify
        assert_eq!(chip8.pc, 0x204);
    }

    #[test]
    fn test_c8_cond_skipifeq_reg_does_not_skip() {
        // when
        let mut chip8 = Chip8::new(
            Target::Chip8,
            *profile::PROFILES.get(&Target::Chip8).unwrap(),
            KeyMapping::Qwerty,
            Box::new(rand::thread_rng()),
        )
        .expect("Could not initialise Chip8");
        chip8.memory[0x200] = 0x55;
        chip8.memory[0x201] = 0x40;
        chip8.v[5] = 1;
        chip8.v[4] = 2;

        // then
        chip8.emulate_cycle();

        // verify
        assert_eq!(chip8.pc, 0x202);
    }

    #[test]
    fn test_c8_const_set_num() {
        // when
        let mut chip8 = Chip8::new(
            Target::Chip8,
            *profile::PROFILES.get(&Target::Chip8).unwrap(),
            KeyMapping::Qwerty,
            Box::new(rand::thread_rng()),
        )
        .expect("Could not initialise Chip8");
        chip8.memory[0x200] = 0x66;
        chip8.memory[0x201] = 0xD2;
        chip8.v[6] = 0;

        // then
        chip8.emulate_cycle();

        // verify
        assert_eq!(chip8.v[6], 0xD2);
        assert_eq!(chip8.pc, 0x202);
    }

    #[test]
    fn test_c8_const_add_num_overflow_no_carry() {
        // when
        let mut chip8 = Chip8::new(
            Target::Chip8,
            *profile::PROFILES.get(&Target::Chip8).unwrap(),
            KeyMapping::Qwerty,
            Box::new(rand::thread_rng()),
        )
        .expect("Could not initialise Chip8");
        chip8.memory[0x200] = 0x76;
        chip8.memory[0x201] = 0xD2;
        chip8.v[6] = 0x65;

        // then
        chip8.emulate_cycle();

        // verify
        assert_eq!(chip8.v[6], 0x37);
        assert_eq!(chip8.v[15], 0);
        assert_eq!(chip8.pc, 0x202);
    }

    #[test]
    fn test_c8_const_add_num_no_overflow() {
        // when
        let mut chip8 = Chip8::new(
            Target::Chip8,
            *profile::PROFILES.get(&Target::Chip8).unwrap(),
            KeyMapping::Qwerty,
            Box::new(rand::thread_rng()),
        )
        .expect("Could not initialise Chip8");
        chip8.memory[0x200] = 0x73;
        chip8.memory[0x201] = 0x12;
        chip8.v[3] = 0x17;

        // then
        chip8.emulate_cycle();

        // verify
        assert_eq!(chip8.v[3], 0x29);
        assert_eq!(chip8.v[15], 0);
        assert_eq!(chip8.pc, 0x202);
    }

    #[test]
    fn test_c8_assign_set_reg() {
        // when
        let mut chip8 = Chip8::new(
            Target::Chip8,
            *profile::PROFILES.get(&Target::Chip8).unwrap(),
            KeyMapping::Qwerty,
            Box::new(rand::thread_rng()),
        )
        .expect("Could not initialise Chip8");
        chip8.memory[0x200] = 0x83;
        chip8.memory[0x201] = 0x10;
        chip8.v[3] = 1;
        chip8.v[1] = 52;

        // then
        chip8.emulate_cycle();

        // verify
        assert_eq!(chip8.v[3], 52);
        assert_eq!(chip8.pc, 0x202);
    }

    #[test]
    fn test_c8_bitop_or_reg() {
        // when
        let mut chip8 = Chip8::new(
            Target::Chip8,
            *profile::PROFILES.get(&Target::Chip8).unwrap(),
            KeyMapping::Qwerty,
            Box::new(rand::thread_rng()),
        )
        .expect("Could not initialise Chip8");
        chip8.memory[0x200] = 0x83;
        chip8.memory[0x201] = 0xa1;
        chip8.v[3] = 0b0110;
        chip8.v[10] = 0b1001;
        chip8.v[15] = 1;

        // then
        chip8.emulate_cycle();

        // verify
        assert_eq!(chip8.v[3], 0b1111);
        assert_eq!(chip8.v[15], 0);
        assert_eq!(chip8.pc, 0x202);
    }

    #[test]
    fn test_c8_bitop_and_reg() {
        // when
        let mut chip8 = Chip8::new(
            Target::Chip8,
            *profile::PROFILES.get(&Target::Chip8).unwrap(),
            KeyMapping::Qwerty,
            Box::new(rand::thread_rng()),
        )
        .expect("Could not initialise Chip8");
        chip8.memory[0x200] = 0x83;
        chip8.memory[0x201] = 0x72;
        chip8.v[3] = 0b0110;
        chip8.v[7] = 0b1001;
        chip8.v[15] = 1;

        // then
        chip8.emulate_cycle();

        // verify
        assert_eq!(chip8.v[3], 0);
        assert_eq!(chip8.v[15], 0);
        assert_eq!(chip8.pc, 0x202);
    }

    #[test]
    fn test_c8_bitop_xor_reg() {
        // when
        let mut chip8 = Chip8::new(
            Target::Chip8,
            *profile::PROFILES.get(&Target::Chip8).unwrap(),
            KeyMapping::Qwerty,
            Box::new(rand::thread_rng()),
        )
        .expect("Could not initialise Chip8");
        chip8.memory[0x200] = 0x83;
        chip8.memory[0x201] = 0x23;
        chip8.v[3] = 0b1001;
        chip8.v[2] = 0b1010;
        chip8.v[15] = 1;

        // then
        chip8.emulate_cycle();

        // verify
        assert_eq!(chip8.v[3], 0b11);
        assert_eq!(chip8.v[15], 0);
        assert_eq!(chip8.pc, 0x202);
    }

    #[test]
    fn test_c8_math_add_reg_overflow_carry() {
        // when
        let mut chip8 = Chip8::new(
            Target::Chip8,
            *profile::PROFILES.get(&Target::Chip8).unwrap(),
            KeyMapping::Qwerty,
            Box::new(rand::thread_rng()),
        )
        .expect("Could not initialise Chip8");
        chip8.memory[0x200] = 0x82;
        chip8.memory[0x201] = 0xC4;
        chip8.v[2] = 0x82;
        chip8.v[0xc] = 0xa7;

        // then
        chip8.emulate_cycle();

        // verify
        assert_eq!(chip8.v[2], 0x29);
        assert_eq!(chip8.v[15], 1);
        assert_eq!(chip8.pc, 0x202);
    }

    #[test]
    fn test_c8_math_add_reg_no_overflow() {
        // when
        let mut chip8 = Chip8::new(
            Target::Chip8,
            *profile::PROFILES.get(&Target::Chip8).unwrap(),
            KeyMapping::Qwerty,
            Box::new(rand::thread_rng()),
        )
        .expect("Could not initialise Chip8");
        chip8.memory[0x200] = 0x83;
        chip8.memory[0x201] = 0x14;
        chip8.v[3] = 0x17;
        chip8.v[1] = 0x15;

        // then
        chip8.emulate_cycle();

        // verify
        assert_eq!(chip8.v[3], 0x2c);
        assert_eq!(chip8.v[15], 0);
        assert_eq!(chip8.pc, 0x202);
    }

    #[test]
    fn test_c8_math_sub_reg_overflow_borrow() {
        // when
        let mut chip8 = Chip8::new(
            Target::Chip8,
            *profile::PROFILES.get(&Target::Chip8).unwrap(),
            KeyMapping::Qwerty,
            Box::new(rand::thread_rng()),
        )
        .expect("Could not initialise Chip8");
        chip8.memory[0x200] = 0x81;
        chip8.memory[0x201] = 0xa5;
        chip8.v[1] = 0x82;
        chip8.v[0xa] = 0xa7;

        // then
        chip8.emulate_cycle();

        // verify
        assert_eq!(chip8.v[1], 0xdb);
        assert_eq!(chip8.v[15], 0);
        assert_eq!(chip8.pc, 0x202);
    }

    #[test]
    fn test_c8_math_sub_reg_no_overflow() {
        // when
        let mut chip8 = Chip8::new(
            Target::Chip8,
            *profile::PROFILES.get(&Target::Chip8).unwrap(),
            KeyMapping::Qwerty,
            Box::new(rand::thread_rng()),
        )
        .expect("Could not initialise Chip8");
        chip8.memory[0x200] = 0x85;
        chip8.memory[0x201] = 0x45;
        chip8.v[5] = 0x17;
        chip8.v[4] = 0x15;

        // then
        chip8.emulate_cycle();

        // verify
        assert_eq!(chip8.v[5], 0x02);
        assert_eq!(chip8.v[15], 1);
        assert_eq!(chip8.pc, 0x202);
    }

    #[test]
    fn test_c8_math_neg_reg_overflow_borrow() {
        // when
        let mut chip8 = Chip8::new(
            Target::Chip8,
            *profile::PROFILES.get(&Target::Chip8).unwrap(),
            KeyMapping::Qwerty,
            Box::new(rand::thread_rng()),
        )
        .expect("Could not initialise Chip8");
        chip8.memory[0x200] = 0x84;
        chip8.memory[0x201] = 0x67;
        chip8.v[4] = 0x82;
        chip8.v[6] = 0x15;

        // then
        chip8.emulate_cycle();

        // verify
        assert_eq!(chip8.v[4], 0x93);
        assert_eq!(chip8.v[15], 0);
        assert_eq!(chip8.pc, 0x202);
    }

    #[test]
    fn test_c8_math_neg_reg_no_overflow() {
        // when
        let mut chip8 = Chip8::new(
            Target::Chip8,
            *profile::PROFILES.get(&Target::Chip8).unwrap(),
            KeyMapping::Qwerty,
            Box::new(rand::thread_rng()),
        )
        .expect("Could not initialise Chip8");
        chip8.memory[0x200] = 0x81;
        chip8.memory[0x201] = 0x37;
        chip8.v[1] = 0x13;
        chip8.v[3] = 0x15;

        // then
        chip8.emulate_cycle();

        // verify
        assert_eq!(chip8.v[1], 0x02);
        assert_eq!(chip8.v[15], 1);
        assert_eq!(chip8.pc, 0x202);
    }

    #[test]
    fn test_c8_cond_skip_eq_num_skips() {
        // when
        let mut chip8 = Chip8::new(
            Target::Chip8,
            *profile::PROFILES.get(&Target::Chip8).unwrap(),
            KeyMapping::Qwerty,
            Box::new(rand::thread_rng()),
        )
        .expect("Could not initialise Chip8");
        chip8.memory[0x200] = 0x33;
        chip8.memory[0x201] = 0x77;
        chip8.v[3] = 0x77;

        // then
        chip8.emulate_cycle();

        // verify
        assert_eq!(chip8.pc, 0x204);
    }

    #[test]
    fn test_c8_cond_skip_eq_num_does_not_skip() {
        // when
        let mut chip8 = Chip8::new(
            Target::Chip8,
            *profile::PROFILES.get(&Target::Chip8).unwrap(),
            KeyMapping::Qwerty,
            Box::new(rand::thread_rng()),
        )
        .expect("Could not initialise Chip8");
        chip8.memory[0x200] = 0x33;
        chip8.memory[0x201] = 0x75;
        chip8.v[3] = 0x77;

        // then
        chip8.emulate_cycle();

        // verify
        assert_eq!(chip8.pc, 0x202);
    }

    #[test]
    fn test_c8_cond_skip_neq_num_skips() {
        // when
        let mut chip8 = Chip8::new(
            Target::Chip8,
            *profile::PROFILES.get(&Target::Chip8).unwrap(),
            KeyMapping::Qwerty,
            Box::new(rand::thread_rng()),
        )
        .expect("Could not initialise Chip8");
        chip8.memory[0x200] = 0x43;
        chip8.memory[0x201] = 0x77;
        chip8.v[3] = 0x77;

        // then
        chip8.emulate_cycle();

        // verify
        assert_eq!(chip8.pc, 0x202);
    }

    #[test]
    fn test_c8_cond_skip_neq_num_does_not_skip() {
        // when
        let mut chip8 = Chip8::new(
            Target::Chip8,
            *profile::PROFILES.get(&Target::Chip8).unwrap(),
            KeyMapping::Qwerty,
            Box::new(rand::thread_rng()),
        )
        .expect("Could not initialise Chip8");
        chip8.memory[0x200] = 0x43;
        chip8.memory[0x201] = 0x75;
        chip8.v[3] = 0x77;

        // then
        chip8.emulate_cycle();

        // verify
        assert_eq!(chip8.pc, 0x204);
    }

    #[test]
    fn test_c8_cond_skipifneq_reg_skips() {
        // when
        let mut chip8 = Chip8::new(
            Target::Chip8,
            *profile::PROFILES.get(&Target::Chip8).unwrap(),
            KeyMapping::Qwerty,
            Box::new(rand::thread_rng()),
        )
        .expect("Could not initialise Chip8");
        chip8.memory[0x200] = 0x93;
        chip8.memory[0x201] = 0x10;
        chip8.v[3] = 1;
        chip8.v[1] = 2;

        // then
        chip8.emulate_cycle();

        // verify
        assert_eq!(chip8.pc, 0x204);
    }

    #[test]
    fn test_c8_cond_skipifneq_reg_does_not_skip() {
        // when
        let mut chip8 = Chip8::new(
            Target::Chip8,
            *profile::PROFILES.get(&Target::Chip8).unwrap(),
            KeyMapping::Qwerty,
            Box::new(rand::thread_rng()),
        )
        .expect("Could not initialise Chip8");
        chip8.memory[0x200] = 0x95;
        chip8.memory[0x201] = 0x40;
        chip8.v[5] = 2;
        chip8.v[4] = 2;

        // then
        chip8.emulate_cycle();

        // verify
        assert_eq!(chip8.pc, 0x202);
    }

    #[test]
    fn test_c8_mem_store() {
        // when
        let mut chip8 = Chip8::new(
            Target::Chip8,
            *profile::PROFILES.get(&Target::Chip8).unwrap(),
            KeyMapping::Qwerty,
            Box::new(rand::thread_rng()),
        )
        .expect("Could not initialise Chip8");
        chip8.memory[0x200] = 0xA4;
        chip8.memory[0x201] = 0xD8;

        // then
        chip8.emulate_cycle();

        // verify
        assert_eq!(chip8.i, 0x4D8);
        assert_eq!(chip8.pc, 0x202);
    }

    #[test]
    fn test_c8_flow_jump() {
        // when
        let mut chip8 = Chip8::new(
            Target::Chip8,
            *profile::PROFILES.get(&Target::Chip8).unwrap(),
            KeyMapping::Qwerty,
            Box::new(rand::thread_rng()),
        )
        .expect("Could not initialise Chip8");
        chip8.memory[0x200] = 0xB8;
        chip8.memory[0x201] = 0xB3;
        chip8.v[0] = 0x55;

        // then
        chip8.emulate_cycle();

        // verify
        assert_eq!(chip8.pc, 0x908);
    }

    #[test]
    fn test_c8_rand_and_reg() {
        // when
        let rng = StepRng::new(23, 2);
        let mut chip8 = Chip8::new(
            Target::Chip8,
            *profile::PROFILES.get(&Target::Chip8).unwrap(),
            KeyMapping::Qwerty,
            Box::new(rng),
        )
        .expect("Could not initialise Chip8");
        chip8.memory[0x200] = 0xC2;
        chip8.memory[0x201] = 0x66;
        chip8.v[2] = 0x23;

        // then
        chip8.emulate_cycle();

        // verify
        assert_eq!(chip8.v[2], 0x6);
        assert_eq!(chip8.pc, 0x202);
    }

    #[test]
    fn test_c8_draw_sprite_no_unset() {
        // when
        let mut chip8 = Chip8::new(
            Target::Chip8,
            *profile::PROFILES.get(&Target::Chip8).unwrap(),
            KeyMapping::Qwerty,
            Box::new(rand::thread_rng()),
        )
        .expect("Could not initialise Chip8");
        chip8.memory[0x200] = 0xD2;
        chip8.memory[0x201] = 0x32;
        chip8.v[2] = 0x7;
        chip8.v[3] = 0x9;
        chip8.i = 0x600;
        chip8.memory[0x600] = 0xFF;
        chip8.memory[0x601] = 0xFF;

        // then
        chip8.emulate_cycle();

        // verify
        assert_eq!(chip8.pc, 0x202);
        assert_eq!(chip8.gfx[0..583], [false; 583]);
        assert_eq!(chip8.gfx[583..591], [true; 8]);
        assert_eq!(chip8.gfx[591..647], [false; 56]);
        assert_eq!(chip8.gfx[647..655], [true; 8]);
        assert_eq!(chip8.gfx[655..2048], [false; 1393]);
        assert_eq!(chip8.v[15], 0);
    }

    #[test]
    fn test_c8_draw_sprite_unset() {
        // when
        let mut chip8 = Chip8::new(
            Target::Chip8,
            *profile::PROFILES.get(&Target::Chip8).unwrap(),
            KeyMapping::Qwerty,
            Box::new(rand::thread_rng()),
        )
        .expect("Could not initialise Chip8");
        chip8.gfx = Box::new([true; 64 * 32]);
        chip8.memory[0x200] = 0xD2;
        chip8.memory[0x201] = 0x32;
        chip8.v[2] = 0x7;
        chip8.v[3] = 0x9;
        chip8.i = 0x600;
        chip8.memory[0x600] = 0xFF;
        chip8.memory[0x601] = 0xFF;

        // then
        chip8.emulate_cycle();

        // verify
        assert_eq!(chip8.pc, 0x202);
        assert_eq!(chip8.gfx[0..583], [true; 583]);
        assert_eq!(chip8.gfx[583..591], [false; 8]);
        assert_eq!(chip8.gfx[591..647], [true; 56]);
        assert_eq!(chip8.gfx[647..655], [false; 8]);
        assert_eq!(chip8.gfx[655..2048], [true; 1393]);
        assert_eq!(chip8.v[15], 1);
    }

    // #[test]
    // fn test_c8_draw_sprite_overlap_x() {

    // }

    // #[test]
    // fn test_c8_draw_sprite_overlap_y() {

    // }

    // #[test]
    // fn test_c8_draw_sprite_overlap_x_and_y() {

    // }

    #[test]
    fn test_c8_key_pressedskip_pressed() {
        // when
        let mut chip8 = Chip8::new(
            Target::Chip8,
            *profile::PROFILES.get(&Target::Chip8).unwrap(),
            KeyMapping::Qwerty,
            Box::new(rand::thread_rng()),
        )
        .expect("Could not initialise Chip8");
        chip8.memory[0x200] = 0xE1;
        chip8.memory[0x201] = 0x9E;
        chip8.v[1] = 0x8;
        chip8.key[8] = true;

        // then
        chip8.emulate_cycle();

        // verify
        assert_eq!(chip8.pc, 0x204);
    }

    #[test]
    fn test_c8_key_pressedskip_notpressed() {
        // when
        let mut chip8 = Chip8::new(
            Target::Chip8,
            *profile::PROFILES.get(&Target::Chip8).unwrap(),
            KeyMapping::Qwerty,
            Box::new(rand::thread_rng()),
        )
        .expect("Could not initialise Chip8");
        chip8.memory[0x200] = 0xE1;
        chip8.memory[0x201] = 0x9E;
        chip8.v[1] = 0x7;
        chip8.key[7] = false;

        // then
        chip8.emulate_cycle();

        // verify
        assert_eq!(chip8.pc, 0x202);
    }

    #[test]
    fn test_c8_key_notpressedskip_pressed() {
        // when
        let mut chip8 = Chip8::new(
            Target::Chip8,
            *profile::PROFILES.get(&Target::Chip8).unwrap(),
            KeyMapping::Qwerty,
            Box::new(rand::thread_rng()),
        )
        .expect("Could not initialise Chip8");
        chip8.memory[0x200] = 0xE1;
        chip8.memory[0x201] = 0xA1;
        chip8.v[1] = 0x8;
        chip8.key[8] = true;

        // then
        chip8.emulate_cycle();

        // verify
        assert_eq!(chip8.pc, 0x202);
    }

    #[test]
    fn test_c8_key_notpressedskip_notpressed() {
        // when
        let mut chip8 = Chip8::new(
            Target::Chip8,
            *profile::PROFILES.get(&Target::Chip8).unwrap(),
            KeyMapping::Qwerty,
            Box::new(rand::thread_rng()),
        )
        .expect("Could not initialise Chip8");
        chip8.memory[0x200] = 0xE1;
        chip8.memory[0x201] = 0xA1;
        chip8.v[1] = 0x7;
        chip8.key[7] = false;

        // then
        chip8.emulate_cycle();

        // verify
        assert_eq!(chip8.pc, 0x204);
    }

    #[test]
    fn test_c8_timer_delay_store() {
        // when
        let mut chip8 = Chip8::new(
            Target::Chip8,
            *profile::PROFILES.get(&Target::Chip8).unwrap(),
            KeyMapping::Qwerty,
            Box::new(rand::thread_rng()),
        )
        .expect("Could not initialise Chip8");
        chip8.memory[0x200] = 0xF4;
        chip8.memory[0x201] = 0x07;
        chip8.v[4] = 0x3;
        chip8.delay_timer = 0x55;

        // then
        chip8.emulate_cycle();

        // verify
        assert_eq!(chip8.v[4], 0x55);
        assert_eq!(chip8.pc, 0x202);
    }

    #[test]
    fn test_c8_timer_delay_set() {
        // when
        let mut chip8 = Chip8::new(
            Target::Chip8,
            *profile::PROFILES.get(&Target::Chip8).unwrap(),
            KeyMapping::Qwerty,
            Box::new(rand::thread_rng()),
        )
        .expect("Could not initialise Chip8");
        chip8.memory[0x200] = 0xF1;
        chip8.memory[0x201] = 0x15;
        chip8.v[1] = 0x17;
        chip8.delay_timer = 0x55;

        // then
        chip8.emulate_cycle();

        // verify
        assert_eq!(chip8.delay_timer, 0x17);
        assert_eq!(chip8.pc, 0x202);
    }

    #[test]
    fn test_c8_timer_sound_set() {
        // when
        let mut chip8 = Chip8::new(
            Target::Chip8,
            *profile::PROFILES.get(&Target::Chip8).unwrap(),
            KeyMapping::Qwerty,
            Box::new(rand::thread_rng()),
        )
        .expect("Could not initialise Chip8");
        chip8.memory[0x200] = 0xF9;
        chip8.memory[0x201] = 0x18;
        chip8.v[9] = 0x22;
        chip8.sound_timer = 0x15;

        // then
        chip8.emulate_cycle();

        // verify
        assert_eq!(chip8.sound_timer, 0x22);
        assert_eq!(chip8.pc, 0x202);
    }

    #[test]
    fn test_c8_mem_addi_no_overflow() {
        // when
        let mut chip8 = Chip8::new(
            Target::Chip8,
            *profile::PROFILES.get(&Target::Chip8).unwrap(),
            KeyMapping::Qwerty,
            Box::new(rand::thread_rng()),
        )
        .expect("Could not initialise Chip8");
        chip8.memory[0x200] = 0xF6;
        chip8.memory[0x201] = 0x1E;
        chip8.v[6] = 0x44;
        chip8.i = 0x3;
        chip8.v[15] = 0;

        // then
        chip8.emulate_cycle();

        // verify
        assert_eq!(chip8.i, 0x47);
        assert_eq!(chip8.v[15], 0);
        assert_eq!(chip8.pc, 0x202);
    }

    #[test]
    fn test_c8_mem_addi_overflow() {
        // when
        let mut chip8 = Chip8::new(
            Target::Chip8,
            *profile::PROFILES.get(&Target::Chip8).unwrap(),
            KeyMapping::Qwerty,
            Box::new(rand::thread_rng()),
        )
        .expect("Could not initialise Chip8");
        chip8.memory[0x200] = 0xF3;
        chip8.memory[0x201] = 0x1E;
        chip8.v[3] = 0x04;
        chip8.i = 0xFFE;
        chip8.v[15] = 0;

        // then
        chip8.emulate_cycle();

        // verify
        assert_eq!(chip8.i, 0x2);
        assert_eq!(chip8.v[15], 1);
        assert_eq!(chip8.pc, 0x202);
    }

    #[test]
    fn test_c8_mem_reg_dump() {
        // when
        let mut chip8 = Chip8::new(
            Target::Chip8,
            *profile::PROFILES.get(&Target::Chip8).unwrap(),
            KeyMapping::Qwerty,
            Box::new(rand::thread_rng()),
        )
        .expect("Could not initialise Chip8");
        chip8.i = 0xC60;
        chip8.memory[0x200] = 0xF3;
        chip8.memory[0x201] = 0x55;
        chip8.v[0] = 0xDE;
        chip8.v[1] = 0xAD;
        chip8.v[2] = 0xBE;
        let expected_memory: [u8; 5] = [0xDE, 0xAD, 0xBE, 0, 0];

        // then
        chip8.emulate_cycle();

        // verify
        assert_eq!(chip8.memory[0xC60..0xC65], expected_memory);
        assert_eq!(chip8.i, 0xC64);
        assert_eq!(chip8.pc, 0x202);
    }

    #[test]
    fn test_sc_mem_reg_dump() {
        // when
        let mut chip8 = Chip8::new(
            Target::SuperChip,
            *profile::PROFILES.get(&Target::SuperChip).unwrap(),
            KeyMapping::Qwerty,
            Box::new(rand::thread_rng()),
        )
        .expect("Could not initialise SuperChip");
        chip8.i = 0xC60;
        chip8.memory[0x200] = 0xF3;
        chip8.memory[0x201] = 0x55;
        chip8.v[0] = 0xDE;
        chip8.v[1] = 0xAD;
        chip8.v[2] = 0xBE;
        let expected_memory: [u8; 5] = [0xDE, 0xAD, 0xBE, 0, 0];

        // then
        chip8.emulate_cycle();

        // verify
        assert_eq!(chip8.memory[0xC60..0xC65], expected_memory);
        assert_eq!(chip8.i, 0xC60);
        assert_eq!(chip8.pc, 0x202);
    }

    #[test]
    fn test_c8_mem_reg_load() {
        // when
        let mut chip8 = Chip8::new(
            Target::Chip8,
            *profile::PROFILES.get(&Target::Chip8).unwrap(),
            KeyMapping::Qwerty,
            Box::new(rand::thread_rng()),
        )
        .expect("Could not initialise Chip8");
        chip8.i = 0xD00;
        chip8.memory[0x200] = 0xF4;
        chip8.memory[0x201] = 0x65;
        chip8.memory[0xd00] = 0xDE;
        chip8.memory[0xd01] = 0xAD;
        chip8.memory[0xd02] = 0xBE;
        chip8.memory[0xd03] = 0xEF;
        let expected_registers: [u8; 6] = [0xDE, 0xAD, 0xBE, 0xEF, 0, 0];

        // then
        chip8.emulate_cycle();

        // verify
        assert_eq!(chip8.v[0..6], expected_registers);
        assert_eq!(chip8.i, 0xD05);
        assert_eq!(chip8.pc, 0x202);
    }

    #[test]
    fn test_sc_mem_reg_load() {
        // when
        let mut chip8 = Chip8::new(
            Target::SuperChip,
            *profile::PROFILES.get(&Target::SuperChip).unwrap(),
            KeyMapping::Qwerty,
            Box::new(rand::thread_rng()),
        )
        .expect("Could not initialise Chip8");
        chip8.i = 0xD00;
        chip8.memory[0x200] = 0xF4;
        chip8.memory[0x201] = 0x65;
        chip8.memory[0xd00] = 0xDE;
        chip8.memory[0xd01] = 0xAD;
        chip8.memory[0xd02] = 0xBE;
        chip8.memory[0xd03] = 0xEF;
        let expected_registers: [u8; 6] = [0xDE, 0xAD, 0xBE, 0xEF, 0, 0];

        // then
        chip8.emulate_cycle();

        // verify
        assert_eq!(chip8.v[0..6], expected_registers);
        assert_eq!(chip8.i, 0xD00);
        assert_eq!(chip8.pc, 0x202);
    }

    #[test]
    fn test_unknown_opcode_panics() {
        let result = catch_unwind_silent(|| {
            // when
            let mut chip8 = Chip8::new(
                Target::Chip8,
                *profile::PROFILES.get(&Target::Chip8).unwrap(),
                KeyMapping::Qwerty,
                Box::new(rand::thread_rng()),
            )
            .expect("Could not initialise Chip8");
            chip8.memory[0x200] = 0xFF;
            chip8.memory[0x201] = 0xFF;

            // then
            chip8.emulate_cycle();
        });

        // verify
        assert!(result.is_err());
    }

    fn catch_unwind_silent<F: FnOnce() -> R + panic::UnwindSafe, R>(
        f: F,
    ) -> std::thread::Result<R> {
        let prev_hook = panic::take_hook();
        panic::set_hook(Box::new(|_| {}));
        let result = panic::catch_unwind(f);
        panic::set_hook(prev_hook);
        result
    }
}
