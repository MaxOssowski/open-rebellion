# Informants and Maintenance Shortfall read their texts from TEXTSTRA

Date: 2026-10-21 (overnight run, "Ekonomia i incydenty").

## The original

- Informants Provide Information (`FUN_00491060`, notification `0x153`)
  takes TEXTSTRA `0x7150` and `0x7151` "Informants have provided us with
  information about {1:1}.".
- Maintenance Shortfall (`FUN_00498970`, notification `0x304`) speaks to
  the destroyed unit's side.
  - The title is `0x7080`.
  - The body is `0x7081` "The following units have been destroyed at
    {3:1} due to a maintenance shortfall:". Its third substitution is the
    system, found up the unit's containers (family `0x90`..`0x97`).
  - Each unit adds a line `0x7082` "{1:1}".

## What the port did before

The app held both messages as English strings in its code, with the
title joined to the body.

## Port

- `story_messages::informants_texts` and `maintenance_texts` fill the
  title and body from the player's TEXTSTRA.
- The app files them as before, with the title kept separate.
- port: one autoscrap is one message with one unit line.

## Tests

- New: `informants_and_maintenance_shortfall_take_their_texts`.
- `cargo mutants --in-diff`: 10 mutants, 10 caught.

## Gates (`f25`)

- The 24 dual-AI campaigns and the seed-42 golden are unchanged.
- The replay equivalence check passes with 0 browser errors (release WASM
  SHA-256
  `7d8bd0131009192b937ed0e65a3d137dd6e3a7199ab311552c5756fcc516b4fc`).
- The Fleet window suite passes 54 cases.
- No frame took over 50 ms; the console has 152 `[ai]` lines and 0
  errors.
- Not checked: the messages in the browser.

Hashes of the committed files are in `SHA256SUMS`.
