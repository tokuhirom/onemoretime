use std::collections::VecDeque;
use core_graphics::event::{CGEventFlags, CGKeyCode};

use std::ops::BitAnd;
use crate::event::Event;
use crate::key::Key;
use crate::keycodes::key_from_code;
use crate::KeyState;

use crate::sender::Sender;
use crate::shortcut::Shortcut;
use crate::state::State;

pub struct Handler {
    buffer: VecDeque<KeyState>,
    capacity: usize,
    playing: bool, // TODO remove this?
    latest_flags: CGEventFlags,
    shortcut_pressed: bool, // TODO remove this?
    shortcut: Shortcut,
}

impl Handler {
    pub fn new(capacity: usize, shortcut: Shortcut) -> Handler {
        Handler {
            buffer: VecDeque::with_capacity(capacity),
            capacity,
            playing: false,
            latest_flags: CGEventFlags::CGEventFlagNonCoalesced,
            shortcut_pressed: false,
            shortcut,
        }
    }

    pub fn callback(&mut self, event: Event) -> Option<Event> {
        if self.playing {
            log::info!("Ignoring event due to playing macro now: {:?}", event);
            return Some(event);
        }

        match event {
            Event::KeyPress(code) => {
                if is_shortcut_pressed(self.latest_flags, code, &self.shortcut) {
                    log::info!("Shortcut key pressed!! 444");
                    self.shortcut_pressed = true;

                    let sender = Sender::new();
                    if let Err(err) = sender.process(State::new(
                        self.buffer.clone(),
                        self.latest_flags
                    )) {
                        log::error!("Cannot process shortcut: {:?}", err);
                    }
                    return None;
                }

                // fill buffer
                self.buffer.push_front(KeyState {
                    code,
                    flags: self.latest_flags
                });
                if self.capacity < self.buffer.len() {
                    self.buffer.pop_back();
                }
                log::info!("pressed~~~ code={}, buffer={:?}", code, self.buffer);
            }
            Event::KeyRelease(_code) => {
            }
            Event::FlagsChanged(key, flags) => {
                log::info!("Flags changed: key={:?}, flags={:?}", key, flags);
                if self.shortcut_pressed && !self.is_modifier_pressing(flags) {
                    self.shortcut_pressed = false;
                    return Some(event);
                }

                self.latest_flags = flags;
            }
        }
        Some(event)
    }

    fn is_modifier_pressing(&self, flags: CGEventFlags) -> bool {
        let modifiers = vec![
            CGEventFlags::CGEventFlagControl,
            CGEventFlags::CGEventFlagAlternate,
            CGEventFlags::CGEventFlagShift,
            CGEventFlags::CGEventFlagCommand,
        ];
        for modifier in modifiers {
            if !(flags & modifier).is_empty() {
                log::info!("Pressing moldifier: {:?}, {}", modifier, (flags & modifier).is_empty());
                return true;
            }
        }
        false
    }
}

fn is_shortcut_pressed(flags: CGEventFlags, code: CGKeyCode, shortcut: &Shortcut) -> bool {
    let expected_flags = shortcut.flags;
    let expected_code = shortcut.keycode;

    log::info!("is_shortcut_pressed?: flags={:?} code={:?}, expected({:?}, {:?})",
        flags, code,
        expected_flags, expected_code);

    // 全てのキー修飾フラグを取得
    let all_modifiers = CGEventFlags::CGEventFlagControl
        | CGEventFlags::CGEventFlagAlternate
        | CGEventFlags::CGEventFlagShift
        | CGEventFlags::CGEventFlagCommand;

    // 期待するフラグだけが押されていて、それ以外のフラグは押されていないことをチェック
    let is_correct_flags_pressed = flags & all_modifiers == expected_flags;

    // キーコードが期待通りであることをチェック
    let is_correct_keycode = code == expected_code;

    is_correct_flags_pressed && is_correct_keycode
}

