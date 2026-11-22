use super::*;

#[test]
fn an_original_dll_is_read_from_the_installation_before_the_gdata_directory() {
    let root = std::env::temp_dir().join(format!("open-rebellion-dll-path-{}", std::process::id()));
    let game_dir = root.join("install");
    let gdata_path = root.join("gdata");
    std::fs::create_dir_all(&game_dir).expect("create install dir");
    std::fs::create_dir_all(&gdata_path).expect("create gdata dir");

    // Staged beside the DAT files only: the GData copy is the source.
    std::fs::write(gdata_path.join("TACTICAL.DLL"), b"gdata").expect("write gdata dll");
    assert_eq!(
        original_dll_path(&game_dir, &gdata_path, "TACTICAL.DLL"),
        gdata_path.join("TACTICAL.DLL")
    );

    std::fs::write(game_dir.join("TACTICAL.DLL"), b"install").expect("write installed dll");
    assert_eq!(
        original_dll_path(&game_dir, &gdata_path, "TACTICAL.DLL"),
        game_dir.join("TACTICAL.DLL")
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_game_over_action_picks_the_ending_from_the_victory_word_empire_win_first() {
    // FUN_00425c90: 0x100000 -> state 0x11 + (side == 2), 0x1000000 ->
    // 0x10 for side 1 else 0x13, nothing else -> 0.
    assert_eq!(ending_from_conditions_word(0x10_0000, false), Some(108));
    assert_eq!(ending_from_conditions_word(0x10_0000, true), Some(106));
    assert_eq!(ending_from_conditions_word(0x100_0000, true), Some(105));
    assert_eq!(ending_from_conditions_word(0x100_0000, false), Some(107));
    assert_eq!(ending_from_conditions_word(0x110_0000, true), Some(106));
    assert_eq!(ending_from_conditions_word(0x1000_0001, false), None);
}

#[test]
fn the_games_end_plays_the_movie_of_its_winner_for_the_players_side() {
    // FUN_00425c90's states 0x10..0x13 play MDATA.105..108.
    assert_eq!(ending_cutscene(true, true), 105);
    assert_eq!(ending_cutscene(false, true), 106);
    assert_eq!(ending_cutscene(true, false), 107);
    assert_eq!(ending_cutscene(false, false), 108);
    // State 7: MDATA.003 for side 1, MDATA.004 otherwise.
    assert!(new_game_cutscene(true).ends_with("003.webm"));
    assert!(new_game_cutscene(false).ends_with("004.webm"));
}

#[test]
fn any_key_skips_a_movie_but_the_trench_runs_take_only_escape() {
    // FUN_005f7240 (Escape, left button) for every movie; FUN_00421790's
    // WM_KEYDOWN ignores the keys in ids 0x70/0x71 (MDATA.202/201).
    for kind in [
        CutsceneKind::Intro,
        CutsceneKind::IntroCinematic,
        CutsceneKind::NewGame,
        CutsceneKind::Credits,
        CutsceneKind::Ending(105),
        CutsceneKind::Story(104),
    ] {
        assert!(key_skips_cutscene(&kind, false), "{kind:?}");
        assert!(key_skips_cutscene(&kind, true), "{kind:?}");
    }
    let trench = CutsceneKind::TrenchRun(TacticalTrenchRunOutcome::Success);
    assert!(!key_skips_cutscene(&trench, false));
    assert!(key_skips_cutscene(&trench, true));
}

#[test]
fn a_finished_cutscene_restarts_the_score_of_the_mode_it_returns_to() {
    // The logos hand over to the opening cinematic and the side's movie
    // to the game, which start their own sound; the cinematic, the
    // credits and the ending return to the main menu's score.
    assert_eq!(music_context_after_cutscene(&CutsceneKind::Intro), None);
    assert_eq!(music_context_after_cutscene(&CutsceneKind::NewGame), None);
    for kind in [
        CutsceneKind::IntroCinematic,
        CutsceneKind::Credits,
        CutsceneKind::Ending(105),
    ] {
        assert_eq!(
            music_context_after_cutscene(&kind),
            Some(MusicContext::MainMenu)
        );
    }
    for outcome in [
        TacticalTrenchRunOutcome::Success,
        TacticalTrenchRunOutcome::Failure,
    ] {
        assert_eq!(
            music_context_after_cutscene(&CutsceneKind::TrenchRun(outcome)),
            None
        );
    }
    // FUN_00422ce0's 0x402 after a story movie: FUN_004173c0 and
    // FUN_00422cb0 → FUN_00417520 restart the galaxy view's music.
    assert_eq!(
        music_context_after_cutscene(&CutsceneKind::Story(101)),
        Some(MusicContext::GalaxyMap)
    );
}

#[test]
fn the_menu_score_waits_for_the_main_menu_after_the_intro_movies() {
    // FUN_004138a0's 0x405: the logos' end or skip (0x65) starts state 2,
    // the cinematic; only its end (0x66) starts state 3, whose menu
    // window starts the score at WM_CREATE (FUN_00405050: FUN_004173c0,
    // FUN_00417520). No movie plays under a score (FUN_00610c00(0)).
    for kind in [
        CutsceneKind::Intro,
        CutsceneKind::IntroCinematic,
        CutsceneKind::NewGame,
        CutsceneKind::Credits,
        CutsceneKind::Ending(105),
        CutsceneKind::Story(101),
        CutsceneKind::TrenchRun(TacticalTrenchRunOutcome::Success),
    ] {
        assert!(
            !score_may_start(&GameMode::Cutscene { kind: kind.clone() }),
            "{kind:?}"
        );
    }
    assert!(score_may_start(&GameMode::MainMenu));
    assert!(score_may_start(&GameMode::Galaxy));
}

#[test]
fn the_side_movie_leaves_the_score_off_until_the_galaxy_view_shows() {
    // The side's movie (state 7) stops the music; the game's galaxy
    // window starts it at its creation (FUN_00422ce0: FUN_004173c0,
    // FUN_00422cb0 -> FUN_00417520). The frames between, while the
    // campaign loads, play no score.
    assert!(!score_may_start(&GameMode::GameSetup));
}

#[test]
fn a_story_movie_over_the_battle_summary_returns_to_the_galaxy_score() {
    use rebellion_render::MusicTrack;
    // MDATA.308, the Alliance viewer's win (FUN_0046f360, 0x006a8478).
    let summary = MusicTrack::Original(12);
    let story = GameMode::Cutscene {
        kind: CutsceneKind::Story(102),
    };
    let mut playing = None;
    // The summary's first paint plays its track once (FUN_00471200 ->
    // FUN_00470e70 clears +0x188).
    assert_eq!(
        screen_score(&GameMode::Galaxy, Some(summary), &mut playing),
        ScreenScore::Play(summary)
    );
    // FUN_00422ce0's 0x482 stops the music for the movie; its 0x402
    // starts the galaxy view's again (FUN_00422cb0), and the summary,
    // already painted, does not replay its track.
    assert_eq!(
        screen_score(&story, Some(summary), &mut playing),
        ScreenScore::Keep
    );
    assert_eq!(
        screen_score(&GameMode::Galaxy, Some(summary), &mut playing),
        ScreenScore::Keep
    );
    // Its Close stops and restarts the galaxy view's (FUN_004727a0).
    assert_eq!(
        screen_score(&GameMode::Galaxy, None, &mut playing),
        ScreenScore::Restore
    );
    assert_eq!(playing, None);

    // A summary that opens on the movie's first frame has painted
    // before the movie too (0x482 pumps messages until the animations
    // end), so the movie's end does not start its track either.
    let mut playing = None;
    assert_eq!(
        screen_score(&story, Some(summary), &mut playing),
        ScreenScore::Keep
    );
    assert_eq!(
        screen_score(&GameMode::Galaxy, Some(summary), &mut playing),
        ScreenScore::Keep
    );
}

#[test]
fn the_battle_alert_closing_into_the_battle_leaves_no_score_to_restore() {
    use rebellion_render::MusicTrack;
    // MDATA.307, the Alliance viewer's Battle Alert (FUN_0044f860).
    let alert = MusicTrack::Original(10);
    let mut playing = None;
    assert_eq!(
        screen_score(&GameMode::TacticalCombat, Some(alert), &mut playing),
        ScreenScore::Play(alert)
    );
    // The battle has no score (FUN_00450540 -> FUN_00417640); nothing
    // gives the galaxy's back under it.
    assert_eq!(
        screen_score(&GameMode::TacticalCombat, None, &mut playing),
        ScreenScore::Keep
    );
    assert_eq!(playing, None);
}

#[test]
fn game_options_over_the_battle_summary_keep_its_score_until_the_menu() {
    use rebellion_render::MusicTrack;
    // MDATA.315, the Empire viewer's loss (FUN_0046f360, 0x006a8478).
    let summary = MusicTrack::Original(15);
    let options = GameMode::GameOptions {
        origin: GameOptionsOrigin::CommandCenter,
    };
    let mut playing = None;
    assert_eq!(
        screen_score(&GameMode::Galaxy, Some(summary), &mut playing),
        ScreenScore::Play(summary)
    );
    // The summary painted once (FUN_00470e70); Game Options and the
    // return to it do not start its track again.
    assert_eq!(
        screen_score(&options, None, &mut playing),
        ScreenScore::Keep
    );
    assert_eq!(
        screen_score(&GameMode::Galaxy, Some(summary), &mut playing),
        ScreenScore::Keep
    );
    // Restart leaves for the main menu, whose window starts its own
    // score (FUN_00405050 WM_CREATE: FUN_004173c0, FUN_00417520).
    assert_eq!(
        screen_score(&options, None, &mut playing),
        ScreenScore::Keep
    );
    assert_eq!(
        screen_score(&GameMode::MainMenu, None, &mut playing),
        ScreenScore::Restore
    );
}
