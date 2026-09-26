//! Dependency-free bridge between the browser accessibility tree and the
//! authentic bitmap main menu.

use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

use rebellion_render::{MainMenuControl, MainMenuState};

const NO_FOCUS_UPDATE: i32 = -2;
const NO_ACTIVATION: i32 = -1;

static PENDING_FOCUS: AtomicI32 = AtomicI32::new(NO_FOCUS_UPDATE);
static PENDING_ACTIVATION: AtomicI32 = AtomicI32::new(NO_ACTIVATION);
static USER_INTERACTION: AtomicBool = AtomicBool::new(false);

extern "C" {
    fn open_rebellion_quit();
    fn open_rebellion_a11y_sync(
        active: u32,
        focused: i32,
        difficulty: u32,
        galaxy_size: u32,
        headquarters_only: u32,
        music_enabled: u32,
    );
}

#[no_mangle]
pub extern "C" fn open_rebellion_a11y_crate_version() -> u32 {
    1
}

#[no_mangle]
pub extern "C" fn open_rebellion_menu_focus(index: i32) {
    if index == -1 || MainMenuControl::from_index(index as u32).is_some() {
        PENDING_FOCUS.store(index, Ordering::Release);
        if index >= 0 {
            USER_INTERACTION.store(true, Ordering::Release);
        }
    }
}

#[no_mangle]
pub extern "C" fn open_rebellion_menu_activate(index: u32) {
    if MainMenuControl::from_index(index).is_some() {
        PENDING_ACTIVATION.store(index as i32, Ordering::Release);
        USER_INTERACTION.store(true, Ordering::Release);
    }
}

pub fn take_focus_update() -> Option<Option<MainMenuControl>> {
    match PENDING_FOCUS.swap(NO_FOCUS_UPDATE, Ordering::AcqRel) {
        NO_FOCUS_UPDATE => None,
        -1 => Some(None),
        index => Some(MainMenuControl::from_index(index as u32)),
    }
}

pub fn take_activation() -> Option<MainMenuControl> {
    let index = PENDING_ACTIVATION.swap(NO_ACTIVATION, Ordering::AcqRel);
    (index >= 0)
        .then(|| MainMenuControl::from_index(index as u32))
        .flatten()
}

pub fn take_user_interaction() -> bool {
    USER_INTERACTION.swap(false, Ordering::AcqRel)
}

pub fn sync_menu(active: bool, state: &MainMenuState, music_enabled: bool) {
    let difficulty = match state.difficulty {
        rebellion_render::Difficulty::Easy => MainMenuControl::Easy,
        rebellion_render::Difficulty::Medium => MainMenuControl::Intermediate,
        rebellion_render::Difficulty::Hard => MainMenuControl::Expert,
    };
    let galaxy_size = match state.galaxy_size {
        rebellion_core::dat::GalaxySize::Standard => MainMenuControl::SmallGalaxy,
        rebellion_core::dat::GalaxySize::Large => MainMenuControl::MediumGalaxy,
        rebellion_core::dat::GalaxySize::Huge => MainMenuControl::LargeGalaxy,
    };
    unsafe {
        open_rebellion_a11y_sync(
            active as u32,
            state
                .semantic_focus()
                .map(|control| control.index() as i32)
                .unwrap_or(-1),
            difficulty.index() as u32,
            galaxy_size.index() as u32,
            state.headquarters_only as u32,
            music_enabled as u32,
        );
    }
}

/// Replace the stopped canvas with an accessible restart action.
pub fn show_quit() {
    unsafe {
        open_rebellion_quit();
    }
}
