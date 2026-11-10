# A new Empire game's first galaxy view against A0, and the Death Star display

Date: 2026-11-10. Covers:

- CMD-01-C002 (Imperial initial);
- CMD-01-C003 (all apertures populated), the Empire side;
- CMD-02-C025 (Death Star intelligence), excluded.

Docs only. No code changed.

## A0

Windows 10 VM, original REBEXE. A new Empire game (Standard) was
started from the shuttle cockpit. It opens on day 2 with the Message
Index over the galaxy view
(`00-new-empire-game-day2-message-index`). One Esc closed the index,
the pointer was moved to the corner, and the galaxy view was captured
on Popular Support (`01-new-empire-game-day2-popular-support`). The
captures are in `.artifacts/a0-vm/empire-initial/`, with their SHA-256
in `SHA256SUMS`.

## The comparison

The port's frame is the GID gate's Empire Popular Support capture at 640
by 480 from batch b168's run:
`.artifacts/interface-parity/2026-11-10T05-58-13-529Z-4030835/gid/empire/popular-support/640x480/actual.png`
(SHA-256 `e678ae340803c229d1c4b6ac86140a2c99f6d5184d0ba2c3b9fba1a44e4573b6`).
The two were compared pixel for pixel:

| Region (canvas) | Identical pixels |
| --- | --- |
| whole canvas | 304,574 of 307,200 (99.15%) |
| galaxy aperture (55..539, 40..389) | 168,255 of 169,750 (99.12%) |
| top bar (y 0..39) | 25,255 of 25,600 (98.65%) |
| cockpit bottom (y 395..479) | 53,704 of 54,400 (98.72%) |
| cockpit left (x 0..54, y 40..389) | 19,227 of 19,250 (99.88%) |
| cockpit right (x 540..639, y 40..389) | 35,000 of 35,000 (100%) |

Every differing pixel is state that differs between the two games, or
animation:

- the GID marks of the systems near the Core, whose sides and support
  differ between games;
- the three resource readouts and the day number;
- the caption's antialiasing;
- the battery indicator by the day;
- the Imperial advisor's idle frame at the bottom left.

The shell, the starfield, the apertures, the rails and the controls are
pixel-identical. This is the Alliance comparison of
`2026-10-28-galaxy-composition-a0` (CMD-01-C001), now for the Empire.

## CMD-02-C025: no Death Star intelligence state

The manual (p. 73) says the Alliance sees Death Star shields on the
Defenses display only if informants or espionage found them. The
executable has no such state:

- `FUN_004511e0` builds the GID menu. Its only Death Star entry is
  display `0x73`, Death Star Shields (`ghidra/notes/gid-markers.md`).
- `FUN_0042b330` case `0x73` calls `FUN_0051c0d0(system, holder, 1)`.
  That walks the system's objects of families `0x25..0x26`
  (`FUN_004f6c00` → `FUN_00513090`).
- The walk's filter, `FUN_005131d0`, keeps an object when:
  - its family is in the range;
  - `FUN_004f6b90` mode 1 holds, that is `+0x50` bit 0, the object is
    completed;
  - its side (`+0x24` bits 6..7) is the holder's.

  There is no intelligence or fog test.

No display or mark shows the Death Star itself. Under the owner's
decision of 2026-11-10 (states absent from the original are removed),
the cell is excluded.
