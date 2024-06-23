use std::collections::HashMap;
use std::sync::OnceLock;

use crate::Target;

#[derive(Debug, Copy, Clone)]
pub(crate) struct Profile {
    screen_width: u8,
    screen_height: u8,
    lores_display_wait: bool,
    default_screen_scale: u8,
    memory_capacity: usize,
    user_register_count: u8,
}

impl Profile {
    #[must_use]
    pub(crate) fn screen_width(self) -> u8 {
        self.screen_width
    }

    #[must_use]
    pub(crate) fn screen_height(self) -> u8 {
        self.screen_height
    }

    #[must_use]
    pub(crate) fn lores_display_wait(self) -> bool {
        self.lores_display_wait
    }

    #[must_use]
    pub(crate) fn default_screen_scale(self) -> u8 {
        self.default_screen_scale
    }

    #[must_use]
    pub(crate) fn memory_capacity(self) -> usize {
        self.memory_capacity
    }

    #[must_use]
    pub(crate) fn user_register_count(self) -> u8 {
        self.user_register_count
    }
}

pub(crate) fn profiles() -> &'static HashMap<Target, Profile> {
    static LOCK: OnceLock<HashMap<Target, Profile>> = OnceLock::new();
    LOCK.get_or_init(|| {
        HashMap::from([
            (
                Target::Chip8,
                Profile {
                    screen_width: 64,
                    screen_height: 32,
                    lores_display_wait: true,
                    default_screen_scale: 12,
                    memory_capacity: 4_096,
                    user_register_count: 0,
                },
            ),
            (
                Target::SuperChipLegacy,
                Profile {
                    screen_width: 128,
                    screen_height: 64,
                    lores_display_wait: true,
                    default_screen_scale: 6,
                    memory_capacity: 4_096,
                    user_register_count: 8,
                },
            ),
            (
                Target::SuperChip,
                Profile {
                    screen_width: 128,
                    screen_height: 64,
                    lores_display_wait: false,
                    default_screen_scale: 6,
                    memory_capacity: 4_096,
                    user_register_count: 8,
                },
            ),
            (
                Target::XoChip,
                Profile {
                    screen_width: 128,
                    screen_height: 64,
                    lores_display_wait: false,
                    default_screen_scale: 6,
                    memory_capacity: 65_536,
                    user_register_count: 16,
                },
            ),
        ])
    })
}
