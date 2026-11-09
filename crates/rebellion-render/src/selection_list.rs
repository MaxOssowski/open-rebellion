//! The drop-down of a `CoolSelectionBoxClass` (`FUN_0060bed0`): the window
//! `FUN_0060cac0` opens under the box, `item_height * visible + 4` high and
//! as wide as the box, holding a `CoolDragList` at (2, 2) (`FUN_0060d020`)
//! with the scroll bar `FUN_0060f640` makes from RCDATA `0x299a` at its
//! right. The mission dialog's kinds (113-pixel items, one visible) and
//! Build Selection's classes (70-pixel items, two visible) use it.
//!
//! A0 (`kinds/23`, `kinds/31`, `manufacturing/24`): the list scrolls by
//! pixels while its down arrow is held; the wheel does nothing; its
//! background is STRATEGY 10598 tiled from the window's top-left inside a
//! white one-pixel frame `w - 1` by `h - 1`.

use egui_macroquad::egui;

use crate::bmp_cache::BmpCache;
use crate::list_scroll_bar::Hold;

/// The window's background tile (A0 `kinds/31`).
pub(crate) const BACKGROUND: u32 = 10_598;
const TILE: (f32, f32) = (195.0, 61.0);
/// The list's arrow step, as the Status window's list of the same bar
/// takes it (hyp: `FUN_0060a490`'s step argument is not read).
pub(crate) const STEP: f32 = 10.0;

/// A drop-down's box: its window-pixel left, the box's bottom, width, item
/// height and visible items (`FUN_0060bed0`'s arguments, `+0xa4`/`+0xa8`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct DropDown {
    pub x: f32,
    pub top: f32,
    pub width: f32,
    pub item_height: f32,
    pub visible: usize,
}

impl DropDown {
    /// The window's height, `item_height * visible + 4`.
    pub(crate) fn height(self) -> f32 {
        self.item_height * self.visible as f32 + 4.0
    }

    /// The list: (2, 2) in, `width - 4` by `item_height * visible`.
    pub(crate) fn list(self) -> (f32, f32, f32, f32) {
        (
            self.x + 2.0,
            self.top + 2.0,
            self.width - 4.0,
            self.item_height * self.visible as f32,
        )
    }

    /// The scroll position a newly opened list takes: from its top, only
    /// as far as item `index` needs to be seen, as the last visible item
    /// (A0 build-selection4: Build Selection's list opened on items 1 and 2
    /// with either shown, and on 3 and 4 with the fourth shown).
    pub(crate) fn scroll_to(self, index: usize) -> f32 {
        self.item_height * index.saturating_sub(self.visible.saturating_sub(1)) as f32
    }
}

/// What a frame of the drop-down did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct Outcome {
    /// The item a click chose.
    pub chosen: Option<usize>,
    /// The drop-down closed (a click on the list or outside the bar).
    pub closed: bool,
}

/// Draw the drop-down of `count` items at `scroll` and take its input.
/// `paint_item(painter, index, rect)` paints item `index` in its
/// `width` by `item_height` rect at the list's left.
#[expect(
    clippy::too_many_arguments,
    reason = "The window, its art cache, the box's geometry and the list's state."
)]
pub(crate) fn draw(
    ui: &egui::Ui,
    painter: &egui::Painter,
    cache: &mut BmpCache,
    frame: egui::Rect,
    scale: f32,
    id: egui::Id,
    drop: DropDown,
    count: usize,
    scroll: &mut f32,
    hold: &mut Option<Hold>,
    mut paint_item: impl FnMut(&egui::Painter, &mut BmpCache, usize, egui::Rect),
) -> Outcome {
    let ctx = ui.ctx().clone();
    let at = |x: f32, y: f32, w: f32, h: f32| crate::system_window::logical_rect(frame, scale, x, y, w, h);
    let height = drop.height();
    let popup = at(drop.x, drop.top, drop.width, height);
    let list = drop.list();
    let list_rect = at(list.0, list.1, list.2, list.3);
    painter.rect_filled(popup, 0.0, egui::Color32::BLACK);
    let inside =
        painter.with_clip_rect(painter.clip_rect().intersect(at(drop.x, drop.top, drop.width - 1.0, height - 1.0)));
    let mut ty = 0.0;
    while ty < height {
        let mut tx = 0.0;
        while tx < drop.width {
            crate::mission_dialog::paint(
                &inside,
                &ctx,
                cache,
                BACKGROUND,
                at(drop.x + tx, drop.top + ty, TILE.0, TILE.1),
                scale,
            );
            tx += TILE.0;
        }
        ty += TILE.1;
    }
    inside.rect_stroke(
        at(drop.x + 0.5, drop.top + 0.5, drop.width - 2.0, height - 2.0),
        0.0,
        egui::Stroke::new(scale, egui::Color32::WHITE),
        egui::StrokeKind::Middle,
    );
    let content = drop.item_height * count as f32;
    let bar = crate::list_scroll_bar::ScrollBar::new(
        crate::status_window::BAR_ART,
        list.0,
        list.1,
        list.2,
        list.3,
        content,
        *scroll,
    );
    let (bx, by, bw, bh) = bar.rect();
    let bar_rect = at(bx, by, bw, bh);
    let response = ui.interact(popup, id, egui::Sense::click());
    let items = painter.with_clip_rect(painter.clip_rect().intersect(list_rect));
    for index in 0..count {
        let top = list.1 + drop.item_height * index as f32 - *scroll;
        if top + drop.item_height <= list.1 || top >= list.1 + list.3 {
            continue;
        }
        paint_item(&items, cache, index, at(list.0, top, drop.width, drop.item_height));
    }
    if bar.visible() {
        *scroll = crate::list_scroll_bar::ui_bar(
            ui,
            cache,
            frame,
            scale,
            crate::status_window::BAR_ART,
            list,
            content,
            *scroll,
            hold,
            STEP,
        );
    }
    let mut outcome = Outcome::default();
    // A click on an item chooses it (command 0xbba, FUN_0060c970) and
    // closes the list; one on the bar only scrolls.
    if crate::system_window::exact_clicked(&response, popup) {
        let point = response.interact_pointer_pos();
        let on_bar = bar.visible() && point.is_some_and(|p| bar_rect.contains(p));
        if !on_bar {
            if let Some(point) = point.filter(|p| list_rect.contains(*p)) {
                let row = ((point.y - list_rect.min.y) / scale + *scroll) / drop.item_height;
                outcome.chosen = Some(row.floor() as usize);
            }
            outcome.closed = true;
        }
    }
    outcome
}
