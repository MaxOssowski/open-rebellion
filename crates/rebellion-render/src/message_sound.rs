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
use crate::story_messages::CharacterHealth;

/// The staged directory of DLL slot 7, STRATEGY.DLL (`FUN_00421c70`).
pub const STRATEGY_DIRECTORY: &str = "strategy-dll";

/// A message's display sounds: `+0x30` (STRATEGY.DLL) and `+0x32` (the
/// side's sprite DLL), 0 for none.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MessageSounds {
    pub strategy: u16,
    pub sprite: u16,
    /// The sprite DLL's `RT_RCDATA` voice list `+0x32` is drawn from
    /// (`FUN_004c4990`'s argument), until [`Self::drawn`] draws it; 0 for
    /// none.
    pub voice: u16,
}

impl MessageSounds {
    /// No sound: the base class's `FUN_004c4d90` zeroes both words.
    pub const NONE: Self = Self {
        strategy: 0,
        sprite: 0,
        voice: 0,
    };

    /// `+0x30` alone.
    #[must_use]
    pub const fn strategy(strategy: u16) -> Self {
        Self {
            strategy,
            sprite: 0,
            voice: 0,
        }
    }

    /// `+0x30` written as `id + (side != 1)`: `id` for the Alliance, the
    /// next WAVE for the Empire.
    #[must_use]
    pub const fn by_side(id: u16, alliance: bool) -> Self {
        Self::strategy(if alliance { id } else { id + 1 })
    }

    /// `+0x32 = FUN_004c4990(list)`: a voice drawn from the sprite DLL's
    /// `RT_RCDATA` `list` when the message is filed ([`Self::drawn`]).
    #[must_use]
    pub const fn with_voice(mut self, list: u16) -> Self {
        self.voice = list;
        self
    }

    /// Draw the voice (`FUN_004c4990`): the list's only WAVE, or for a
    /// longer list the one at `draw(count)` (`FUN_0041cd80`, a draw below
    /// `count`); 0 when the list is missing. `draw` is called only for a
    /// list of more than one.
    #[must_use]
    pub fn drawn(mut self, draw: impl FnOnce(u16) -> u16) -> Self {
        if self.voice != 0 {
            self.sprite = voice_wave(self.voice, draw);
            self.voice = 0;
        }
        self
    }

    /// The advice message's (`FUN_0048b2e0`) and the other side's message's
    /// (`FUN_00498e90`): `+0x30` 0x461 for the Alliance (side 1), 0x462 for
    /// the Empire.
    #[must_use]
    pub const fn advice(alliance: bool) -> Self {
        Self::by_side(0x461, alliance)
    }
}

/// The sprite DLLs' voice lists (`RT_RCDATA` `0xc8..0x139`, a count and
/// then the WAVE ids; ALSPRITE.DLL and EMSPRITE.DLL hold the same lists).
const VOICE_LISTS: &[(u16, &[u16])] = &[
    (0xc8, &[1311, 1312]),
    (0xc9, &[1301, 1302, 1303]),
    (0xca, &[1304, 1305, 1306]),
    (0xcb, &[1307, 1308]),
    (0xcc, &[1306]),
    (0xcd, &[1313, 1314]),
    (0xce, &[1315, 1316]),
    (0xcf, &[1317]),
    (0xd0, &[1319]),
    (0xd1, &[1318]),
    (0xd2, &[1322, 1323]),
    (0xd3, &[1301, 1302, 1303]),
    (0xd4, &[1304, 1305, 1306]),
    (0xd5, &[1307, 1308]),
    (0xd6, &[1306]),
    (0xd7, &[1324]),
    (0xd8, &[1325]),
    (0xd9, &[1326]),
    (0xda, &[1328]),
    (0xdb, &[1328]),
    (0xdc, &[1344]),
    (0xdd, &[1333, 1334]),
    (0xde, &[1335, 1336]),
    (0xdf, &[1337, 1338]),
    (0xe0, &[1336]),
    (0xe1, &[1345]),
    (0xe2, &[1346]),
    (0xe3, &[1347]),
    (0xe4, &[1351]),
    (0xe5, &[1348]),
    (0xe6, &[1361, 1362]),
    (0xe7, &[1353]),
    (0xe8, &[1354, 1355, 1356, 1357]),
    (0xe9, &[1358, 1359]),
    (0xea, &[1354, 1355, 1356, 1357]),
    (0xeb, &[1363, 1364]),
    (0xec, &[1365]),
    (0xed, &[1366]),
    (0xf0, &[1372]),
    (0xf1, &[1368]),
    (0xf2, &[1369]),
    (0xf3, &[1370]),
    (0xf4, &[1369]),
    (0xf5, &[1373]),
    (0xf6, &[1374]),
    (0xf7, &[1375]),
    (0xfa, &[1379]),
    (0xfb, &[1376]),
    (0xfc, &[1377]),
    (0xfd, &[1378]),
    (0xfe, &[1377]),
    (0xff, &[1380]),
    (0x100, &[1381]),
    (0x101, &[1382]),
    (0x102, &[1383]),
    (0x104, &[1388]),
    (0x105, &[1385]),
    (0x106, &[1386]),
    (0x107, &[1387]),
    (0x108, &[1386]),
    (0x109, &[1389]),
    (0x10a, &[1390]),
    (0x10b, &[1391]),
    (0x10c, &[1392]),
    (0x118, &[1399]),
    (0x119, &[1400]),
    (0x11a, &[1401]),
    (0x11b, &[1402]),
    (0x11c, &[1404]),
    (0x11d, &[1395]),
    (0x11e, &[1397]),
    (0x11f, &[1398]),
    (0x120, &[1367]),
    (0x121, &[1332]),
    (0x123, &[1407]),
    (0x124, &[1349]),
    (0x125, &[1321]),
    (0x126, &[1321]),
    (0x127, &[1352]),
    (0x128, &[1384]),
    (0x129, &[1393]),
    (0x12f, &[1403]),
    (0x130, &[1394]),
    (0x131, &[1310]),
    (0x132, &[1320]),
    (0x133, &[1329]),
    (0x134, &[1350]),
    (0x135, &[1331]),
    (0x137, &[1405]),
    (0x138, &[1406]),
    (0x139, &[1408]),
];

