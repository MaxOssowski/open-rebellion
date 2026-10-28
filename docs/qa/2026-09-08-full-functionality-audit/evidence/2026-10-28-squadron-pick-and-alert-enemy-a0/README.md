# Squadrons are not picked by a click; the alert lists the enemy in full (P24, A0 VM)

Date: 2026-10-28. This settles two items that were still open on tracker
item 9, both against the original in the Windows VM. The session is the
one in `../2026-10-28-squadron-parent-after-battle-vm/`: the Battle at Mon
Calamari from the clean `resource turn 14` save. Captures are in
`.artifacts/a0-vm/2026-10-28-ctrl-squadrons/` (ignored; hashes below).

## A click on a squadron in the 3D view

`FUN_005c2d20` picks the first visual whose object is marked pickable. It
was not traced whether a squadron's visual is.

In the VM the camera followed Imperial Star Destroyer 1, zoomed in so that
its TIE squadrons showed as separate craft around it
(`pick-isd-selected-ties-near`):

- A left click on a TIE, on its pixels, selected nothing. It cleared the
  ISD's selection, as a click on empty space does (`pick-tie-click-clears`).
  Two more TIEs and a repeated click gave the same
  (`pick-tie-click-again`).
- A left click on the ISD in the same view selected it at once
  (`pick-isd-click`).
- With the ISD selected, a right click on a TIE changed nothing
  (`pick-tie-right-click`).

So our own squadrons cannot be picked in the 3D view; a group is selected
only through its button. The port does the same: its left click takes
capital ships only, and a click elsewhere clears the selection.

Not reached: a right click on an enemy squadron. There were no Alliance
fighters in this battle. The port keeps its rule from s141
(`enemy_squadron_hit`): the right click targets the squadron.

## The enemy's forces in the Battle Alert

The question was whether the original hides an enemy's ships and
squadrons it has no intelligence on.

The Imperial player's Battle Alert at Mon Calamari listed the Alliance's
forces in full on the Alliance tab: Fleet 4, Swift, Fleet 9, Quick, and
the Alliance Fleet Regiment (`mc-alliance-forces`). The Empire had no
agents or presence at Mon Calamari before the fleet arrived. The list
walks the side's fleets with mode 1 (`FUN_004ffef0(system, 1, side)`,
usable). It has no intelligence filter. The port lists every fleet of the
side the same way (`battle_alert_rows`). No change.

## Order voices

The three order voices were traced statically on 2026-10-27
(`../2026-10-27-order-voice-conditions/`), and no `hyp:` tag is left in
the tactical code. They were not listened to in the VM.

## Captures

| Capture | SHA-256 |
|---|---|
| `pick-isd-selected-ties-near.png` | `f4f79467f12206885cb2115e91ecc000918810d36d4ff07f154bda14d8bd0f51` |
| `pick-tie-click-clears.png`, `pick-tie-click-again.png` | `c5ca1e36a0ae3a2fbe76c7747fd6791f5ace2fcf8cb8c2622657ee36eb00e579` |
| `pick-isd-click.png` | `6f0bf2d98bcc36840bd486ea3fb527c23ccfa8c279c6427217a57ef617dc0004` |
| `pick-tie-right-click.png` | `f74743c90e8d85491ce3fcd815dbfee312ba4dab80ae5a4ebcca9cbe95f91613` |
| `mc-alliance-forces.png` | `1b9accee2681515507691e0b1cf331abe22f37851b182b961ff6ccd699c07940` |
