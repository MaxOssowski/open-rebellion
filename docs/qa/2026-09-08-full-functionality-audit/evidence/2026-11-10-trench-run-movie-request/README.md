# The trench run's movie request, counted when Escape aborts it

Date: 2026-11-10. Gate tooling only. No game code and no ledger cell
changed.

## The flake

The tactical matrix's `production-trench-run-success` and `-failure`
cases (`run.mjs`) failed in several batches (b155, b156, b159, b164,
b166). In each, the request list missed `/data/video/201.webm` or
`202.webm`. Run alone, they always passed.

## The cause

The trench run plays its movie before the battle returns. The gate
presses Escape as soon as a `<video>` element exists. Escape stops the
movie and aborts its download (`net::ERR_ABORTED`), which the gate
already ignored as an error.

The gate built its request list from `response` events. On a busy host,
Escape could abort the download before its response arrived. The
browser had asked for the movie, but no response event came, so the
request was missing from the list.

## The change

On the trench run's aborted movie request, the gate now adds the request
to the list with no status, marked `aborted`. The list check is
unchanged and still needs the movie's path. The status check skips only
these aborted entries; every request that got a response must still be
200.

## Runs

- Each case run alone after the change: 4 of 4 for both.
- Batch b167: 23 of 23 interface gates on Linux; workspace tests 3498
  passed, fixture tests 139. Tactical matrix
  `2026-11-10T05-13-44-374Z-3914083`: 148 of 148.