/// `FUN_004c4990(list)`: the WAVE a voice list gives, `draw(count)` picking
/// among several (a draw below `count`, `FUN_0041cd80`); 0 for a missing
/// list (`FindResourceA` fails), as for `0x136`.
#[must_use]
pub fn voice_wave(list: u16, draw: impl FnOnce(u16) -> u16) -> u16 {
    let Some((_, waves)) = VOICE_LISTS.iter().find(|(id, _)| *id == list) else {
        return 0;
    };
    match waves {
        [only] => *only,
        _ => {
            let count = u16::try_from(waves.len()).unwrap_or(u16::MAX);
            waves[usize::from(draw(count).min(count - 1))]
        }
    }
}


/// A bounty hunters' message (`FUN_0048fa00`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bounty {
    /// Han captured (`0x716c` / `0x716d`), both sides.
    Captured,
    /// Han attacked (`0x7168` / `0x7169`), the Alliance.
    Attacked,
    /// Han located (`0x716a` / `0x716b`), the Empire.
    Located,
}

/// A major character's first voice list: the switches on the character's
/// index (`FUN_004955b0` L192..226, `FUN_00490340`, `FUN_0048fdf0`,
/// `FUN_004927c0`, `FUN_00491500`) take this base plus a fixed offset.
/// Luke's list moves from 0xc8 to 0xd2 once his `+0xb0` bit 3 (Dagobah
/// completed) is set; no other character tests it.
const fn character_voice_base(character: u32, dagobah: bool) -> Option<u16> {
    match character {
        0x240 => Some(0xf0),
        0x241 => Some(0xdc),
        0x242 if dagobah => Some(0xd2),
        0x242 => Some(0xc8),
        0x243 => Some(0xe6),
        0x280 => Some(0x104),
        0x281 => Some(0xfa),
        _ => None,
    }
}

/// The voice `offset` lists past the character's base, 0 for a character
/// with no voice.
const fn character_voice(character: u32, dagobah: bool, offset: u16) -> u16 {
    match character_voice_base(character, dagobah) {
        Some(base) => base + offset,
        None => 0,
    }
}

impl MessageSounds {
    /// Facility Idle (`FUN_0048be60`): `0x44d`.
    #[must_use]
    pub const fn facility_idle() -> Self {
        Self::strategy(0x44d)
    }

    /// A resource change (`FUN_0048e940`, picture `0x3ea`): `0x44d`.
    #[must_use]
    pub const fn resources() -> Self {
        Self::strategy(0x44d)
    }

    /// A disaster (`FUN_0048e940`, `+100` bit 3): `0x44e`.
    #[must_use]
    pub const fn disaster() -> Self {
        Self::strategy(0x44e)
    }

    /// Smuggling (`FUN_00499130`): `0x44f` when it begins (`param_4` not
    /// 0), nothing when it ends.
    #[must_use]
    pub const fn smuggling(begins: bool) -> Self {
        if begins {
            Self::strategy(0x44f)
        } else {
            Self::NONE
        }
    }

    /// A system changes hands (`FUN_00499760`): `0x450` when a side gains
    /// it, `0x451` when it goes neutral.
    #[must_use]
    pub const fn control_change(to_a_side: bool) -> Self {
        Self::strategy(if to_a_side { 0x450 } else { 0x451 })
    }

