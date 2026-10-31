# Dialog text against A0 (interface text)

Date: 2026-10-31. Follows `../2026-10-31-message-index-text-a0/`.

## Change

Both dialogs now go through `theme::paint_gdi_text` and
`theme::paint_gdi_text_block`:
- **Pause alert** ("Resume Game Play?"). Font 5, `DT_CENTER |
  DT_VCENTER` in (43, 25)–(368, 112), drawn by `Class_00658950_vt3c`.
- **Game Options questions.** Also `Class_00658950_vt3c`:
  - the first string in font 5, centred and word-broken (`DT_WORDBREAK`)
    from the top of (43, 25)–(368, 86);
  - the second string centred on one line (`DT_VCENTER`) in its band.

## Check

| Gate (Linux Chromium) | A0 capture | Result |
|---|---|---|
| `personnel-orders.mjs --a0=.artifacts/a0-vm/tooltip` | `tooltip/321-resume-box-base` | the message's ink box is [254, 215, 384, 227] in both, identical |
| `game-options.mjs --a0=.artifacts/a0-vm/tooltip` | `tooltip/211-dlg-yes-base` (the Restart question) | passes its centre, top and bottom checks |
| `main-menu-states.mjs --a0` | — | passes |

The old pause text was centred by egui's galley; the GDI version is set
by the line's GDI width and font 5's `tmHeight`.

The unit test `the_pause_alert_message_is_centred_in_its_rectangle` now
reads the line's pen and baseline.
