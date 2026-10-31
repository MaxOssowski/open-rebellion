# The pause alert against A0 (CMD-05-C001)

Date: 2026-10-31.

A0 `tooltip/321-resume-box-base.png` (hash in `a0-captures.sha256`) shows
the "Resume Game Play?" alert in an Empire game. REBDLOG 10622 is at
(114, 152) and the checkmark 10625 at (290, 286). In the galaxy view the
alert's DIB colours show unmapped (97.4 per cent of 10622's pixels match it
directly; the rest are its text and the pointer). This differs from Game
Options, whose palette remaps them.

The port drew the message in egui's proportional face, centred by
half-pixels. It now draws it as `Class_00658950_vt3c` does: font 5
(`theme::original_font`), colour 0x2f0fbff, centred horizontally, with its
line's top (87 - 16) / 2 = 35 whole pixels into the (43, 25)-(368, 112)
rectangle. The unit test `the_pause_alert_message_is_centred_in_its_rectangle`
now asserts that top and that font.

`personnel-orders.mjs --a0=DIR` (Empire abort case, Linux Chromium) pauses
the game and compares the alert with A0 (`pause-alert-result.json`):
- **Dialog.** No pixel differs outside the message's rectangle and A0's
  pointer.
- **Message ink.** A0 254, 215 to 384, 227; the port 249, 215 to 389, 227.
  The top and bottom are the same and the centre is the same; the port's
  line is 10 pixels wider, which is the accepted text deviation.