    /// Near Uprising (`FUN_00498cb0`): `0x452` for an Alliance system,
    /// `0x453` for an Empire one.
    #[must_use]
    pub const fn near_uprising(alliance_system: bool) -> Self {
        Self::strategy(if alliance_system { 0x452 } else { 0x453 })
    }

    /// Uprising Begins / Ends (`FUN_00499460`): `0x454` / `0x455`.
    #[must_use]
    pub const fn uprising(begins: bool) -> Self {
        Self::strategy(if begins { 0x454 } else { 0x455 })
    }

    /// Units destroyed: Saboteurs Strike (`FUN_0048b9a0`), Maintenance
    /// Shortfall (`FUN_00498970`), both through `FUN_0048bbf0`: `0x456`.
    #[must_use]
    pub const fn units_destroyed() -> Self {
        Self::strategy(0x456)
    }

    /// Deployed (`FUN_00497940`): only case 1, a fighter squadron
    /// (`0x708a`), writes `0x457 + (side != 1)`.
    #[must_use]
    pub const fn deployed(fighter: bool, alliance: bool) -> Self {
        if fighter {
            Self::by_side(0x457, alliance)
        } else {
            Self::NONE
        }
    }

    /// Research Complete (`FUN_00499aa0`) and Exhausted (`FUN_0048abc0`):
    /// `0x459`.
    #[must_use]
    pub const fn research() -> Self {
        Self::strategy(0x459)
    }

    /// Unit Arrival (`FUN_004981c0`): `0x45a` for ships or a fleet (cases
    /// 1, 2), `0x45b + (side != 1)` for other units (case 3), `0x45d` for
    /// the Headquarters (case 4).
    #[must_use]
    pub const fn arrival(what: crate::message_pictures::Arrival, alliance: bool) -> Self {
        use crate::message_pictures::Arrival;
        match what {
            Arrival::Fleet => Self::strategy(0x45a),
            Arrival::Units => Self::by_side(0x45b, alliance),
            Arrival::Headquarters => Self::strategy(0x45d),
        }
    }

    /// A blockade (`FUN_004960f0`): `0x45e`.
    #[must_use]
    pub const fn blockade() -> Self {
        Self::strategy(0x45e)
    }

    /// Evacuation Losses (`FUN_00495e90`): `0x460` for reader 1, else
    /// `0x45f`.
    #[must_use]
    pub const fn evacuation_losses(alliance: bool) -> Self {
        Self::strategy(if alliance { 0x460 } else { 0x45f })
    }

    /// Informants (`FUN_00491060`): `0x44c`.
    #[must_use]
    pub const fn informants() -> Self {
        Self::strategy(0x44c)
    }

    /// A character captured (`FUN_00490340` cases 3, 4): the default
    /// `0x44c`.
    #[must_use]
    pub const fn captured() -> Self {
        Self::strategy(0x44c)
    }

    /// A character captured by Jabba (`FUN_00490340` case 5): `0x478`.
    #[must_use]
    pub const fn jabba_captures() -> Self {
        Self::strategy(0x478)
    }

    /// A character escaped (`FUN_00490340` cases 1, 2): `0x469`; its own
    /// side (case 1) hears the escaper's voice, its base list plus 6.
    #[must_use]
    pub const fn escaped(own_side: bool, character: u32, dagobah: bool) -> Self {
        let voice = if own_side {
            character_voice(character, dagobah, 6)
        } else {
            0
        };
        Self::strategy(0x469).with_voice(voice)
    }

    /// Character Health (`FUN_0048fdf0`): `0x474`; a healed character
    /// speaks its base list plus 7.
    #[must_use]
    pub const fn character_health(health: CharacterHealth, character: u32, dagobah: bool) -> Self {
        let voice = match health {
            CharacterHealth::Healed => character_voice(character, dagobah, 7),
            CharacterHealth::Injured | CharacterHealth::Killed => 0,
        };
        Self::strategy(0x474).with_voice(voice)
    }

    /// Killed by Imperial Assassins (`FUN_0048b050`): `0x474`.
    #[must_use]
    pub const fn assassins() -> Self {
        Self::strategy(0x474)
    }

    /// A confrontation (`FUN_0048df20`), by whether the Alliance character
    /// is Luke, his Dagobah bit and whether the Empire's is Vader.
    #[must_use]
    pub const fn confrontation(luke: bool, dagobah: bool, vader: bool) -> Self {
        let (strategy, voice) = match (luke, dagobah, vader) {
            (false, _, true) => (0x46a, 0x137),
            (false, _, false) => (0x46b, 0x138),
            (true, false, true) => (0x46e, 0x118),
            (true, false, false) => (0x470, 0x119),
            (true, true, true) => (0x46f, 0x11a),
            (true, true, false) => (0x471, 0x11b),
        };
        Self::strategy(strategy).with_voice(voice)
    }

