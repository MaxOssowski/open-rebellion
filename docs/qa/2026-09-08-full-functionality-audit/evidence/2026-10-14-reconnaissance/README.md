# Reconnaissance (F-040)

Date: 2026-10-14. `agent-browser` 0.38.1, headless Chromium 153, 640 by 480;
driver `mission-flow.py 62 alliance,empire`.

Fixture `ReconTargeting` (code 63) puts the side's reconnaissance special
force (SPECFCSD class 3, Longprobe Y-wing Recon Team, mask `0x4`; class 5,
Imperial Probe Droid, mask `0x100`) alone on the primary system's personnel
page and opens the enemy system's sector window, as `MissionTargeting` does
for an agent.

| Side | Mission list | Target | After Begin |
|---|---|---|---|
| Alliance | Reconnaissance alone | F'tral (the planet) | Reconnaissance active |
| Empire | Reconnaissance alone | the planet | Reconnaissance active |

4 of 4 requests HTTP 200, 0 console errors on each side; screenshots
inspected (`*/2-menu.png` shows Mission enabled for the special force,
`*/4-dialog.png` the Recon icon and target). An agent (character) is not
offered Reconnaissance: the record's character mask is 0, so in the
`MissionTargeting` scenario the list stays Espionage and Incite Uprising.

Hashes are in `SHA256SUMS`.
