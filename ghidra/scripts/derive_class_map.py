#!/usr/bin/env python3
"""Build the class map: every vtable with its class name, parent, and the
functions in its slots.

A vtable is a run of code pointers in .rdata whose address a constructor
or destructor stores into an object (`mov dword ptr [reg], imm32`). A class
derives from the class whose vtable its constructor's callee stores, and a
slot is inherited when it holds the parent's function, overridden when it
holds another, and new past the parent's length.

Names come from the notifier strings: a class is named after the prefix of
the notifiers it adds to its parent's (notifier-slots.tsv), or after KNOWN.

Event and timer classes take their id from events.tsv.

Usage: derive_class_map.py REBEXE.EXE notifier-slots.tsv function-ranges.tsv
       events.tsv CLASSES.tsv SLOTS.tsv NAMES.tsv
"""
import bisect
import collections
import csv
import pathlib
import re
import struct
import sys

exe_path, slots_path, ranges_path, events_path, classes_path, out_slots_path, names_path = map(
    pathlib.Path, sys.argv[1:8]
)

# Classes identified by hand (ghidra/notes/side-resource-monitors.md,
# facility-ownership.md).
KNOWN = {
    0x006606A0: "Mine",
    0x006608C8: "Refinery",
    0x00660AF8: "Side",
    0x00663698: "CoreSystem",
    0x00664098: "RimSystem",
}

for row in csv.DictReader(events_path.open(), delimiter="\t"):
    if row["vtable"]:
        tag = ("Timer" if row["kind"] == "timer" else "Event") + f"{int(row['id'], 16):03X}"
        KNOWN[int(row["vtable"], 16)] = tag

exe = exe_path.read_bytes()
pe = struct.unpack_from("<I", exe, 0x3C)[0]
optional = struct.unpack_from("<H", exe, pe + 20)[0]
sections = {}
for index in range(struct.unpack_from("<H", exe, pe + 6)[0]):
    at = pe + 24 + optional + index * 40
    name = exe[at : at + 8].rstrip(b"\0").decode()
    _, address, raw_size, raw = struct.unpack_from("<4I", exe, at + 8)
    sections[name] = (0x400000 + address, raw_size, raw)
text_start, text_size, text_raw = sections[".text"]
rdata_start, rdata_size, rdata_raw = sections[".rdata"]
text = exe[text_raw : text_raw + text_size]


def in_text(address):
    return text_start <= address < text_start + text_size


def rdata_pointer(address):
    if rdata_start <= address <= rdata_start + rdata_size - 4:
        return struct.unpack_from("<I", exe, rdata_raw + address - rdata_start)[0]
    return None


ranges = sorted(
    (int(row["first"], 16), int(row["last"], 16), int(row["function"], 16), row["name"])
    for row in csv.DictReader(ranges_path.open(), delimiter="\t")
)
range_starts = [first for first, _, _, _ in ranges]
function_names = {function: name for _, _, function, name in ranges}


def owner(address):
    index = bisect.bisect_right(range_starts, address) - 1
    if index >= 0 and ranges[index][0] <= address <= ranges[index][1]:
        return ranges[index][2]
    return None


# Vtable stores: `mov dword ptr [reg], imm32` with imm32 in .rdata.
writers = collections.defaultdict(set)  # vtable -> functions storing it
for match in re.finditer(rb"\xc7[\x00-\x03\x06\x07](....)", text, re.S):
    target = struct.unpack("<I", match.group(1))[0]
    first = rdata_pointer(target)
    if first is None or not in_text(first):
        continue
    function = owner(text_start + match.start())
    if function is not None:
        writers[target].add(function)

starts = sorted(writers)
vtables = {}
for index, start in enumerate(starts):
    limit = starts[index + 1] if index + 1 < len(starts) else rdata_start + rdata_size
    slots = []
    address = start
    while address < limit:
        target = rdata_pointer(address)
        if target is None or not in_text(target):
            break
        slots.append(target)
        address += 4
    vtables[start] = slots

# Direct calls made by each function.
calls = collections.defaultdict(set)
for match in re.finditer(rb"\xe8", text):
    at = match.start()
    if at + 5 > len(text):
        continue
    target = (text_start + at + 5 + struct.unpack_from("<i", text, at + 1)[0]) & 0xFFFFFFFF
    caller = owner(text_start + at)
    if caller is not None and in_text(target):
        calls[caller].add(target)

writes = collections.defaultdict(set)  # function -> vtables it stores
for vtable, functions in writers.items():
    for function in functions:
        writes[function].add(vtable)


def common_prefix(a, b):
    count = 0
    for left, right in zip(a, b):
        if left != right:
            break
        count += 1
    return count