    /// Luke Goes to Dagobah (`FUN_0048d190`).
    #[must_use]
    pub const fn luke_to_dagobah() -> Self {
        Self::strategy(0x46c).with_voice(0x11d)
    }

    /// Luke Discovers His Heritage (`FUN_0048dc50`).
    #[must_use]
    pub const fn heritage() -> Self {
        Self::strategy(0x46d).with_voice(0x11e)
    }

    /// The Final Battle (`FUN_0048e3c0`), by Luke's Dagobah bit.
    #[must_use]
    pub const fn final_battle(dagobah: bool) -> Self {
        if dagobah {
            Self::strategy(0x473).with_voice(0x12f)
        } else {
            Self::strategy(0x472).with_voice(0x11c)
        }
    }

    /// Jabba Delivers Prisoners (`FUN_0048b6e0`), by Luke's Dagobah bit.
    #[must_use]
    pub const fn jabba_delivers(dagobah: bool) -> Self {
        if dagobah {
            Self::strategy(0x47a).with_voice(0x139)
        } else {
            Self::strategy(0x479).with_voice(0x123)
        }
    }

    /// Emperor Arrives at Coruscant (`FUN_0048ce80`).
    #[must_use]
    pub const fn emperor_arrives() -> Self {
        Self::strategy(0x47b).with_voice(0x130)
    }

    /// The bounty hunters (`FUN_0048fa00`).
    #[must_use]
    pub const fn bounty(what: Bounty) -> Self {
        match what {
            Bounty::Captured => Self::strategy(0x476).with_voice(0x11f),
            Bounty::Attacked => Self::strategy(0x475).with_voice(0x120),
            Bounty::Located => Self::strategy(0x477),
        }
    }

    /// Luke Leaves Dagobah (`FUN_0048d0f0`): `0x47c + (side != 1)`, side 1
    /// only.
    #[must_use]
    pub const fn luke_leaves_dagobah() -> Self {
        Self::strategy(0x47c)
    }

    /// Rescue Attempt (`FUN_0048c4c0`): the rescuer's voice, Leia `0xe5`,
    /// Luke `0xd1` / `0xdb`.
    #[must_use]
    pub const fn rescue_attempt(alliance: bool, rescuer: u32, dagobah: bool) -> Self {
        let voice = match rescuer {
            0x241 => 0xe5,
            0x242 if dagobah => 0xdb,
            0x242 => 0xd1,
            _ => 0,
        };
        Self::by_side(0x47c, alliance).with_voice(voice)
    }

    /// Discovers Traitor (`FUN_0048c9c0`): the discoverer's voice.
    #[must_use]
    pub const fn discovers_traitor(alliance: bool, discoverer: u32, dagobah: bool) -> Self {
        let voice = match discoverer {
            0x241 => 0x127,
            0x242 if dagobah => 0x126,
            0x242 => 0x125,
            0x280 => 0x129,
            0x281 => 0x128,
            _ => 0,
        };
        Self::by_side(0x47c, alliance).with_voice(voice)
    }

    /// Detects Enemy (`FUN_0048e1d0`): the reader's own character's voice.
    #[must_use]
    pub const fn detects_enemy(alliance: bool, own: u32, dagobah: bool) -> Self {
        let voice = match own {
            0x241 => 0xe4,
            0x242 if dagobah => 0xda,
            0x242 => 0xd0,
            _ => 0,
        };
        Self::by_side(0x47c, alliance).with_voice(voice)
    }

    /// Force Growth (`FUN_0048ed80`): the character's voice.
    #[must_use]
    pub const fn force_growth(alliance: bool, character: u32, dagobah: bool) -> Self {
        let voice = match character {
            0x241 => 0x134,
            0x242 if dagobah => 0x133,
            0x242 => 0x132,
            _ => 0,
        };
        Self::by_side(0x47c, alliance).with_voice(voice)
    }

    /// Future Jedi / Leia Uses the Force (`FUN_0048f1e0`): the trainer's
    /// voice; Vader's `0x136` is no resource, so he is silent.
    #[must_use]
    pub const fn future_jedi(alliance: bool, trainer: u32) -> Self {
        let voice = match trainer {
            0x241 => 0x124,
            0x242 => 0x135,
            0x281 => 0x136,
            _ => 0,
        };
        Self::by_side(0x47c, alliance).with_voice(voice)
    }

    /// Personnel Arrive (`FUN_004955b0`): the leader's base list.
    #[must_use]
    pub const fn personnel_arrive(alliance: bool, leader: u32, dagobah: bool) -> Self {
        Self::by_side(0x47c, alliance).with_voice(character_voice(leader, dagobah, 0))
    }

