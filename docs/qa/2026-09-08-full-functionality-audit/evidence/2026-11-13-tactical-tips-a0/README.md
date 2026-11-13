# The tactical battle's tips against the Windows VM (2026-11-13)

This adds to CMD-11-C030 (tactical battle panels' tips). It follows
`2026-11-12-tactical-panels`, which checked one tip (Missions) against a Wine
capture. The ledger JSON is not changed.

## A0 used

`.artifacts/a0-vm/2026-11-13-tactical-panel-tips/` in the main checkout
(git-ignored). The original REBEXE in the offline Windows 10 QEMU VM, the
unsaved Alliance game, day 377: Battle at Averam, Take Command, paused
throughout. Corellian Corvette 1 (Task Force 1) is selected for 00..41, Blue
Group (Y-wings) for 43..49, and the battle's options panel is open for
50..54. Each file name ends with the walked point. Every box in A0 stands at
that point plus (0, 21), or ends at x 639, or sits above the point near the
bottom, so the walked point is the VM's pointer. The arrow drawn a few pixels
off is the cursor sprite, not the hot spot.

`SHA256SUMS` (sha256 `89941cf8650c7784c8c01300876470913a2b6ce140e9d4b1509b9c36344a3480`)
holds all 57 files. `sha256sum -c` passed on 2026-11-13.

| File | SHA-256 |
| --- | --- |
| `00-corvette-left-arrow-493-62.png` | `f37baafdc88b2179347eea74162d1e4c8b791fbf10dd81c39dbef62284d9e749` |
| `01-corvette-right-arrow-621-62.png` | `267d584ce36546fab381ec2547f125f1f2ee6edd07b09558a7f9b93e22f8ce3c` |
| `02-corvette-ship-picture-556-55.png` | `b7092a4096d9005db162c1834525e2b100c6479329d5c1a8e7dae55c4e734c65` |
| `03-corvette-shield-button-502-112.png` | `03885ef7664c2d23a8c4d98b735c247149c2f576359ab7449937ce084173295f` |
| `04-corvette-shield-bar-527-112.png` | `b2da678c31239b9f809dd64b070b5cd6a03a36052b65e27397cb0d35de0505a3` |
| `05-corvette-weapon-button-571-112.png` | `2c1df4c1bc22c8efdc5ba24fce11954a99dd1b2274a9ace43da4ff1d4ef49412` |
| `06-corvette-weapon-bar-600-112.png` | `9eed3770132d1d6ec0a95fb081464020554b75ce456bdf4ca73ec5daab1f4635` |
| `07-corvette-icon1-503-138.png` | `5899a53546247e2154c1e7005355d86fa26fa5c4aed9af5ed87a7d239f2f9c8e` |
| `08-corvette-icon2-530-138.png` | `627e95aa61d9f98b978aaa0e8e162f8d07311550f9a88263d93f602a76ed7754` |
| `09-corvette-icon3-557-138.png` | `02e9a8946777656029787fa89735a51beb65c7a9eb955dba3278d0e0be456997` |
| `10-corvette-icon4-584-138.png` | `cea937e2eb23e31289c8791eecdaa83fbe43106c92f969e8f70489450b744227` |
| `11-corvette-icon5-611-138.png` | `30b78e054fbf8bf12c6fa2a6e40b7afb71c23421bd83f809daa0e7c4a4bbae7d` |
| `12-corvette-fighters-area-556-172.png` | `8366eb026ea6f627007115bf4c7cbfb57918ae56508e2d7dd9b06801296e8d4b` |
| `13-corvette-fighters-bar-556-195.png` | `99d95fa1e5debcbc0291b3a5581ef033ec931439776e6c0f759f59db39717c41` |
| `14-corvette-orders-text-556-220.png` | `f6ef7a6d2ed409a7ce1dab302b396b88f4c55700ad4c8c4d9f2641c10ad218b0` |
| `15-corvette-order-button-522-248.png` | `358294803fd38c6592513f2f5b0b44e1eb5de0602042e2ebfc1a61bb74b5bd1f` |
| `16-corvette-missions-button-588-248.png` | `ec5275ec387a7945afd23a5ceb3dc52c0a3f7658a5ce5ab145f72f6a5f0d9476` |
| `17-hud-cube1-497-284.png` | `aa1f738af55ecc20c0814cfcb6c7ea0beb707df44cf25a9618f0e6f74b13d672` |
| `18-hud-cube2-536-284.png` | `452d89d97f44192f9f95bdc7f7543168a4a398c66801412e19745f8efacaf937` |
| `19-hud-cube3-575-284.png` | `e5c48acbe50fa4477ebe1a1520ea27c44e7d66dd321cea3faf81bcc19c09347f` |
| `20-hud-cube4-614-284.png` | `da6603101f362e60f64eb3b73238bb517f6da8eceb1bf85812d3c575ed6e7478` |
| `21-hud-empire-emblem-497-318.png` | `9694fff2c0d97ef70f92d69c52b478f7156e82dd90b34c108cad4baf1a0b9046` |
| `22-hud-alliance-emblem-530-318.png` | `d9cf928f382a6bdb083eb262e24d26140eba90d5996dbf2bcd81e5700fe674bb` |
| `23-hud-pause-575-318.png` | `aafc5d91a861550d76c4ce06bedb9a5ff9f98ea8625478e339895961aae37cf6` |
| `24-hud-purple-617-318.png` | `932ce6d81c570cfe82f0858ff82541da7546dd0afac36b60a55165574a1301be` |
| `25-hud-zoom-in-503-353.png` | `3184d185740473ada37e8e720538844d39bf8028a3f3e42ea91e6de52340b359` |
| `26-hud-zoom-out-620-353.png` | `0e9496fbf26f2596fbdb5a082a10a1d7d5288a08493fce8675e7a59f6a896815` |
| `27-hud-dpad-up-561-372.png` | `7b59165d52fdfcf0f1642d7d392d3dd556305e70591eac64e2aa39c86f05fcf0` |
| `28-hud-dpad-left-520-402.png` | `8e3f4d5e1998b69bb9db15f85513c753e0d5c4a7859b6ea0c817448b4307385a` |
| `29-hud-dpad-right-602-402.png` | `7fff0aa4f7282d72c796efd662a88095729564f08df9d4723200a22d4b4449d5` |
| `30-hud-dpad-down-561-432.png` | `3cb9e0ed5e4b655769dde2b757a199c96f7e99e71c488069e565b70b4087b389` |
| `31-hud-dpad-centre-561-402.png` | `4a7b148c03bd8655072d7e3136e9dbf283ea7fa692f0b3a00d2de3df9189a2c6` |
| `32-hud-bottom-left-503-457.png` | `41e39be94788732d2105596559570d78f922628234117656fffdfdaf9b145c08` |
| `33-hud-bottom-right-614-457.png` | `1c67577ea8ced125ad0db153b4732e48558bd795eb1c612c613283f0b830866a` |
| `34-hud-top-tf-icon-35-10.png` | `2a2be22b6a08e706b8153eebd05fe33bf02c4ebde88bce67233ab042fbbef018` |
| `35-hud-top-tf1-72-10.png` | `1c4491ca329ae770d295a0c0157d6aa227c7f77c17abca3f45ffad7925f2d3ab` |
| `36-hud-top-slot2-103-10.png` | `331c5cdd95bf5d8ef83a7ae39382c609b5df76ff0d2ee155183366d2cb97500a` |
| `37-hud-top-fighters-320-10.png` | `f80a06867e1d501a4c97ab1427ba85e6965bd0437c5f346ce45e3da6b70538c7` |
| `38-hud-top-btn1-361-10.png` | `63f055a028a863ac747e85c2aaaf19c7ab8e30cc8966233cdfef81e0bcdcabc8` |
| `39-hud-top-btn2-390-10.png` | `01d13eb0409f6c39bd9d7d63e66729137afaeb8f15df3f0f0f61ab5385328662` |
| `40-hud-top-btn3-418-10.png` | `32b6edea5549184daca90182448fbf766dc8bb7a744fcfc7e7219bbc634ec05f` |
| `41a-battle-at-averam-first-frame.png` | `ddd6fdfeff32dfcffef28f9319e1cdc2b41a16529bedd411672c577dd6c96516` |
| `41b-corvette-1-selected.png` | `bf597f12c704b534d3a532da732cef28f7ac98932cd2a3af69812288e65af70f` |
| `41-hud-top-btn4-447-10.png` | `42223d0a2bad55c61954f29300b15826a9adf419029468759e1efa06387d44e9` |
| `42-fighter-group-selected.png` | `b67cbf6fa142caa19857988046ed09f9c139efb62fe6259fef55857fa34c8354` |
| `43-fighter-group-text-556-55.png` | `7411073faa8b89abd5713035a1fa35d27d07f446b8f9dc9f7e91de408d4907e5` |
| `44-fighter-craft-picture-556-130.png` | `a0f2270f44804047fd0c3f3c4739a1dff563a371fa650be640f3a3afed12a04f` |
| `45-fighter-craft-count-556-180.png` | `42440521eb7bdd8243e859b0f6ae5052a07d9f0cc2bf1cd9a2c7b1df72bb8896` |
| `46-fighter-grid-left-527-220.png` | `14f6f5bffecc85ae5119e71f7539618afe330f1561fc0bb3242f50b2641a7c36` |
| `47-fighter-grid-right-604-215.png` | `7893ffe9c49c0defeb2856310f4110f24dfeaec5221cc722bdf2ae95f4f515df` |
| `48-fighter-order-button-522-248.png` | `ac9e4d27581858965c68bd426b7dd496d80c41d3386a522be1b1cff46308c646` |
| `49-fighter-missions-button-588-248.png` | `b42b9c97dc2a973e3aa9ab8977d5e94f758bdaa4cfa6e63db968027e130dfcca` |
| `50-options-top-527-62.png` | `35ca8504d48e2ae2fa90305d39c5f61470ba2dad8d8c32811ab46b74e67e59ee` |
| `51-options-fast-518-133.png` | `8e04756241c88de869bb636a58be976927f46691e735740128d90935579533a4` |
| `52-options-play-590-133.png` | `28333a1db1ac0346a47d48cf7c2a5cfca0ff546919e598c06c63d7dd5698645f` |
| `53-options-planet-530-227.png` | `5ca0aea092735df51c41bec7d6e7d01cec51c955ff520f108b01631d64468ae9` |
| `54-options-close-594-227.png` | `62bb209353113e37302a4d0e4def695f10e713c7e19f41aba30dfa03022157ff` |

## What the gate does

New scenario `production-tactical-tips-a0` in `run.mjs --battle` (appended at
the end of `tactical.catalog.json`). It reuses the Coruscant chase fixture
(65835 / 66091): A0 TAC-04-C014's paused battle, an Alliance Escort Carrier
with an X-wing (Red) and a Y-wing (Blue) squadron against Coruscant's home
fleet. No Rust fixture was added. The fixture's ready checks are shared with
`production-coruscant-chase` (`coruscantChaseFixture`), except for the
chase's own F1 counts.

The probe (`probeTacticalTipsA0`, 640 x 480 only):

1. clicks Task Force 1 (72, 10), then hovers 17 capital-panel and 25 HUD
   points;
2. clicks Blue Group (390, 10), then hovers 7 fighter-panel points;
3. clicks the purple Game Options button (617, 318), then hovers 5
   options-panel points.

For each point it moves to an empty spot of the battle view (the timers
reset), moves to the point, waits 1 s and takes a stable frame. Then it
checks:

- **The text.** Interface-test builds now log each tip once, when it shows
  (`[tooltip] shown text=... box=...`, `win_tooltip::log_shown`, called from
  both `win_tooltip::paint` and the battle's own `paint_tactical_tip`). The
  text must equal A0's. A point with no tip in A0 must show no tip: nothing
  logged and no box.
- **The place.** The box is found in the frame by its bottom-left corner
  (BORDER around FILL; the HUD's art holds both colours). It must follow the
  shared rule: x and y as `win_tooltip::tip_rect`, 19 high. Against A0 it
  needs the same y and height; the same x, or the right end at 639 where
  A0's box (or the port's wider one) is pushed left.
- **The width.** Within the accepted text deviation: Liberation Sans for
  GDI's Arial, user decision 2026-10-30. The other tip gates allow 12%; this
  one uses the same rule. Each delta is recorded below.
- **The frame.** The left column, the shared top and bottom rows, and each
  box's own right column, pixel for pixel against A0's. A0's arrow (12 by 21
  from the hot spot, taken from A0's own box) is skipped. Pixels where the
  port's last glyph reaches its right border are counted apart (see the open
  items). Every other pixel must match.

The Empire has no A0 for its side. Its tips are held to the same texts and
rule. Only the fighter slots are excluded, because they follow each side's
own squadrons.

## The fix

`paint_tactical_tip` drew the raw TEXTTACT string, so 0xddcd showed as
"Maneuvers & Tactics". A0 15 and 48 show "Maneuvers  Tactics". The galaxy
tips already dropped the '&' (`win_tooltip::shown_text`, commit 14fea358).
The battle draws its tips on its own path, so that change did not reach them.
`tactical_tool` now registers `tactical_tip_shown(text)`, which applies
`shown_text`. The box shrank from 118 to 110 pixels; A0's is 106.

Test `a_battle_tip_drops_the_ampersand_of_maneuvers_and_tactics`
(`tactical_view.rs`). It failed before the fix: left `"Maneuvers &
Tactics"`, right `"Maneuvers  Tactics"`.

## Results (run `2026-11-13T22-40-15-726Z-777840`, both sides pass)

Columns:

- **Box** is (x, y) and width; the height is 19 everywhere.
- **Δw** is the port's width minus A0's.
- **Frame** is differing pixels over pixels checked.

| Panel | Control | Point | A0 text | Port text (Alliance) | A0 box | Port box | Δw | Frame | Port text (Empire) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| capital | prev-capital | (493, 62) | Previous Capital Ship in Task Force | Previous Capital Ship in Task Force | (451, 83) 188 | (443, 83) 196 | 8 | 0/396 | Previous Capital Ship in Task Force |
| capital | next-capital | (621, 62) | Next Capital Ship in Task Force | Next Capital Ship in Task Force | (471, 83) 168 | (465, 83) 174 | 6 | 0/356 | Next Capital Ship in Task Force |
| capital | ship-picture | (556, 55) | none | none | none | none |  |  | none |
| capital | left-round-button | (502, 112) | none | none | none | none |  |  | none |
| capital | hull-bar | (527, 112) | Hull Integrity | Hull Integrity | (527, 133) 75 | (527, 133) 72 | -3 | 0/165 | Hull Integrity |
| capital | right-round-button | (571, 112) | none | none | none | none |  |  | none |
| capital | shield-bar | (600, 112) | Shield Strength | Shield Strength | (553, 133) 86 | (551, 133) 88 | 2 | 0/192 | Shield Strength |
| capital | shield-recharge | (503, 138) | Shield Recharge | Shield Recharge | (503, 159) 90 | (503, 159) 94 | 4 | 0/201 | Shield Recharge |
| capital | weapon-recharge | (530, 138) | Weapon Recharge | Weapon Recharge | (530, 159) 102 | (530, 159) 106 | 4 | 0/225 | Weapon Recharge |
| capital | tractor-beam | (557, 138) | Tractor Beam Power | Tractor Beam Power | (527, 159) 112 | (523, 159) 116 | 4 | 0/244 | Tractor Beam Power |
| capital | engines | (584, 138) | Engines | Engines | (584, 159) 47 | (584, 159) 49 | 2 | 0/115 | Engines |
| capital | hyperspace | (611, 138) | Hyperspace Engines | Hyperspace Engines | (527, 159) 112 | (523, 159) 116 | 4 | 0/244 | Hyperspace Engines |
| capital | fighter-list | (556, 172) | none | none | none | none |  |  | none |
| capital | fighter-bar | (556, 195) | none | none | none | none |  |  | none |
| capital | orders-text | (556, 220) | none | none | none | none |  |  | none |
| capital | maneuvers | (522, 248) | Maneuvers  Tactics | Maneuvers  Tactics | (522, 269) 106 | (522, 269) 110 | 4 | 0/233 | Maneuvers  Tactics |
| capital | missions | (588, 248) | Missions | Missions | (587, 269) 52 | (587, 269) 52 | 0 | 0/124 | Missions |
| hud | nav-1 | (497, 284) | Navigation Point Set 1 | Navigation Point Set 1 | (497, 305) 123 | (497, 305) 124 | 1 | 0/267 | Navigation Point Set 1 |
| hud | nav-2 | (536, 284) | Navigation Point Set 2 | Navigation Point Set 2 | (516, 305) 123 | (515, 305) 124 | 1 | 0/266 | Navigation Point Set 2 |
| hud | nav-3 | (575, 284) | Navigation Point Set 3 | Navigation Point Set 3 | (516, 305) 123 | (515, 305) 124 | 1 | 0/266 | Navigation Point Set 3 |
| hud | nav-4 | (614, 284) | Navigation Point Set 4 | Navigation Point Set 4 | (516, 305) 123 | (515, 305) 124 | 1 | 0/266 | Navigation Point Set 4 |
| hud | highlight-empire | (497, 318) | Highlight Empire Ships | Highlight Empire Ships | (497, 339) 127 | (497, 339) 127 | 0 | 0/275 | Highlight Empire Ships |
| hud | highlight-alliance | (530, 318) | Highlight Alliance Ships | Highlight Alliance Ships | (507, 339) 132 | (508, 339) 131 | -1 | 0/282 | Highlight Alliance Ships |
| hud | resume | (575, 318) | Resume Battle | Resume Battle | (558, 339) 81 | (555, 339) 84 | 3 | 0/182 | Resume Battle |
| hud | game-options | (617, 318) | Game Options | Game Options | (557, 339) 82 | (556, 339) 83 | 1 | 0/184 | Game Options |
| hud | zoom-in | (503, 353) | Zoom In | Zoom In | (503, 374) 51 | (503, 374) 50 | -1 | 0/121 | Zoom In |
| hud | zoom-out | (620, 353) | Zoom Out | Zoom Out | (578, 374) 61 | (580, 374) 59 | -2 | 0/138 | Zoom Out |
| hud | tilt-up | (561, 372) | Tilt Up | Tilt Up | (561, 393) 40 | (561, 393) 40 | 0 | 0/101 | Tilt Up |
| hud | rotate-left | (520, 402) | Rotate Left | Rotate Left | (520, 383) 63 | (520, 383) 64 | 1 | 0/147 | Rotate Left |
| hud | rotate-right | (602, 402) | Rotate Right | Rotate Right | (568, 383) 71 | (567, 383) 72 | 1 | 0/162 | Rotate Right |
| hud | tilt-down | (561, 432) | Tilt Down | Tilt Down | (561, 413) 56 | (561, 413) 56 | 0 | 0/133 | Tilt Down |
| hud | camera-subject | (561, 402) | Reset Camera Subject | Reset Camera Subject | (519, 383) 120 | (513, 383) 126 | 6 | 0/260 | Reset Camera Subject |
| hud | camera-view | (503, 457) | Reset Camera View | Reset Camera View | (503, 438) 106 | (503, 438) 112 | 6 | 0/233 (+3 ink) | Reset Camera View |
| hud | camera-remember | (614, 457) | Remember Camera Position | Remember Camera Position | (485, 438) 154 | (482, 438) 157 | 3 | 0/328 | Remember Camera Position |
| hud | task-forces | (35, 10) | Task Forces | Task Forces | (35, 31) 66 | (35, 31) 72 | 6 | 0/153 | Task Forces |
| hud | task-force-1 | (72, 10) | Task Force 1 | Task Force 1 | (72, 31) 70 | (72, 31) 76 | 6 | 0/161 | Task Force 1 |
| hud | unassigned | (103, 10) | Unassigned Task Force | Unassigned Task Force | (103, 31) 125 | (103, 31) 132 | 7 | 0/271 | Unassigned Task Force |
| hud | fighter-squadrons | (320, 10) | Fighter Squadrons | Fighter Squadrons | (320, 31) 102 | (320, 31) 104 | 2 | 0/225 | Fighter Squadrons |
| hud | fighter-slot-1 | (361, 10) | none | Red Fighter Group | none | (361, 31) 105 |  |  | Red Fighter Group |
| hud | fighter-slot-2 | (390, 10) | Blue Fighter Group | Blue Fighter Group | (390, 31) 105 | (390, 31) 107 | 2 | 0/231 | Blue Fighter Group |
| hud | fighter-slot-3 | (418, 10) | none | none | none | none |  |  | Green Fighter Group |
| hud | fighter-slot-4 | (447, 10) | none | none | none | none |  |  | none |
| fighter | group-text | (556, 55) | none | none | none | none |  |  | none |
| fighter | craft-picture | (556, 130) | none | none | none | none |  |  | none |
| fighter | craft-count | (556, 180) | none | none | none | none |  |  | none |
| fighter | grid-left | (527, 220) | none | none | none | none |  |  | none |
| fighter | grid-right | (604, 215) | none | none | none | none |  |  | none |
| fighter | fighter-maneuvers | (522, 248) | Maneuvers  Tactics | Maneuvers  Tactics | (522, 269) 106 | (522, 269) 110 | 4 | 0/233 | Maneuvers  Tactics |
| fighter | fighter-missions | (588, 248) | Missions | Missions | (587, 269) 52 | (587, 269) 52 | 0 | 0/124 | Missions |
| options | withdraw | (527, 62) | Withdraw From Battle | Withdraw From Battle | (518, 83) 121 | (518, 83) 121 | 0 | 0/262 | Withdraw From Battle |
| options | immediate-result | (518, 133) | Get Immediate Result | Get Immediate Result | (518, 154) 119 | (517, 154) 122 | 3 | 0/259 | Get Immediate Result |
| options | observe | (590, 133) | Observe Battle / TAKE COMMAND | Observe Battle / TAKE COMMAND | (452, 154) 187 | (446, 154) 193 | 6 | 0/394 | Observe Battle / TAKE COMMAND |
| options | options-game-options | (530, 227) | Game Options | Game Options | (530, 248) 82 | (530, 248) 83 | 1 | 0/185 | Game Options |
| options | close | (594, 227) | Close | Close | (594, 248) 35 | (594, 248) 36 | 1 | 0/91 | Close |

`fighter-slot-1` (361, 10) depends on the battle's state. A0's Red slot is
empty, and `FUN_0059b690` disables an empty slot, so A0 shows no tip. The
fixture's Red Group holds X-wings and shows "Red Fighter Group". The
Empire's Green slot (418, 10) is filled in its fixture, so it shows "Green
Fighter Group". The probe does not compare either against A0.

## Open items

- **Widths.** 18 cases (17 controls) differ from A0's width by 3 to 8
  pixels, at most 4.3%. All but one are wider:
  Previous/Next Capital Ship, Hull Integrity (3 narrower), the recharge and
  power icons, Maneuvers  Tactics, Resume Battle, Reset Camera
  Subject/View, Remember Camera Position, Task Forces, Task Force 1,
  Unassigned Task Force, Get Immediate Result and Observe Battle / TAKE
  COMMAND. These fall within the accepted Liberation Sans deviation and
  outside a 2-pixel bound. A wider box moves the pushed-left tips 1 to 8
  pixels further left.
- **Ink on the right border.** In "Reset Camera View" the last glyph's ink
  (the "w") reaches the right border: 3 pixels of 124..127 grey where A0
  has border grey 128. The gate counts these apart. The port's box adds a
  fixed pad to the text's advance, and Liberation Sans' "w" ink overhangs
  that advance.
- **Glyphs.** Inside the box, the port's text is anti-aliased grey and A0's
  is crisp GDI black. The interior is not compared.

## Gate runs (Linux, Arch Chromium 155.0.8059.84)

Run ids are UTC. Each was run with a temporary copy of `run.mjs` that reads
`.browser-linux.json`. The WASM was built from this tree
(`scripts/build-interface-test-wasm.sh`).

- `2026-11-13T22-21-01-948Z-687476`: the first run, before the fix and the
  log in the battle's paint. No text was logged, and Maneuvers' box was 118
  wide.
- `2026-11-13T22-31-39-936Z-728613`, `2026-11-13T22-36-50-950Z-759831`: after
  the fix. All texts matched A0. These runs failed on probe rules that were
  then corrected: A0's pointer exclusion, a port box pushed where A0's was
  not, and the shared fixture checks.
- `2026-11-13T22-40-15-726Z-777840`: `--scenario=production-tactical-tips-a0`,
  2 of 2 pass.
- `2026-11-13T22-43-28-740Z-791814`: `run.mjs --battle --all`. All 180
  executions pass: 45 scenarios, both sides, both viewports, among them
  `production-capital-panel-missions-tip` and the new scenario. At 1280 x 800
  the new probe records a skip, because A0 is 640 x 480. The machine's load
  average was about 30 on 16 cores during the run (other sessions).

## Checks

- `cargo test -p rebellion-render --lib`: pass (1,589, 6 ignored).
- `cargo run -q -p provenance-scan -- check`: 0 uncited items.
- `rustfmt --check` on `win_tooltip.rs`: clean. `tactical_view.rs` has
  only its baseline diffs, none in the touched lines.
- Mutants: `cargo mutants --re` does not filter the field-deletion mutants
  of other functions, so the touched functions' mutants were applied by a
  script. Each was run against the `win_tooltip` tests and the new test:
  - `TipTimer::is_shown`: 4 of 4 caught (true, false, `&&` to `||`, `!`
    deleted). New test
    `a_timer_reports_its_tip_shown_only_between_the_delay_and_the_autopop`.
  - `tactical_tip_shown`: 2 of 2 caught.
  - `log_shown` (replace with `()`): not run. Its body exists only in
    interface-test builds, and the browser probe is what reads it: with no
    log line, every tipped case fails on its text.
