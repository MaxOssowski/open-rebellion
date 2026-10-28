# The advisor's clip records and voice: format settled, staged (P34)

Date: 2026-10-28. Tracker item "Droidy-doradcy", stage 1 of the briefing's
playback. The reading is in `ghidra/notes/advisor-briefing.md`, "The clip
classes and the definition record".

## The original

- A queued record's word 0 is an op. `FUN_00403ed0` clones the prototype
  registered for it (`FUN_00403eb0`, slot `+0x2c`).
  - Op 4 is the clip-definition class `0x00658520`.
  - Op `0x11` is the frame-list class `0x00658848` (the idle clips).
  - The note lists the other five ops.
- Op 4's loader `FUN_00404440` reads the definition record as 16-bit
  words:
  - word 0: a sound;
  - words 1 and 2: a WAVE and its DLL slot;
  - word 3: the frame count;
  - word 4 on: the first frame.
- The first frame is a standard bitmap; the rest are the type-302 deltas
  `first + 1 ..`.
- `briefing-definitions.txt` checks every slot-`0xd` definition in both
  briefing DLLs:
  - each one the briefing plays has its BMP, all its deltas and its WAVE;
  - the two it does not play (ALBRIEF `0x103c` and `0x1042`) lack their
    WAVE and some deltas.

## The port

- `tools/stage-ui-assets` now stages each advisor DLL's RCDATA records and
  WAVE clips, unchanged:
  - records go to `RCDATA/{id}.bin`;
  - clips go to `WAVE/{id}.wav`, checked by the tool's RIFF validator.
- It also stages the two briefing DLLs into `albrief-dll/` and
  `embrief-dll/`.
  - These targets are optional: they are skipped when absent.
  - They are also read under their GOG names `ALBRIEF.DLL.OLD` and
    `EMBRIEF.DLL.OLD`.
- `verify` checks every count.
- `scripts/build-runtime-pack.py` keeps the briefing directories out of
  `runtime.orpk` until the runtime reads them (`UNPACKED_UI_DIRS`). So the
  browser pack is unchanged.
- Nothing plays yet. The player, the queue from message `0x29` / `0x4a`,
  and the start are the next stages.

## Run (`staging.txt`)

The run used `go run ./tools/stage-ui-assets --source "<GOG install>"`.
`source-dlls.sha256` holds the source DLLs' hashes.

| DLL | Result |
|---|---|
| ALSPRITE | 752 RCDATA, 213 WAVE |
| EMSPRITE | 753 RCDATA, 216 WAVE |
| ALBRIEF | 20 BMP, 2684 type-302, 366 RCDATA, 17 WAVE |
| EMBRIEF | 18 BMP, 2738 type-302, 471 RCDATA, 22 WAVE |

- Every resource staged before is unchanged (6314 unchanged, 8270
  written).
- Every target verifies.
- The staged files are original data and are not committed.

## Tests

Go (`go test ./tools/stage-ui-assets/...`, `go vet`, `gofmt` clean):
- `TestStageTargetsStagesClipRecordsFromAnAliasedDLL`: a record staged
  from `TEST.DLL.OLD`, byte for byte, then verified;
- `TestStageTargetsStagesVoiceClipsAsWavFiles`: two WAVE clips staged and
  verified, and a wrong count refused;
- `TestOptionalTargetsAbsentFromTheSourceAreSkipped`: an absent optional
  target is skipped by stage and verify; a required one fails.

Before the change, these tests do not compile, since the target fields
are new.

Python (`scripts/test_build_runtime_pack.py`, 10 tests):
- `test_briefing_dlls_stay_out_of_the_pack_until_the_runtime_reads_them`;
- it fails against the previous packer (1 failure).

No Rust changed, so `cargo mutants` does not apply. The Go code has no
mutation tool in this repository.