    /// A Mission Report (`FUN_004927c0`): `+0x30` by the mission's family
    /// (Subdue by its outcome), `+0x32` the leader's verdict, its base
    /// list plus 2 on success and 3 on failure (L1928..2014).
    #[must_use]
    pub const fn mission_report(
        mission: crate::message_pictures::ReportedMission,
        success: bool,
        alliance: bool,
        leader: u32,
        dagobah: bool,
    ) -> Self {
        use crate::message_pictures::ReportedMission;
        let strategy = match mission {
            ReportedMission::Espionage => Self::strategy(0x480),
            ReportedMission::Diplomacy => Self::strategy(0x47e),
            ReportedMission::Research => Self::strategy(0x47f),
            ReportedMission::SubdueUprising => Self::uprising(!success),
            ReportedMission::InciteUprising | ReportedMission::Other => {
                Self::by_side(0x47c, alliance)
            }
        };
        strategy.with_voice(character_voice(leader, dagobah, if success { 2 } else { 3 }))
    }

    /// The reader's own mission foiled (`FUN_00491500` codes 3..5):
    /// `0x480` and the leader's failure voice.
    #[must_use]
    pub const fn mission_foiled(leader: u32, dagobah: bool) -> Self {
        Self::strategy(0x480).with_voice(character_voice(leader, dagobah, 3))
    }

    /// The reader's own mission ended for its target or its system's side
    /// (`FUN_00491500` codes 6..10, `0xc`): `0x47c + (side != 1)` and the
    /// leader's failure voice.
    #[must_use]
    pub const fn mission_failed(alliance: bool, leader: u32, dagobah: bool) -> Self {
        Self::by_side(0x47c, alliance).with_voice(character_voice(leader, dagobah, 3))
    }

    /// Capital Ship Repaired (`FUN_00499de0`): `0x45e`; Squadron at Full
    /// Strength: `0x457 + (side != 1)`.
    #[must_use]
    pub const fn repaired(squadron: bool, alliance: bool) -> Self {
        if squadron {
            Self::by_side(0x457, alliance)
        } else {
            Self::strategy(0x45e)
        }
    }