# Parent: a vtable stored by a function this class's constructor or
# destructor calls; of several, the one sharing the most slots.
parents = {}
for vtable, functions in writers.items():
    candidates = set()
    for function in functions:
        for callee in calls[function]:
            candidates |= writes.get(callee, set())
    candidates.discard(vtable)
    candidates = [c for c in candidates if len(vtables[c]) <= len(vtables[vtable])]
    if candidates:
        parents[vtable] = max(
            candidates,
            key=lambda c: (sum(a == b for a, b in zip(vtables[c], vtables[vtable])), len(vtables[c])),
        )

# Notifier names per vtable and function.
notifier_prefix = {}
for row in csv.DictReader(slots_path.open(), delimiter="\t"):
    if row["name"].startswith("?") or not row["field"]:
        continue
    stem = row["name"][: -len("Notif")]
    if stem.endswith(row["field"]) and len(stem) > len(row["field"]):
        notifier_prefix[int(row["function"], 16)] = stem[: -len(row["field"])]


def depth(vtable):
    count = 0
    while vtable in parents and count < 32:
        vtable = parents[vtable]
        count += 1
    return count


class_names = {}
for vtable in sorted(vtables, key=depth):
    parent = parents.get(vtable)
    inherited = set(vtables[parent]) if parent else set()
    own = collections.Counter(
        notifier_prefix[function]
        for function in vtables[vtable]
        if function in notifier_prefix and function not in inherited
    )
    if vtable in KNOWN:
        name = KNOWN[vtable]
    elif len(own) == 1:
        name = next(iter(own))
    elif parent:
        name = class_names[parent].split("_0")[0] + f"_{vtable:08x}"
    else:
        name = f"Class_{vtable:08x}"
    class_names[vtable] = name
# Several classes may add notifiers with one prefix; keep the first plain.
seen = collections.Counter()
for vtable in sorted(class_names, key=lambda v: (depth(v), v)):
    name = class_names[vtable]
    seen[name] += 1
    if seen[name] > 1:
        class_names[vtable] = f"{name}_{vtable:08x}"

slot_rows = []
first_owner = collections.defaultdict(set)  # function -> classes introducing it
for vtable in sorted(vtables):
    parent = parents.get(vtable)
    parent_slots = vtables[parent] if parent else []
    for index, function in enumerate(vtables[vtable]):
        if index >= len(parent_slots):
            status = "new"
        elif parent_slots[index] == function:
            status = "inherited"
        else:
            status = "override"
        if status != "inherited":
            first_owner[function].add(vtable)
        slot_rows.append((
            f"{vtable:08x}", class_names[vtable], f"0x{index * 4:x}", f"{function:08x}",
            function_names.get(function, ""), status,
        ))

with classes_path.open("w", newline="") as handle:
    writer = csv.writer(handle, delimiter="\t", lineterminator="\n")
    writer.writerow(["vtable", "class", "parent_vtable", "parent", "slots", "stored_by"])
    for vtable in sorted(vtables):
        parent = parents.get(vtable)
        writer.writerow([
            f"{vtable:08x}", class_names[vtable],
            f"{parent:08x}" if parent else "", class_names.get(parent, ""),
            len(vtables[vtable]), " ".join(f"{f:08x}" for f in sorted(writers[vtable])),
        ])

with out_slots_path.open("w", newline="") as handle:
    writer = csv.writer(handle, delimiter="\t", lineterminator="\n")
    writer.writerow(["vtable", "class", "slot", "function", "name", "status"])
    writer.writerows(slot_rows)

# Name a still-unnamed function after the one class that introduces it.
slot_of = {}
for vtable_hex, _, slot, function_hex, _, status in slot_rows:
    if status != "inherited":
        slot_of.setdefault((int(function_hex, 16), int(vtable_hex, 16)), slot)
named = 0
with names_path.open("w", newline="") as handle:
    writer = csv.writer(handle, delimiter="\t", lineterminator="\n")
    writer.writerow(["function", "name"])
    for function, owners in sorted(first_owner.items()):
        current = function_names.get(function)
        if len(owners) != 1 or current is None or not current.startswith("FUN_"):
            continue
        vtable = next(iter(owners))
        writer.writerow([f"{function:08x}", f"{class_names[vtable]}_vt{slot_of[(function, vtable)][2:]}"])
        named += 1

total = len(function_names)
in_slots = {int(row[3], 16) for row in slot_rows}
defined_in_slots = in_slots & set(function_names)
plain = sum(1 for name in function_names.values() if name.startswith(("FUN_", "thunk_FUN_")))
print(f"vtables: {len(vtables)}, with a parent: {len(parents)}, slot rows: {len(slot_rows)}")
print(f"classes named from notifiers or by hand: "
      f"{sum(1 for name in class_names.values() if not re.search(r'_[0-9a-f]{8}$', name))}")
print(f"functions: {total}; still default-named: {plain}; in a vtable slot: {len(defined_in_slots)}; "
      f"slot targets that are undefined stubs: {len(in_slots - set(function_names))}")
print(f"names to apply: {named}")
