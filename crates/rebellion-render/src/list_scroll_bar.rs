//! The original list's vertical scroll bar: the "Scroll Bar" window
//! (`FUN_0060a490` -> `FUN_0060aa70`, vtable `0x66e2e8`) that a
//! `CoolDragList` (`FUN_00607ea0`) holds in its scroll host (`+0x94`).
//!
//! Its art is a STRATEGY.DLL RCDATA record that the list's owner names,
//! read by `FUN_0060af70`: the width, the track tile, the up and down
//! arrows (normal and pressed), and the thumb's top cap, middle and bottom
//! cap (`FUN_0060f910`). Each piece, 13 wide, is centred on the bar
//! (`width / 2 - 13 / 2`: the arrows `FUN_0060fa80`, the track
//! `FUN_0060fb80`, the thumb `FUN_0060fca0`), so a bar narrower than 13
//! clips it.
//!
//! The range is in pixels (`+0xdc` total, `+0xe0` first, `+0xe4` end): an
//! arrow scrolls by the list's step (`+0xc0`), a click on the track by the
//! list's height, and a drag follows the pointer. This module is pure
//! geometry and input; the caller draws [`ScrollBar::pieces`].

/// A bar's RCDATA record (`FUN_0060af70`): `[0]` the width, `[1]` the
/// track tile (13 by 13), `[2]`/`[3]` the up arrow and `[4]`/`[5]` the down
/// arrow, normal and pressed (13 by 9), `[6]`..`[8]` the thumb's top cap
/// (13 by 6), middle (13 by 12, tiled) and bottom cap. `[9]` is not read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BarArt {
    pub width: u16,
    pub track: u32,
    pub up: [u32; 2],
    pub down: [u32; 2],
    pub top: u32,
    pub middle: u32,
    pub bottom: u32,
}

/// RCDATA 10653, the Battle Results' forces list's bar (`FUN_0046f360`).
pub const RESULTS_ART: BarArt = BarArt {
    width: 12,
    track: 10_656,
    up: [10_685, 10_686],
    down: [10_687, 10_688],
    top: 10_689,
    middle: 10_690,
    bottom: 10_691,
};

/// RCDATA 10748 (`0x29fc`), the Battle Alert's list's bar (`FUN_0044f860`).
pub const ALERT_ART: BarArt = BarArt {
    width: 10,
    track: 10_749,
    up: [10_750, 10_751],
    down: [10_752, 10_753],
    top: 10_754,
    middle: 10_755,
    bottom: 10_756,
};

/// Every piece's width.
const PIECE_WIDTH: u16 = 13;

const ARROW_HEIGHT: f32 = 9.0;
const TRACK_TILE_HEIGHT: f32 = 13.0;
const THUMB_CAP_HEIGHT: f32 = 6.0;
const THUMB_MIDDLE_HEIGHT: f32 = 12.0;
/// `FUN_0060fca0`: `if (iVar1 < 0xd) iVar1 = 0xc`, which on whole
/// pixels keeps the thumb at least 12 high.
const MIN_THUMB: f32 = 12.0;
/// `FUN_00609ae0`: content that fills the list gains 5 pixels.
const CONTENT_MARGIN: f32 = 5.0;

/// An arrow held this long starts repeating (the strobe's threshold
/// `0xfa / 50` ticks, `FUN_00602150`'s `+0x104`), seconds.
const ARROW_REPEAT_DELAY: f64 = 0.25;
/// The arrow strobe's period, seconds: the arrows (ids 100 and 101, made at
/// `0x60b179` and `0x60b24e`) pass 0x32 to `FUN_00602150` (`+0xfc`), whose
/// strobe `FUN_0060f1e0` starts a periodic `timeSetEvent` of that delay
/// (`FUN_00601f40(delay, TIME_PERIODIC, 10)`).
const ARROW_REPEAT_PERIOD: f64 = 0.05;
/// A held track click pages again on this timer (`SetTimer(.., 0x96)`).
const PAGE_REPEAT_PERIOD: f64 = 0.15;

