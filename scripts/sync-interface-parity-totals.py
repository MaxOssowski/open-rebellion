#!/usr/bin/env python3
"""Recompute the interface parity ledgers' derived totals from the cells.

The surface ledger's acceptance_summary, the reverse-engineering ledger's
coverage_summary and the audit report's scope and summary totals restate
counts of the surface ledger. Cells and surfaces were added and excluded
after those totals were written; this script recomputes them, editing each
value in place so the files keep their formatting.
Run: python3 scripts/sync-interface-parity-totals.py
"""
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
AUDIT = ROOT / "docs/qa/2026-09-10-interface-parity-audit"
SURFACE = AUDIT / "2026-10-06-surface-ledger.json"
REVERSE = AUDIT / "reverse-engineering-ledger.json"
REPORT = AUDIT / "audit-report.json"


def set_value(path, block, key, value):
    """Replace "key": <old> inside the object that follows "block": once."""
    text = path.read_text()
    start = text.index(f'"{block}"')
    end = text.index("}", start)
    segment = text[start:end]
    pattern = re.compile(rf'("{key}":\s*)("[^"]*"|\d+)')
    assert len(pattern.findall(segment)) == 1, (path.name, block, key)
    rendered = json.dumps(value)
    segment = pattern.sub(lambda m: m.group(1) + rendered, segment)
    path.write_text(text[:start] + segment + text[end:])


surface = json.loads(SURFACE.read_text())
reverse = json.loads(REVERSE.read_text())
surfaces = surface["surfaces"]
cells = [cell for s in surfaces for cell in s["acceptance_cells"]]
required = [s for s in surfaces if s["required"]]
counted = [cell for cell in cells if cell["counts_toward_parity"]]
excluded = [cell for cell in cells if cell["status"] == "excluded"]
battle = [
    cell
    for s in surfaces
    if re.fullmatch(r"TAC-0[1-7]", s["id"])
    for cell in s["acceptance_cells"]
    if cell["counts_toward_parity"]
]
with_packages = [s for s in required if s["reverse_engineering_package_ids"]]

for key, value in [
    ("surface_families", len(surfaces)),
    ("required_surface_families", len(required)),
    ("baseline_cells", len(cells)),
    ("required_baseline_cells", len(counted)),
    ("excluded_baseline_cells", len(excluded)),
    ("required_surfaces_with_retrieval_packages", len(with_packages)),
]:
    set_value(SURFACE, "acceptance_summary", key, value)
for key, value in [
    ("work_packages", len(reverse["work_packages"])),
    ("required_surface_families", len(required)),
    ("required_surface_families_with_packages", len(with_packages)),
    ("required_baseline_cells", len(counted)),
    ("space_battle_baseline_cells", len(battle)),
]:
    set_value(REVERSE, "coverage_summary", key, value)
set_value(REPORT, "scope", "surface_families", len(surfaces))
set_value(REPORT, "summary", "required_baseline_cells", f"{len(counted)}_pending_or_open")
set_value(
    REPORT,
    "summary",
    "retrieval_package_coverage",
    f"{len(with_packages)}_of_{len(required)}_required_surface_families",
)
set_value(
    REPORT,
    "summary",
    "space_battle_scope",
    f"{len(battle)}_baseline_cells_across_TAC-01_through_TAC-07",
)
accepted = [cell for cell in counted if cell["status"] in ("pass", "passed")]
set_value(
    REPORT,
    "summary",
    "strictly_accepted_required_cells",
    f"{len(accepted)}_of_{len(counted)}",
)
for path in (SURFACE, REVERSE, REPORT):
    json.loads(path.read_text())
print(len(surfaces), len(required), len(cells), len(counted), len(excluded), len(battle))
