//! The victory word settled into game-decided messages and ending movies,
//! moved out of `main.rs`. The code reads `main.rs`'s items through
//! `super`.

use super::*;

/// The victory conditions word's check and the game's end.
///
/// The word (`FUN_005180e0`): each change is `0x128`, Alliance
/// Headquarters Destroyed (`FUN_0048e5a0`) with bit `0x1` and Coruscant
/// Captured (`FUN_0048f790`) while the Alliance holds Coruscant. Once it
/// ends the game, its change reaches the advisor's slot `0x10`
/// (`FUN_004c3010` / `FUN_004c0f10`), which schedules its win or loss
/// speech; the speech's last clip carries the game-over action (kind 3),
/// and only that sets the ending's state (`FUN_004c3060` -> `FUN_0041d830`
/// -> `FUN_00425c90`). So the ending waits for the advisor's next idle end
/// and the speech, while the word's game-over bit holds the clock
/// ([`galaxy_clock_runs`], `FUN_0051de80`). port: without the advisor's
/// messages the end follows at once.
#[expect(
    clippy::too_many_arguments,
    reason = "the word's change reaches the movies, the log, the advisor and the ending"
)]
pub(super) fn settle_victory_word(
    victory_state: &mut VictoryState,
    world: &GameWorld,
    victory_conditions: VictoryConditions,
    message_texts: &std::collections::HashMap<u16, String>,
    msg_log: &mut MessageLog,
    advisor_schedule: Option<&mut rebellion_render::advisor_schedule::AdvisorSchedule>,
    advisor_now: u32,
    pending_ending: &mut Option<u32>,
    player_is_alliance: bool,
    tick: u64,
) {
    if let Some((_, new)) =
        rebellion_core::victory::advance_conditions_word(victory_state, world, victory_conditions)
    {
        victory_word_movies(msg_log, new, message_texts, tick);
    }
    let Some(outcome) = VictorySystem::outcome(victory_state, world, victory_conditions) else {
        return;
    };
    victory_state.resolved = true;
    game_decided_messages(
        &outcome,
        victory_state,
        msg_log,
        advisor_schedule,
        advisor_now,
        pending_ending,
        player_is_alliance,
        tick,
    );
}

/// The victory conditions word's change to `new` (`0x128`): its story
/// movies, Alliance Headquarters Destroyed and Coruscant Captured.
pub(super) fn victory_word_movies(
    msg_log: &mut MessageLog,
    new: u32,
    message_texts: &std::collections::HashMap<u16, String>,
    tick: u64,
) {
    macroquad::logging::info!("[victory] word={:#x} day={}", new, tick);
    for movie in rebellion_render::story_messages::victory_conditions_movies(new, message_texts) {
        msg_log.queue_story_movie(movie);
    }
}

/// The decided game's line, then the advisor's win or loss speech or the
/// ending ([`settle_victory_word`]).
#[expect(
    clippy::too_many_arguments,
    reason = "the end reaches the log, the advisor and the ending"
)]
pub(super) fn game_decided_messages(
    outcome: &rebellion_core::victory::VictoryOutcome,
    victory_state: &VictoryState,
    msg_log: &mut MessageLog,
    advisor_schedule: Option<&mut rebellion_render::advisor_schedule::AdvisorSchedule>,
    advisor_now: u32,
    pending_ending: &mut Option<u32>,
    player_is_alliance: bool,
    tick: u64,
) {
    let msg = match outcome {
        rebellion_core::victory::VictoryOutcome::HqCaptured { winner, loser, .. } => {
            format!("{winner:?} captured {loser:?} headquarters! {winner:?} wins!")
        }
        rebellion_core::victory::VictoryOutcome::HqDestroyed { .. } => {
            "The Empire destroyed the Alliance headquarters and secured its system. Empire wins!"
                .to_string()
        }
        rebellion_core::victory::VictoryOutcome::DeathStarVictory { .. } => {
            "The Death Star destroyed the Alliance headquarters system. Empire wins!".to_string()
        }
    };
    msg_log.push(GameMessage::new(tick, msg, MessageCategory::Event));
    if let Some(schedule) = advisor_schedule {
        schedule.victory_word(victory_state.conditions_word, advisor_now);
    } else {
        // FUN_00425c90: the end's movie for the winner and the player's
        // side, then the main menu. FUN_00422ce0 starts it once the story
        // movies the same day queued have played (A0 empire-clean/61, 62:
        // MDATA.103, then the defeat).
        *pending_ending = Some(ending_cutscene(
            victory_winner_is_alliance(outcome),
            player_is_alliance,
        ));
    }
}

/// The word settled between day steps, before the clock moves.
///
/// The original stores the word when the event that changes it is
/// handled (`Event300_GameObjExistingChanged_Handle` = `FUN_005140c0` ->
/// `FUN_00517cd0` -> `FUN_005180e0`, in `FUN_0051df30`'s pass), posts its
/// `0x128` in the same pass (`FUN_0051e050` -> `FUN_00518460`), and reads
/// the game-over bit before every sub-tick (`FUN_0041e390` ->
/// `FUN_0051de80`). port: the app's orders run at once, between day
/// steps; the day's own changes settle after its step.
#[expect(
    clippy::too_many_arguments,
    reason = "the word's change reaches the movies, the log, the advisor and the ending"
)]
pub(super) fn settle_victory_word_between_days(
    victory_state: &mut VictoryState,
    world: &GameWorld,
    victory_conditions: VictoryConditions,
    message_texts: &std::collections::HashMap<u16, String>,
    msg_log: &mut MessageLog,
    advisor_schedule: Option<&mut rebellion_render::advisor_schedule::AdvisorSchedule>,
    advisor_now: u32,
    pending_ending: &mut Option<u32>,
    player_is_alliance: bool,
    tick: u64,
) {
    if rebellion_core::victory::word_change_pending(victory_state, world, victory_conditions) {
        settle_victory_word(
            victory_state,
            world,
            victory_conditions,
            message_texts,
            msg_log,
            advisor_schedule,
            advisor_now,
            pending_ending,
            player_is_alliance,
            tick,
        );
    }
}

/// The ending a game left behind, dropped as another game starts or loads.
///
/// The ending's state lives in the galaxy window (`+0xa8`, set by
/// `FUN_00425c90`, posted by `FUN_0042db70`), and app state 3
/// (`FUN_00414a80`) destroys that window, so nothing of a game's ending
/// reaches the next. port: the ending waits in `pending_ending` behind the
/// story movies queued ahead, which do not play while the game is paused.
pub(super) fn drop_left_ending(pending_ending: &mut Option<u32>) {
    *pending_ending = None;
}

pub(super) fn victory_winner_is_alliance(outcome: &rebellion_core::victory::VictoryOutcome) -> bool {
    match outcome {
        rebellion_core::victory::VictoryOutcome::HqCaptured { winner, .. }
        | rebellion_core::victory::VictoryOutcome::HqDestroyed { winner, .. } => {
            *winner == Faction::Alliance
        }
        rebellion_core::victory::VictoryOutcome::DeathStarVictory { .. } => false,
    }
}
