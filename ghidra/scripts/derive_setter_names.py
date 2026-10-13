#!/usr/bin/env python3
"""Derive setter and view-sync names from the named notifiers.

A setter writes a field, calls its sync routine on both side views
(FUN_00539fd0(1) and (2)) and then its own notifier slot. The sync routine
(guard FUN_0053a010, master FUN_00539fd0(0)) copies the field and calls a
view vtable slot, which notifier-slots.tsv names.

Usage: derive_setter_names.py DECOMPILED_DIR notifier-slots.tsv OUT.tsv
"""
import collections
import csv
import pathlib
import re
import sys

corpus, slots_path, out_path = map(pathlib.Path, sys.argv[1:4])

by_slot = collections.defaultdict(set)  # slot -> {(function, name, field)}
for row in csv.DictReader(slots_path.open(), delimiter="\t"):
    if row["slot"] and not row["name"].startswith("?"):
        by_slot[int(row["slot"], 16)].add((int(row["function"], 16), row["name"], row["field"]))

THIS = r"(?:param_1|in_ECX)"
VALUE = r"param_[12]"
N = r"(0x[0-9a-f]+|\d+)"
VCALL = re.compile(rf"\(\*\*\(code \*\*\)\(\*{THIS} \+ {N}\)\)\(")
SYNC_CALLS = re.compile(r"FUN_00539fd0\(1\);\s*(FUN_[0-9a-f]{8})\(")
WORD = re.compile(rf"{THIS}\[{N}\] = {VALUE};")
BYTE = re.compile(rf"\*\([\w ]+\*\)\((?:\(int\))?{THIS} \+ {N}\) = {VALUE};")
FLAG = re.compile(rf"FUN_0053a640\({N},{VALUE},{THIS} \+ {N}\)")
PAIR = re.compile(rf"FUN_00520580\({THIS} \+ {N}\);")
# `this` declared as a pointer to 4-byte words scales `this + n` by 4.
WORDS = re.compile(rf"(?:int|uint|undefined4) \*{THIS}\b")
num = lambda text: int(text, 0)


def read(name):
    path = corpus / f"{name}.c"
    return path.read_text(errors="ignore") if path.exists() else ""


# Sync routines: one view-slot call behind the view guard.
syncs = {}
for path in corpus.glob("FUN_*.c"):
    text = path.read_text(errors="ignore")
    if "FUN_0053a010()" not in text or "FUN_00539fd0(0)" not in text:
        continue
    calls = {num(match) for match in VCALL.findall(text)}
    if len(calls) == 1:
        syncs[path.stem] = calls.pop()


# A class's setters, sync routines and notifiers are laid out in that order
# in one compilation unit, so a sync routine's notifier follows it closely.
# A slot offset alone is not enough: another class may have a named notifier
# at the same offset while this one has none.
UNIT_SPAN = 0x3000
SETTER_SPAN = 0xA000


def notifier_for(address, slot):
    """The notifier at `slot` in the sync routine's own unit, if any."""
    candidates = [
        item for item in by_slot.get(slot, ()) if 0 <= item[0] - address < UNIT_SPAN
    ]
    if not candidates:
        return None, "none"
    if len({name for _, name, _ in candidates}) > 1:
        return None, "ambiguous"
    return candidates[0], "unit"


def split(name, field):
    stem = name[: -len("Notif")]
    if field and stem.endswith(field):
        return stem[: -len(field)], field
    return "", stem


rows = []
named = set()
for path in sorted(corpus.glob("FUN_*.c")):
    text = path.read_text(errors="ignore")
    if "FUN_0053a000()" not in text:
        continue
    sync_names = set(SYNC_CALLS.findall(text))
    if len(sync_names) != 1:
        continue
    sync = sync_names.pop()
    if sync not in syncs:
        continue
    setter = int(path.stem[4:], 16)
    notifier, confidence = notifier_for(int(sync[4:], 16), syncs[sync])
    # The linker folds identical sync routines, so a setter of another class
    # may call this one; its own notifier would not lie after it.
    if notifier and not 0 <= notifier[0] - setter < SETTER_SPAN:
        notifier, confidence = None, "folded"
    master_slots = {num(match) for match in VCALL.findall(text)}
    master_slot = f"0x{master_slots.pop():x}" if len(master_slots) == 1 else ""
    offset = mask = ""
    scale = 4 if WORDS.search(text) else 1
    if match := FLAG.search(text):
        mask, offset = f"0x{num(match.group(1)):x}", f"0x{num(match.group(2)) * scale:x}"
    elif match := WORD.search(text):
        offset = f"0x{num(match.group(1)) * 4:x}"
    elif match := BYTE.search(text):
        offset = f"0x{num(match.group(1)):x}"
    elif match := PAIR.search(text):
        offset = f"0x{num(match.group(1)) * scale:x}"
    if notifier is None:
        rows.append((path.stem[4:], "", "setter", "", offset, mask, master_slot, confidence))
        continue
    _, notif_name, field = notifier
    owner, what = split(notif_name, field)
    for address, kind, verb in ((path.stem[4:], "setter", "Set"), (sync[4:], "sync", "Sync")):
        name = f"{owner}_{verb}{what}" if owner else f"{verb}{what}"
        if (address, name) in named:
            continue
        named.add((address, name))
        rows.append((address, name, kind, notif_name, offset, mask, master_slot, confidence))

rows.sort()
with out_path.open("w", newline="") as handle:
    writer = csv.writer(handle, delimiter="\t", lineterminator="\n")
    writer.writerow(["function", "name", "kind", "notifier", "field_offset", "mask", "master_slot", "confidence"])
    writer.writerows(rows)

counts = collections.Counter((row[2], row[7]) for row in rows)
print(f"sync routines found: {len(syncs)}")
for key, count in sorted(counts.items()):
    print(key, count)