    /// The other side's mission foiled (`FUN_00491500`): `0x47c + (side
    /// != 1)`.
    #[must_use]
    pub const fn enemy_mission_foiled(alliance: bool) -> Self {
        Self::by_side(0x47c, alliance)
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

    const fn sound(strategy: u16, voice: u16) -> MessageSounds {
        MessageSounds {
            strategy,
            sprite: 0,
            voice,
        }
    }

    const MOTHMA: u32 = 0x240;
    const LEIA: u32 = 0x241;
    const LUKE: u32 = 0x242;
    const HAN: u32 = 0x243;
    const EMPEROR: u32 = 0x280;
    const VADER: u32 = 0x281;

    #[test]
    fn the_economy_and_system_messages_play_their_strategy_waves() {
        // FUN_0048be60: an idle yard, 0x44d whatever the area.
        assert_eq!(MessageSounds::facility_idle(), sound(0x44d, 0));
        // FUN_0048e940: 0x44d with picture 0x3ea (resources), 0x44e for a
        // disaster (+100 bit 3).
        assert_eq!(MessageSounds::resources(), sound(0x44d, 0));
        assert_eq!(MessageSounds::disaster(), sound(0x44e, 0));
        // FUN_00499130: 0x44f only when smuggling begins (param_4 != 0:
        // 0x7060 / 0x7062).
        assert_eq!(MessageSounds::smuggling(true), sound(0x44f, 0));
        assert_eq!(MessageSounds::smuggling(false), MessageSounds::NONE);
        // FUN_00499760: 0x450 when a side gains the system, 0x451 when it
        // goes neutral (no gaining side).
        assert_eq!(MessageSounds::control_change(true), sound(0x450, 0));
        assert_eq!(MessageSounds::control_change(false), sound(0x451, 0));
        // FUN_00498cb0: 0x452 for an Alliance system, 0x453 for an
        // Empire one.
        assert_eq!(MessageSounds::near_uprising(true), sound(0x452, 0));
        assert_eq!(MessageSounds::near_uprising(false), sound(0x453, 0));
        // FUN_00499460: 0x454 when it begins, 0x455 when it ends.
        assert_eq!(MessageSounds::uprising(true), sound(0x454, 0));
        assert_eq!(MessageSounds::uprising(false), sound(0x455, 0));
        // FUN_0048b9a0, FUN_00498970, FUN_0048bbf0: units destroyed, 0x456.
        assert_eq!(MessageSounds::units_destroyed(), sound(0x456, 0));
        // FUN_004960f0: 0x45e. FUN_00491060: 0x44c.
        assert_eq!(MessageSounds::blockade(), sound(0x45e, 0));
        assert_eq!(MessageSounds::informants(), sound(0x44c, 0));
        // FUN_00495e90: 0x460 for reader 1, else 0x45f.
        assert_eq!(MessageSounds::evacuation_losses(true), sound(0x460, 0));
        assert_eq!(MessageSounds::evacuation_losses(false), sound(0x45f, 0));
        // FUN_00499aa0 and FUN_0048abc0: research, 0x459.
        assert_eq!(MessageSounds::research(), sound(0x459, 0));
    }

    #[test]
    fn units_deployed_and_arriving_play_by_kind_and_side() {
        // FUN_00497940: only case 1 (0x708a, a fighter squadron) writes
        // 0x457 + (side != 1).
        assert_eq!(MessageSounds::deployed(true, true), sound(0x457, 0));
        assert_eq!(MessageSounds::deployed(true, false), sound(0x458, 0));
        assert_eq!(MessageSounds::deployed(false, true), MessageSounds::NONE);
        // FUN_004981c0: cases 1 and 2 (0x7078, 0x7076) 0x45a, case 3
        // (0x707a) 0x45b + (side != 1), case 4 (0x707c) 0x45d.
        use crate::message_pictures::Arrival;
        assert_eq!(MessageSounds::arrival(Arrival::Fleet, false), sound(0x45a, 0));
        assert_eq!(MessageSounds::arrival(Arrival::Units, true), sound(0x45b, 0));
        assert_eq!(MessageSounds::arrival(Arrival::Units, false), sound(0x45c, 0));
        assert_eq!(
            MessageSounds::arrival(Arrival::Headquarters, true),
            sound(0x45d, 0)
        );
    }

    #[test]
    fn character_messages_speak_with_the_characters_voice() {
        // FUN_00490340 case 1 (0x715c, the captive's own side): 0x469 and
        // the escaper's list: Mon Mothma 0xf6, Leia 0xe2, Luke 0xce (0xd8
        // after Dagobah), Han 0xec, the Emperor 0x10a, Vader 0x100.
        assert_eq!(MessageSounds::escaped(true, MOTHMA, false), sound(0x469, 0xf6));
        assert_eq!(MessageSounds::escaped(true, LEIA, false), sound(0x469, 0xe2));
        assert_eq!(MessageSounds::escaped(true, LUKE, false), sound(0x469, 0xce));
        assert_eq!(MessageSounds::escaped(true, LUKE, true), sound(0x469, 0xd8));
        assert_eq!(MessageSounds::escaped(true, HAN, false), sound(0x469, 0xec));
        assert_eq!(MessageSounds::escaped(true, EMPEROR, false), sound(0x469, 0x10a));
        assert_eq!(MessageSounds::escaped(true, VADER, true), sound(0x469, 0x100));
        assert_eq!(MessageSounds::escaped(true, 0x2a0, false), sound(0x469, 0));
        // Case 2, the captors: no voice.
        assert_eq!(MessageSounds::escaped(false, MOTHMA, false), sound(0x469, 0));
        // Cases 3 and 4 keep the default 0x44c; case 5 (Jabba) 0x478.
        assert_eq!(MessageSounds::captured(), sound(0x44c, 0));
        assert_eq!(MessageSounds::jabba_captures(), sound(0x478, 0));
        // FUN_0048fdf0: 0x474 always; healed adds the list one past the
        // escape's (Luke 0xcf / 0xd9, Vader 0x101).
        use CharacterHealth::{Healed, Injured, Killed};
        assert_eq!(MessageSounds::character_health(Healed, LUKE, false), sound(0x474, 0xcf));
        assert_eq!(MessageSounds::character_health(Healed, LUKE, true), sound(0x474, 0xd9));
        assert_eq!(MessageSounds::character_health(Healed, VADER, false), sound(0x474, 0x101));
        assert_eq!(MessageSounds::character_health(Healed, MOTHMA, false), sound(0x474, 0xf7));
        assert_eq!(MessageSounds::character_health(Injured, LUKE, false), sound(0x474, 0));
        assert_eq!(MessageSounds::character_health(Killed, LUKE, false), sound(0x474, 0));
        // FUN_0048b050: assassins, 0x474 and no voice.
        assert_eq!(MessageSounds::assassins(), sound(0x474, 0));
    }

    #[test]
    fn the_story_messages_speak_their_scenes() {
        // FUN_0048df20: (Luke, Dagobah, Vader) picks 0x46a..0x471.
        assert_eq!(MessageSounds::confrontation(false, false, true), sound(0x46a, 0x137));
        assert_eq!(MessageSounds::confrontation(false, true, false), sound(0x46b, 0x138));
        assert_eq!(MessageSounds::confrontation(true, false, true), sound(0x46e, 0x118));
        assert_eq!(MessageSounds::confrontation(true, false, false), sound(0x470, 0x119));
        assert_eq!(MessageSounds::confrontation(true, true, true), sound(0x46f, 0x11a));
        assert_eq!(MessageSounds::confrontation(true, true, false), sound(0x471, 0x11b));
        // FUN_0048d190, FUN_0048dc50, FUN_0048ce80.
        assert_eq!(MessageSounds::luke_to_dagobah(), sound(0x46c, 0x11d));
        assert_eq!(MessageSounds::heritage(), sound(0x46d, 0x11e));
        assert_eq!(MessageSounds::emperor_arrives(), sound(0x47b, 0x130));
        // FUN_0048e3c0: by Luke's Dagobah bit.
        assert_eq!(MessageSounds::final_battle(false), sound(0x472, 0x11c));
        assert_eq!(MessageSounds::final_battle(true), sound(0x473, 0x12f));
        // FUN_0048b6e0: by Luke's Dagobah bit.
        assert_eq!(MessageSounds::jabba_delivers(false), sound(0x479, 0x123));
        assert_eq!(MessageSounds::jabba_delivers(true), sound(0x47a, 0x139));
        // FUN_0048fa00: Han captured, attacked, located.
        assert_eq!(MessageSounds::bounty(Bounty::Captured), sound(0x476, 0x11f));
        assert_eq!(MessageSounds::bounty(Bounty::Attacked), sound(0x475, 0x120));
        assert_eq!(MessageSounds::bounty(Bounty::Located), sound(0x477, 0));
    }

    #[test]
    fn the_side_story_messages_play_their_sides_wave_and_a_voice() {
        // 0x47c + (side != 1): FUN_0048d0f0 Luke leaves Dagobah (no voice).
        assert_eq!(MessageSounds::luke_leaves_dagobah(), sound(0x47c, 0));
        // FUN_0048c4c0: the rescuer, Leia 0xe5, Luke 0xd1 / 0xdb.
        assert_eq!(MessageSounds::rescue_attempt(true, LEIA, false), sound(0x47c, 0xe5));
        assert_eq!(MessageSounds::rescue_attempt(true, LUKE, true), sound(0x47c, 0xdb));
        assert_eq!(MessageSounds::rescue_attempt(true, LUKE, false), sound(0x47c, 0xd1));

        assert_eq!(MessageSounds::rescue_attempt(true, HAN, false), sound(0x47c, 0));
        // FUN_0048c9c0: the discoverer, Leia 0x127, Luke 0x125 / 0x126,
        // the Emperor 0x129, Vader 0x128.
        assert_eq!(MessageSounds::discovers_traitor(false, VADER, false), sound(0x47d, 0x128));
        assert_eq!(MessageSounds::discovers_traitor(false, EMPEROR, false), sound(0x47d, 0x129));
        assert_eq!(MessageSounds::discovers_traitor(true, LUKE, true), sound(0x47c, 0x126));
        assert_eq!(MessageSounds::discovers_traitor(true, LUKE, false), sound(0x47c, 0x125));
        assert_eq!(MessageSounds::discovers_traitor(true, LEIA, false), sound(0x47c, 0x127));
        assert_eq!(MessageSounds::discovers_traitor(true, MOTHMA, false), sound(0x47c, 0));
        // FUN_0048e1d0: the reader's own character, Luke 0xd0 / 0xda,
        // Leia 0xe4.
        assert_eq!(MessageSounds::detects_enemy(true, LUKE, false), sound(0x47c, 0xd0));
        assert_eq!(MessageSounds::detects_enemy(true, LUKE, true), sound(0x47c, 0xda));
        assert_eq!(MessageSounds::detects_enemy(true, LEIA, false), sound(0x47c, 0xe4));
        assert_eq!(MessageSounds::detects_enemy(false, VADER, false), sound(0x47d, 0));
        // FUN_0048ed80: Leia 0x134, Luke 0x132 / 0x133.
        assert_eq!(MessageSounds::force_growth(true, LUKE, false), sound(0x47c, 0x132));
        assert_eq!(MessageSounds::force_growth(true, LUKE, true), sound(0x47c, 0x133));
        assert_eq!(MessageSounds::force_growth(true, LEIA, false), sound(0x47c, 0x134));
        // FUN_0048f1e0: the trainer, Leia 0x124, Luke 0x135, Vader 0x136
        // (no such list: silent).
        assert_eq!(MessageSounds::future_jedi(true, LEIA), sound(0x47c, 0x124));
        assert_eq!(MessageSounds::future_jedi(true, LUKE), sound(0x47c, 0x135));
        assert_eq!(MessageSounds::future_jedi(false, VADER), sound(0x47d, 0x136));
        assert_eq!(MessageSounds::future_jedi(false, EMPEROR), sound(0x47d, 0));
        // FUN_004955b0: the leader's own list, Mon Mothma 0xf0, Leia 0xdc,
        // Luke 0xc8 / 0xd2, Han 0xe6, the Emperor 0x104, Vader 0xfa.
        assert_eq!(MessageSounds::personnel_arrive(true, MOTHMA, false), sound(0x47c, 0xf0));
        assert_eq!(MessageSounds::personnel_arrive(true, LUKE, false), sound(0x47c, 0xc8));
        assert_eq!(MessageSounds::personnel_arrive(true, LUKE, true), sound(0x47c, 0xd2));
        assert_eq!(MessageSounds::personnel_arrive(false, EMPEROR, false), sound(0x47d, 0x104));
        assert_eq!(MessageSounds::personnel_arrive(false, 0x2b0, false), sound(0x47d, 0));
    }

    #[test]
    fn a_mission_report_plays_its_familys_wave_and_the_leaders_verdict() {
        use crate::message_pictures::ReportedMission::{
            Diplomacy, Espionage, InciteUprising, Other, Research, SubdueUprising,
        };
        // FUN_004927c0: 0x52 0x480, 0x51 0x47e, 0x53 0x47f; Subdue 0x454
        // on outcome 2, 0x455 on 3; the rest 0x47c + (side != 1). +0x32:
        // the leader's success (outcome 3) or failure list.
        assert_eq!(MessageSounds::mission_report(Espionage, true, true, HAN, false), sound(0x480, 0xe8));
        assert_eq!(MessageSounds::mission_report(Diplomacy, false, true, LEIA, false), sound(0x47e, 0xdf));
        assert_eq!(MessageSounds::mission_report(Research, true, false, VADER, false), sound(0x47f, 0xfc));
        assert_eq!(MessageSounds::mission_report(SubdueUprising, false, true, MOTHMA, false), sound(0x454, 0xf3));
        assert_eq!(MessageSounds::mission_report(SubdueUprising, true, true, MOTHMA, false), sound(0x455, 0xf2));
        assert_eq!(MessageSounds::mission_report(InciteUprising, true, false, EMPEROR, false), sound(0x47d, 0x106));
        assert_eq!(MessageSounds::mission_report(Other, false, false, EMPEROR, false), sound(0x47d, 0x107));
        // Luke: 0xca / 0xcb before Dagobah, 0xd4 / 0xd5 after.
        assert_eq!(MessageSounds::mission_report(Other, true, true, LUKE, false), sound(0x47c, 0xca));
        assert_eq!(MessageSounds::mission_report(Other, false, true, LUKE, false), sound(0x47c, 0xcb));
        assert_eq!(MessageSounds::mission_report(Other, true, true, LUKE, true), sound(0x47c, 0xd4));
        assert_eq!(MessageSounds::mission_report(Other, false, true, LUKE, true), sound(0x47c, 0xd5));
        assert_eq!(MessageSounds::mission_report(Other, true, true, 0x2a0, false), sound(0x47c, 0));
        // FUN_00491500: own side, codes 3..5: 0x480 and the failure list;
        // the other side: 0x47c + (side != 1) and no voice.
        assert_eq!(MessageSounds::mission_foiled(VADER, false), sound(0x480, 0xfd));
        assert_eq!(MessageSounds::mission_foiled(LUKE, true), sound(0x480, 0xd5));
        assert_eq!(MessageSounds::enemy_mission_foiled(false), sound(0x47d, 0));
        // The other own-side ends (codes 6..10, 0xc) keep 0x47c + (side !=
        // 1) with the same failure voice (L217..256, L274).
        assert_eq!(MessageSounds::mission_failed(false, VADER, false), sound(0x47d, 0xfd));
        assert_eq!(MessageSounds::mission_failed(true, LUKE, false), sound(0x47c, 0xcb));
        assert_eq!(MessageSounds::mission_failed(true, 0x2a0, false), sound(0x47c, 0));
    }

    #[test]
    fn a_repaired_unit_plays_its_kinds_wave() {
        // FUN_00499de0: a squadron 0x457 + (side != 1), a ship 0x45e.
        assert_eq!(MessageSounds::repaired(true, true), sound(0x457, 0));
        assert_eq!(MessageSounds::repaired(true, false), sound(0x458, 0));
        assert_eq!(MessageSounds::repaired(false, false), sound(0x45e, 0));
    }

    #[test]
    fn a_voice_list_gives_its_wave_or_a_drawn_one() {
        // FUN_004c4990: count 1 needs no draw; 0xe8 holds four; 0x136 is
        // no resource.
        assert_eq!(voice_wave(0x11d, |_| unreachable!()), 1395);
        assert_eq!(voice_wave(0xe8, |count| count - 1), 1357);
        assert_eq!(voice_wave(0xe8, |_| 0), 1354);
        assert_eq!(voice_wave(0x136, |_| 0), 0);
        // A draw past the list takes its last entry.
        assert_eq!(voice_wave(0xe8, |count| count), 1357);
        assert_eq!(sound(0x469, 0xce).drawn(|_| 1).sprite, 1316);
        assert_eq!(MessageSounds::NONE.drawn(|_| unreachable!()), MessageSounds::NONE);
    }

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
            voice: 0,
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
            voice: 0,
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