/// The most a list of `content` pixels in a `view`-high list scrolls
/// (`FUN_00609ae0`: `content - view`, at least 0, content that fills the
/// list taking 5 more).
#[must_use]
pub fn max_first(content: f32, view: f32) -> f32 {
    let content = if content >= view {
        content + CONTENT_MARGIN
    } else {
        content
    };
    (content - view).max(0.0)
}

/// A bar beside a list, in the caller's coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollBar {
    /// Its art.
    pub art: BarArt,
    /// The bar's left edge: the list's right edge less the bar's width
    /// (`FUN_0060fa80`: `MoveWindow(bar, list.w - width, 0, width, list.h)`).
    pub x: f32,
    /// The list's top.
    pub y: f32,
    /// The list's height, the visible part of the range.
    pub view: f32,
    /// The most `first` can be ([`max_first`]).
    pub max: f32,
    /// The first visible pixel, within `0..=max`.
    first: f32,
}

/// A part of the bar under the pointer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    Up,
    Down,
    PageUp,
    PageDown,
    Thumb,
}

/// One bitmap of the bar: its STRATEGY.DLL id and top left.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Piece {
    pub id: u32,
    pub x: f32,
    pub y: f32,
}

impl ScrollBar {
    /// The bar of a list at `(left, top)`, `width` by `view`, holding
    /// `content` pixels scrolled to `first`, drawn with `art`.
    #[must_use]
    pub fn new(
        art: BarArt,
        left: f32,
        top: f32,
        width: f32,
        view: f32,
        content: f32,
        first: f32,
    ) -> Self {
        let max = max_first(content, view);
        Self {
            art,
            x: left + width - f32::from(art.width),
            y: top,
            view,
            max,
            first: first.clamp(0.0, max),
        }
    }

    /// `FUN_0060b5d0`: the bar shows unless the whole range is in view.
    #[must_use]
    pub fn visible(&self) -> bool {
        self.max > 0.0
    }

    fn total(&self) -> f32 {
        self.max + self.view
    }

    /// The track's length, between the arrows.
    fn track(&self) -> f32 {
        (self.view - 2.0 * ARROW_HEIGHT).max(0.0)
    }

    /// The thumb's top and height (`FUN_0060fca0`): the visible share of
    /// the track, at least 12, placed by `first` over the rest.
    #[must_use]
    pub fn thumb(&self) -> (f32, f32) {
        let track = self.track();
        let top = self.y + ARROW_HEIGHT;
        if self.max <= 0.0 {
            return (top, track);
        }
        let height = (self.view * track / self.total())
            .floor()
            .max(MIN_THUMB)
            .min(track);
        // `first` stays within `0..=max` ([`ScrollBar::new`]), so the
        // offset stays within the track.
        let offset = ((track - height) * self.first / self.max).floor();
        (top + offset, height)
    }

    /// The part under `(x, y)`, if any.
    #[must_use]
    pub fn part_at(&self, (x, y): (f32, f32)) -> Option<Part> {
        if !self.visible() || !(self.x..self.x + f32::from(self.art.width)).contains(&x) {
            return None;
        }
        let bottom = self.y + self.view;
        if !(self.y..bottom).contains(&y) {
            return None;
        }
        if y < self.y + ARROW_HEIGHT {
            return Some(Part::Up);
        }
        if y >= bottom - ARROW_HEIGHT {
            return Some(Part::Down);
        }
        let (top, height) = self.thumb();
        Some(if y < top {
            Part::PageUp
        } else if y >= top + height {
            Part::PageDown
        } else {
            Part::Thumb
        })
    }

