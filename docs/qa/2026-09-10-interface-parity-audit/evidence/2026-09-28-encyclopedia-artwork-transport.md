---
title: "P62 Encyclopedia Artwork Transport"
description: "Validated native and browser transport for original EDATA artwork without enabling the replacement encyclopedia as an accepted interface"
category: evidence
created: 2026-09-28
updated: 2026-09-28
tags: [encyclopedia, edata, wasm, bitmap, interface]
status: verified
---

# P62 Encyclopedia Artwork Transport

## Scope

This checkpoint removes the hard-coded WASM `None` path for encyclopedia
artwork. Original `EDATA.NNN` files are validated before packaging, stored in a
dedicated runtime-pack namespace, separated from simulation data at install,
decoded lazily with nearest-neighbor sampling, and cached by their original
three-digit identity. Native loading also resolves the common original-install
layout where `GData/` and `EData/` are siblings, plus explicit
`REBELLION_EDATA_DIR` and `REBELLION_GAME_DIR` overrides.

No original image bytes or generated runtime pack are committed.

## Evidence contract

- Source identity: the filename remains `EDATA.NNN`; the packer rejects an
  invalid name, non-BMP payload, wrong dimensions, non-indexed pixels,
  compression, or truncation.
- Browser transport: artwork uses `encyclopedia/assets/` entries inside the
  existing ORPK v3 request and is removed before the DAT cache is installed.
- Rendering: only requested images are decoded into GPU textures; original
  artwork uses nearest sampling; a missing or invalid asset logs its exact
  identity once and shows the existing fail-closed placeholder.
- Product boundary: command `0x131` remains fail-closed. This transport does
  not promote the current four-tab egui approximation or any `OBJ-01` cell to
  original-interface acceptance.

## Verification

The isolated implementation checkpoint passes:

- six runtime-pack builder tests, including valid, malformed-dimension, and
  malformed-name and palette-deficient EDATA cases;
- seven focused renderer tests;
- four runtime-pack parser/partition tests;
- one native original-install path test;
- an interface-fixture WASM application compile;
- the complete interface ledger and production-fixture exclusion checks;
- validation and deterministic packaging of all 187 owned 400x200x8 source
  images. The final local test pack was 65,379,478 bytes with SHA-256
  `edc7e94e217b9f976d7f309afda9764cc957d48d7229ae9c9eeda1963d68cd11`;
- one fresh Chrome for Testing 151.0.7922.34 process at 640x480 and DPR 1,
  launched with `--mute-audio`, exactly four HTTP 200 startup requests, no
  browser errors, and closed cleanup;
- exact display of `EDATA.042` at 400x200. All 80,000 source pixels matched,
  with source SHA-256
  `f31aa70c1171f1bb296177632f45d678d14bc3c69070f6f6b33272ef8af99378`,
  screenshot SHA-256
  `5e20132567a768d5e87536b8f605bd076a26c950694b1247ff9165b12e9025b5`,
  and fixture WASM SHA-256
  `8daa902a6ed93a03ad60a3d78f1204c65a0009ec0de7cf6d39ceeee7709c48af`.

The ignored browser result, full screenshot, source decode, and zero-pixel diff
remain local because they contain or reproduce owned original artwork. Primary
local inspection found the displayed image stable, unblank, unclipped, and free
of obvious corruption. An independent browser reviewer inspected the authorized
full-resolution capture and result record, found no P0/P1 visual defects, and
confirmed the muted four-request, zero-error, exact-pixel, closed-cleanup
evidence. This accepts transport and display only, not the authentic
encyclopedia window.

## Remaining original-interface work

`OBJ-01` remains open for the recovered index and topic shells, category and
topic ordering, original description text, exact system/entity bindings,
previous/next navigation, contextual entry, missing-entry behavior, and the
complete faction/viewport evidence matrix. This checkpoint fixes asset
availability only.
