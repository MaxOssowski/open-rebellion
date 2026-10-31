# Build Selection's text against A0 (interface text)

Date: 2026-10-31. Follows `../2026-10-31-finders-text-a0/`.

## Change

Build Selection's text now goes through `theme::paint_gdi_text`:
- **Title.** Font 5, centred from its box's top.
- **"Number to build".** Font 4, `DT_RIGHT`.
- **Time labels.** Font 10, `DT_VCENTER`.
- **Costs and the number box.** Font 4.
- **Times.** Font 10, `DT_RIGHT`, clipped to their field.
- **Class name.** Font 10, centred under its mini.

The sizes were already right: the title is bold 13 ppem and the time
labels are 11 ppem in A0 `build-selection/10-build`. The placement and
the widths were not.

## Check

`build-selection.mjs --a0=.artifacts/a0-vm/build-selection` compares the
Alliance's window with A0 `10-build`. Both windows are at (192, 84). A0 is
black and white; the port's antialiased edges count from 120.

| Label | A0 ink | Port ink |
|---|---|---|
| "Best Time To Completion:" | [10, 148, 126, 157] | [11, 148, 127, 157] |
| "Best Time To Deployment:" | [10, 168, 130, 177] | [11, 168, 131, 177] |
| "Number to build:" | [25, 199, 135, 208] | [25, 198, 136, 208] |

On the old code (`build_selection.rs` from HEAD, rebuilt) the check
failed: "Best Time To Completion:" ran to 134 against A0's 126.

## Gates

`gates.txt`.
