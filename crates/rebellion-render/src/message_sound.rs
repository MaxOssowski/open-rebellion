//! The sounds a message plays when the Message window displays it
//! (`FUN_00469de0`).
//!
//! A message node carries two WAVE ids: `+0x30` from DLL slot 7
//! (STRATEGY.DLL, `FUN_00421c70`) and `+0x32` from slot 9, the side's
//! sprite DLL (`FUN_0042d650`). On each display the window first stops the
//! two sounds it last started (`FUN_0046a6e0`: `+0x128` / `+0x140`, each
//! when its id `+0x12c` / `+0x144` is not 0), then keeps the new ids and
//! plays each one that is not 0 (`FUN_00610c30`). So every display cuts the
//! previous message's sounds; A0 `cmd07-c011-preempt-attempt.wav` shows
//! three arrow presses each restarting STRATEGY WAVE 1122, only the last
//! playing out.

use crate::advisor_clip::{SPRITE_SLOT, slot_directory};
use crate::advisor::AdvisorFaction;

/// The staged directory of DLL slot 7, STRATEGY.DLL (`FUN_00421c70`).
pub const STRATEGY_DIRECTORY: &str = "strategy-dll";

/// A message's display sounds: `+0x30` (STRATEGY.DLL) and `+0x32` (the
/// side's sprite DLL), 0 for none.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MessageSounds {
    pub strategy: u16,
    pub sprite: u16,
}

impl MessageSounds {
    /// No sound: the base class's `FUN_004c4d90` zeroes both words.
    pub const NONE: Self = Self {
        strategy: 0,
        sprite: 0,
    };

    /// The advice message's (`FUN_0048b2e0`) and the other side's message's
    /// (`FUN_00498e90`): `+0x30` 0x461 for the Alliance (side 1), 0x462 for
    /// the Empire.
    #[must_use]
    pub const fn advice(alliance: bool) -> Self {
        Self {
            strategy: if alliance { 0x461 } else { 0x462 },
            sprite: 0,
        }
    }
}

/// What the owner does with its sound engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageSoundCommand {
    /// Stop a sound the previous display started (`FUN_00610c80`).
    Stop { directory: &'static str, wave: u16 },
    /// Start a sound (`FUN_00610c30`).
    Play { directory: &'static str, wave: u16 },
}

/// The Message window's two sound objects (`+0x128`, `+0x140`).
#[derive(Debug, Clone, Default)]
pub struct MessageSoundPlayer {
    playing: [Option<(&'static str, u16)>; 2],
}

impl MessageSoundPlayer {
    /// Display a message with `sounds` for the player's `faction`.
    pub fn display(
        &mut self,
        sounds: MessageSounds,
        faction: AdvisorFaction,
    ) -> Vec<MessageSoundCommand> {
        // FUN_0046a6e0: stop what the last display started.
        let mut commands: Vec<MessageSoundCommand> = self
            .playing
            .iter()
            .flatten()
            .map(|&(directory, wave)| MessageSoundCommand::Stop { directory, wave })
            .collect();
        let sprite = slot_directory(SPRITE_SLOT, faction).unwrap_or("alsprite-dll");
        self.playing = [
            (sounds.strategy != 0).then_some((STRATEGY_DIRECTORY, sounds.strategy)),
            (sounds.sprite != 0).then_some((sprite, sounds.sprite)),
        ];
        commands.extend(
            self.playing
                .iter()
                .flatten()
                .map(|&(directory, wave)| MessageSoundCommand::Play { directory, wave }),
        );
        commands
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use MessageSoundCommand::{Play, Stop};

    #[test]
    fn displaying_an_advice_message_plays_its_strategy_wave() {
        // FUN_0048b2e0: +0x30 = 0x462 (1122) for the Empire; A0
        // cmd07-c011-preempt-attempt.wav: STRATEGY 1122 at 46.65 s.
        let mut player = MessageSoundPlayer::default();
        assert_eq!(
            player.display(MessageSounds::advice(false), AdvisorFaction::Empire),
            [Play {
                directory: STRATEGY_DIRECTORY,
                wave: 1122
            }]
        );
    }

    #[test]
    fn each_display_stops_the_previous_messages_sounds_first() {
        // FUN_0046a6e0 runs before the new ids are kept (FUN_00469de0).
        let mut player = MessageSoundPlayer::default();
        player.display(MessageSounds::advice(true), AdvisorFaction::Alliance);
        let voiced = MessageSounds {
            strategy: 0x46d,
            sprite: 1185,
        };
        assert_eq!(
            player.display(voiced, AdvisorFaction::Alliance),
            [
                Stop {
                    directory: STRATEGY_DIRECTORY,
                    wave: 1121
                },
                Play {
                    directory: STRATEGY_DIRECTORY,
                    wave: 0x46d
                },
                Play {
                    directory: "alsprite-dll",
                    wave: 1185
                },
            ]
        );
        assert_eq!(
            player.display(MessageSounds::NONE, AdvisorFaction::Alliance),
            [
                Stop {
                    directory: STRATEGY_DIRECTORY,
                    wave: 0x46d
                },
                Stop {
                    directory: "alsprite-dll",
                    wave: 1185
                },
            ]
        );
        // A silent message leaves nothing to stop.
        assert!(player.display(MessageSounds::NONE, AdvisorFaction::Alliance).is_empty());
    }

    #[test]
    fn the_second_sound_comes_from_the_empires_sprite_dll_for_the_empire() {
        // FUN_00469de0: +0x32 plays from slot 9 (FUN_0042d650: emsprite.dll
        // for a side other than 1).
        let mut player = MessageSoundPlayer::default();
        let sounds = MessageSounds {
            strategy: 0,
            sprite: 1185,
        };
        assert_eq!(
            player.display(sounds, AdvisorFaction::Empire),
            [Play {
                directory: "emsprite-dll",
                wave: 1185
            }]
        );
    }
}