    /// The bar's bitmaps, back to front, with the arrow `pressed` drawn
    /// pressed. The 13-wide art sits at `width / 2 - 13 / 2` and the caller
    /// clips it to the bar; the track and the thumb's middle tile past their
    /// ends, which the clip takes too.
    #[must_use]
    pub fn pieces(&self, pressed: Option<Part>) -> Vec<Piece> {
        let mut pieces = Vec::new();
        if !self.visible() {
            return pieces;
        }
        let art = self.art;
        let x = self.x
            + f32::from(i16::try_from(art.width / 2).unwrap_or(0)
                - i16::try_from(PIECE_WIDTH / 2).unwrap_or(0));
        let track_top = self.y + ARROW_HEIGHT;
        let track_bottom = self.y + self.view - ARROW_HEIGHT;
        let mut y = track_top;
        while y < track_bottom {
            pieces.push(Piece { id: art.track, x, y });
            y += TRACK_TILE_HEIGHT;
        }
        let (top, height) = self.thumb();
        // The thumb is clipped to its height (A0 2026-10-29: nothing of it
        // shows below the bottom cap). The middle tiles are uniform down
        // their height, so the last one moves up to end at the thumb's
        // bottom instead.
        let mut y = top;
        while y < top + height {
            let tile = y.min(top + height - THUMB_MIDDLE_HEIGHT).max(top);
            pieces.push(Piece {
                id: art.middle,
                x,
                y: tile,
            });
            y += THUMB_MIDDLE_HEIGHT;
        }
        pieces.push(Piece { id: art.top, x, y: top });
        pieces.push(Piece {
            id: art.bottom,
            x,
            y: top + height - THUMB_CAP_HEIGHT,
        });
        let state = |part| usize::from(pressed == Some(part));
        pieces.push(Piece {
            id: art.up[state(Part::Up)],
            x,
            y: self.y,
        });
        pieces.push(Piece {
            id: art.down[state(Part::Down)],
            x,
            y: track_bottom,
        });
        pieces
    }

    /// The bar's clip rectangle: x, y, width, height.
    #[must_use]
    pub fn rect(&self) -> (f32, f32, f32, f32) {
        (self.x, self.y, f32::from(self.art.width), self.view)
    }

    /// `first` for the thumb's top at `top` (`FUN_0060fd50`'s drag:
    /// `(y - grab - 9) * (total - vis) / (L - t)`), clamped.
    fn first_for_thumb(&self, top: f32) -> f32 {
        let (_, height) = self.thumb();
        let travel = self.track() - height;
        if travel <= 0.0 {
            return self.first;
        }
        ((top - self.y - ARROW_HEIGHT) * self.max / travel)
            .floor()
            .clamp(0.0, self.max)
    }
}

/// A press held on the bar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hold {
    pub part: Part,
    /// When the press began, seconds.
    pub since: f64,
    /// When it last stepped, seconds.
    pub last: f64,
    /// For the thumb: the pointer's distance below the thumb's top.
    pub grab: f32,
}

/// The pointer's state this frame, in the bar's coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pointer {
    pub at: (f32, f32),
    /// The left button went down this frame.
    pub pressed: bool,
    /// The left button is down.
    pub down: bool,
    /// The left button came up this frame.
    pub released: bool,
}

/// Advance the bar one frame: `step` is the list's arrow step (`+0xc0`).
/// Returns the new first pixel; `hold` keeps the press between frames.
///
/// - An arrow steps once when it is released over it (the strobe button's
///   `WM_COMMAND`), and held past the strobe's threshold steps on each
///   strobe tick (`FUN_0060f1e0`).
/// - A press on the track pages by the list's height towards the pointer
///   at once and every 150 ms while it stays on that side of the thumb
///   (`FUN_0060fd50`, `FUN_0060fff0`).
/// - A press on the thumb drags it (`SetTimer(1000, 0x32)`; port: every
///   frame).
pub fn update(bar: &ScrollBar, hold: &mut Option<Hold>, pointer: Pointer, step: f32, now: f64) -> f32 {
    let mut first = bar.first;
    if pointer.pressed {
        *hold = bar.part_at(pointer.at).map(|part| Hold {
            part,
            since: now,
            last: now,
            grab: pointer.at.1 - bar.thumb().0,
        });
        if let Some(Hold {
            part: part @ (Part::PageUp | Part::PageDown),
            ..
        }) = *hold
        {
            first = page(bar, part, first);
        }
        // A press and its release can land in one frame (a quick click).
        if !pointer.released {
            return first;
        }
    }
    let Some(mut held) = *hold else {
        return first;
    };
    let over = bar.part_at(pointer.at);
    if pointer.released || !pointer.down {
        if pointer.released && matches!(held.part, Part::Up | Part::Down) && over == Some(held.part) {
            first = arrow(bar, held.part, first, step);
        }
        *hold = None;
        return first;
    }
    match held.part {
        Part::Up | Part::Down => {
            if over == Some(held.part)
                && now - held.since >= ARROW_REPEAT_DELAY
                && now - held.last >= ARROW_REPEAT_PERIOD
            {
                first = arrow(bar, held.part, first, step);
                held.last = now;
            }
        }
        Part::PageUp | Part::PageDown => {
            if over == Some(held.part) && now - held.last >= PAGE_REPEAT_PERIOD {
                first = page(bar, held.part, first);
                held.last = now;
            }
        }
        Part::Thumb => first = bar.first_for_thumb(pointer.at.1 - held.grab),
    }
    *hold = Some(held);
    first
}

fn arrow(bar: &ScrollBar, part: Part, first: f32, step: f32) -> f32 {
    let delta = if part == Part::Up { -step } else { step };
    (first + delta).clamp(0.0, bar.max)
}

fn page(bar: &ScrollBar, part: Part, first: f32) -> f32 {
    let delta = if part == Part::PageUp {
        -bar.view
    } else {
        bar.view
    };
    (first + delta).clamp(0.0, bar.max)
}

/// One frame of a window's list bar, in an egui window whose top-left is
/// `frame` at `scale`: the pointer's press, hold and release on the bar
/// ([`update`]), then its pieces, the held arrow drawn pressed while the
/// pointer stays on it. `list` is the list's `(x, y, width, height)` in
/// window pixels, `content` its rows' height; returns the new first pixel.
#[expect(
    clippy::too_many_arguments,
    reason = "The window, its art cache, the list and the bar's state."
)]
pub fn ui_bar(
    ui: &egui_macroquad::egui::Ui,
    cache: &mut crate::bmp_cache::BmpCache,
    frame: egui_macroquad::egui::Rect,
    scale: f32,
    art: BarArt,
    list: (f32, f32, f32, f32),
    content: f32,
    first: f32,
    hold: &mut Option<Hold>,
    step: f32,
) -> f32 {
    use egui_macroquad::egui;
    let (pointer, pressed, down, released, now) = ui.input(|input| {
        (
            input.pointer.interact_pos(),
            input.pointer.button_pressed(egui::PointerButton::Primary),
            input.pointer.button_down(egui::PointerButton::Primary),
            input.pointer.button_released(egui::PointerButton::Primary),
            input.time,
        )
    });
    let at = pointer.map_or((-1.0, -1.0), |point| {
        ((point.x - frame.min.x) / scale, (point.y - frame.min.y) / scale)
    });
    let bar_at = |first| ScrollBar::new(art, list.0, list.1, list.2, list.3, content, first);
    let first = update(
        &bar_at(first),
        hold,
        Pointer {
            at,
            pressed,
            down,
            released,
        },
        step,
        now,
    );
    let bar = bar_at(first);
    let held = hold.map(|hold| hold.part).filter(|part| {
        matches!(part, Part::Up | Part::Down) && down && bar.part_at(at) == Some(*part)
    });
    let (x, y, width, height) = bar.rect();
    let painter = ui
        .painter()
        .with_clip_rect(crate::system_window::logical_rect(frame, scale, x, y, width, height));
    for piece in bar.pieces(held) {
        crate::fleet_window::paint_native(
            &painter,
            ui.ctx(),
            cache,
            crate::bmp_cache::DllSource::Strategy,
            piece.id,
            frame,
            scale,
            piece.x,
            piece.y,
        );
    }
    first
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The battle results' forces list (`FUN_0046f360`): (37, 136), 347
    /// by 166, three 70-high rows (content 210).
    fn results_bar(first: f32) -> ScrollBar {
        ScrollBar::new(RESULTS_ART, 37.0, 136.0, 347.0, 166.0, 210.0, first)
    }

    fn press(at: (f32, f32)) -> Pointer {
        Pointer {
            at,
            pressed: true,
            down: true,
            released: false,
        }
    }

    fn held(at: (f32, f32)) -> Pointer {
        Pointer {
            at,
            pressed: false,
            down: true,
            released: false,
        }
    }

    fn release(at: (f32, f32)) -> Pointer {
        Pointer {
            at,
            pressed: false,
            down: false,
            released: true,
        }
    }

    #[test]
    fn the_bar_sits_at_the_lists_right_edge_and_hides_when_everything_fits() {
        // FUN_0060fa80: x = list.w - 12; FUN_0060b5d0 hides the bar when
        // first is 0 and end is total. FUN_00609ae0: content that fills the
        // list gains 5 pixels, so three rows scroll 210 + 5 - 166 = 49.
        let bar = results_bar(0.0);
        assert_eq!((bar.x, bar.y, bar.max), (372.0, 136.0, 49.0));
        assert!(bar.visible());
        let two_rows = ScrollBar::new(RESULTS_ART, 37.0, 136.0, 347.0, 166.0, 140.0, 0.0);
        assert!(!two_rows.visible());
        assert!(two_rows.pieces(None).is_empty());
        assert_eq!(two_rows.part_at((375.0, 140.0)), None);
        assert_eq!(max_first(166.0, 166.0), 5.0);
    }

    #[test]
    fn the_thumb_is_the_visible_share_of_the_track_at_least_twelve() {
        // FUN_0060fca0: track 166 - 18 = 148; height 166 * 148 / 215 =
        // 114; its top 9 + (148 - 114) * first / 49 below the bar's top.
        assert_eq!(results_bar(0.0).thumb(), (145.0, 114.0));
        assert_eq!(results_bar(24.0).thumb(), (161.0, 114.0));
        assert_eq!(results_bar(49.0).thumb(), (179.0, 114.0));
        // The thumb's top row is the thumb's, the row above the track's.
        let bar = results_bar(0.0);
        assert_eq!(bar.part_at((375.0, 144.0)), Some(Part::Up));
        assert_eq!(bar.part_at((375.0, 145.0)), Some(Part::Thumb));
        assert_eq!(bar.part_at((375.0, 258.0)), Some(Part::Thumb));
        assert_eq!(bar.part_at((375.0, 259.0)), Some(Part::PageDown));
        assert_eq!(bar.part_at((375.0, 293.0)), Some(Part::Down));
        assert_eq!(bar.part_at((375.0, 301.0)), Some(Part::Down));
        assert_eq!(bar.part_at((375.0, 302.0)), None);
        assert_eq!(bar.part_at((383.0, 200.0)), Some(Part::Thumb));
        assert_eq!(bar.part_at((384.0, 200.0)), None);
        assert_eq!(bar.part_at((371.0, 200.0)), None);
        assert_eq!(results_bar(24.0).part_at((375.0, 160.0)), Some(Part::PageUp));
        // A long list's thumb keeps 12 (`if (iVar1 < 0xd) iVar1 = 0xc`).
        let long = ScrollBar::new(RESULTS_ART, 0.0, 0.0, 100.0, 166.0, 7000.0, 0.0);
        assert_eq!(long.thumb().1, 12.0);
    }

    #[test]
    fn the_bars_art_is_rcdata_10653_from_the_track_up() {
        // FUN_0060af70 reads RCDATA 10653; FUN_0060f910 builds the thumb
        // from its middle tile with the caps at 0 and h - 6.
        let pieces = results_bar(0.0).pieces(Some(Part::Down));
        let ids: Vec<u32> = pieces.iter().map(|piece| piece.id).collect();
        // The track tiles from 145 to 293 (12 tiles of 13), the thumb's
        // middle from 145 to 259 (10 tiles of 12), the caps, the arrows.
        assert_eq!(ids.iter().filter(|&&id| id == RESULTS_ART.track).count(), 12);
        assert_eq!(ids.iter().filter(|&&id| id == RESULTS_ART.middle).count(), 10);
        // A track of 11 whole tiles (161 - 18 = 143) and a thumb of 10
        // whole tiles (161 * 143 / 191 = 120) take no tile past their ends.
        let exact = ScrollBar::new(RESULTS_ART, 0.0, 0.0, 100.0, 161.0, 186.0, 0.0);
        assert_eq!(exact.thumb(), (9.0, 120.0));
        let ids: Vec<u32> = exact.pieces(None).iter().map(|piece| piece.id).collect();
        assert_eq!(ids.iter().filter(|&&id| id == RESULTS_ART.track).count(), 11);
        assert_eq!(ids.iter().filter(|&&id| id == RESULTS_ART.middle).count(), 10);
        assert_eq!(
            &pieces[pieces.len() - 4..],
            [
                Piece { id: RESULTS_ART.top, x: 372.0, y: 145.0 },
                Piece { id: RESULTS_ART.bottom, x: 372.0, y: 253.0 },
                Piece { id: RESULTS_ART.up[0], x: 372.0, y: 136.0 },
                Piece { id: RESULTS_ART.down[1], x: 372.0, y: 293.0 },
            ]
        );
    }

    #[test]
    fn an_arrow_steps_on_release_and_repeats_while_held() {
        // The strobe button posts WM_COMMAND on release; held, its strobe
        // (FUN_0060f1e0(50, ..), threshold 0xfa / 50) repeats it. The step
        // is the list's +0xc0, 10 for the battle results.
        let bar = results_bar(0.0);
        let down = (375.0, 296.0);
        let mut hold = None;
        assert_eq!(update(&bar, &mut hold, press(down), 10.0, 0.0), 0.0);
        assert_eq!(update(&bar, &mut hold, held(down), 10.0, 0.1), 0.0);
        assert_eq!(update(&bar, &mut hold, release(down), 10.0, 0.15), 10.0);
        assert_eq!(hold, None);
        // A press and release in one frame steps once too.
        let click = Pointer {
            released: true,
            ..press(down)
        };
        assert_eq!(update(&bar, &mut hold, click, 10.0, 0.5), 10.0);
        assert_eq!(hold, None);
        // A button found up without its release ends the hold, no step.
        update(&bar, &mut hold, press(down), 10.0, 0.6);
        let up = Pointer {
            down: false,
            ..held(down)
        };
        assert_eq!(update(&bar, &mut hold, up, 10.0, 0.7), 0.0);
        assert_eq!(hold, None);
        // Released elsewhere: no step.
        update(&bar, &mut hold, press(down), 10.0, 1.0);
        assert_eq!(update(&bar, &mut hold, release((0.0, 0.0)), 10.0, 1.1), 0.0);
        // Held past the threshold it steps each tick, not before.
        update(&bar, &mut hold, press(down), 10.0, 2.0);
        assert_eq!(update(&bar, &mut hold, held(down), 10.0, 2.1), 0.0);
        assert_eq!(update(&bar, &mut hold, held(down), 10.0, 2.25), 10.0);
        let bar = results_bar(10.0);
        assert_eq!(update(&bar, &mut hold, held(down), 10.0, 2.27), 10.0);
        assert_eq!(update(&bar, &mut hold, held(down), 10.0, 2.31), 20.0);
        // The up arrow stops at 0, the down one at the most.
        let up = (375.0, 138.0);
        let top = results_bar(5.0);
        update(&top, &mut hold, press(up), 10.0, 3.0);
        assert_eq!(update(&top, &mut hold, release(up), 10.0, 3.1), 0.0);
        let end = results_bar(45.0);
        update(&end, &mut hold, press(down), 10.0, 4.0);
        assert_eq!(update(&end, &mut hold, release(down), 10.0, 4.1), 49.0);
    }

    #[test]
    fn a_track_press_pages_by_the_lists_height_and_repeats() {
        // FUN_0060fd50: below the thumb pages down by the list's height,
        // clamped; held, again every 150 ms (SetTimer 0x96) while the
        // pointer stays below the thumb.
        let tall = |first| ScrollBar::new(RESULTS_ART, 37.0, 136.0, 347.0, 166.0, 700.0, first);
        let mut hold = None;
        let below = (375.0, 290.0);
        assert_eq!(update(&tall(0.0), &mut hold, press(below), 10.0, 1.0), 166.0);
        assert_eq!(update(&tall(166.0), &mut hold, held(below), 10.0, 1.1), 166.0);
        assert_eq!(update(&tall(166.0), &mut hold, held(below), 10.0, 1.2), 332.0);
        assert_eq!(update(&tall(539.0), &mut hold, release(below), 10.0, 1.3), 539.0);
        let above = (375.0, 150.0);
        assert_eq!(update(&tall(100.0), &mut hold, press(above), 10.0, 1.0), 0.0);
    }

    #[test]
    fn a_dragged_thumb_follows_the_pointer() {
        // FUN_0060fd50's drag: first = (y - grab - 9) * (total - vis) /
        // (L - t), clamped to the range.
        let bar = results_bar(0.0);
        let mut hold = None;
        update(&bar, &mut hold, press((375.0, 150.0)), 10.0, 0.0);
        assert_eq!(hold.unwrap().part, Part::Thumb);
        // 17 pixels down of the 34 the thumb travels: half the range.
        assert_eq!(update(&bar, &mut hold, held((375.0, 167.0)), 10.0, 0.1), 24.0);
        assert_eq!(update(&bar, &mut hold, held((375.0, 400.0)), 10.0, 0.2), 49.0);
        assert_eq!(update(&bar, &mut hold, held((375.0, 0.0)), 10.0, 0.3), 0.0);
    }

    #[test]
    fn the_thumbs_middle_never_reaches_past_its_bottom() {
        // A0 2026-10-29 (the Status window's bar, RCDATA 10650): a
        // 176-pixel thumb ends at its bottom cap.
        for content in [210.0, 300.0, 700.0, 7000.0] {
            let bar = ScrollBar::new(RESULTS_ART, 0.0, 0.0, 100.0, 204.0, content, 0.0);
            let (top, height) = bar.thumb();
            let middles: Vec<f32> = bar
                .pieces(None)
                .iter()
                .filter(|piece| piece.id == RESULTS_ART.middle)
                .map(|piece| piece.y)
                .collect();
            for &y in &middles {
                assert!(y >= top && y + THUMB_MIDDLE_HEIGHT <= top + height, "{y} {height}");
            }
            // They cover the thumb from its top to its bottom, without gaps.
            assert_eq!(middles.first(), Some(&top));
            assert_eq!(middles.last().map(|y| y + THUMB_MIDDLE_HEIGHT), Some(top + height));
            for pair in middles.windows(2) {
                assert!(pair[1] - pair[0] <= THUMB_MIDDLE_HEIGHT, "{pair:?}");
            }
        }
    }
}
