#!/usr/bin/env python3
"""Derive each class's field layout and method list for ApplyClassTypes.

A field comes from a named setter (setters.tsv): its offset, its flag mask
when it is one bit of a flags word, and its name. It belongs to the class
that introduces the setter's notifier in its vtable (class-slots.tsv), and
every class below inherits it. SEEDS adds fields traced by hand.

A class's methods are the functions its vtable introduces and the setters
and sync routines of its own fields.

Usage: derive_class_fields.py classes.tsv class-slots.tsv setters.tsv
       notifier-slots.tsv FIELDS.tsv METHODS.tsv
"""
import collections
import csv
import pathlib
import re
import sys

classes_path, slots_path, setters_path, notifiers_path, fields_path, methods_path = map(
    pathlib.Path, sys.argv[1:7]
)

# (class, offset, size, name): fields without a named setter, from
# ghidra/notes/side-resource-monitors.md and object-state-flags.md.
SEEDS = [
    ("GameObj", 0x24, 4, "KeyAndSide"),
    ("GameObj", 0x2C, 4, "ClassRecord"),
    ("Side", 0x58, 4, "MaintCapacity"),
    ("Side", 0x5C, 4, "MaintCapacityAllocated"),
    ("Side", 0x60, 4, "MinePool"),
    ("Side", 0x64, 4, "MinePoolAllocated"),
    ("Side", 0x68, 4, "RefineryPool"),
    ("Side", 0x6C, 4, "RefineryPoolAllocated"),
    ("Side", 0x74, 4, "MaintUsed"),
    ("Side", 0x78, 4, "RawMaterials"),
    ("Side", 0x7C, 4, "RefinedMaterials"),
    ("Side", 0x80, 4, "RawRequestCount"),
    ("Side", 0x84, 4, "RefinedRequestCount"),
    ("Side", 0x88, 4, "RawRequests"),
    ("Side", 0x8C, 4, "RefinedRequests"),
    ("Side", 0xA8, 4, "OverLimit"),
    ("ProcFacil", 0x54, 4, "CycleTimer"),
    ("ProcFacil", 0x58, 4, "State"),
    ("ProcFacil", 0x5C, 4, "ETC"),
    ("ProcFacil_00662778", 0x64, 4, "Capacity"),
    ("ProcFacil_00662778", 0x68, 4, "CapacityAllocated"),
]

classes = {row["vtable"]: row for row in csv.DictReader(classes_path.open(), delimiter="\t")}
by_name = {row["class"]: vtable for vtable, row in classes.items()}
slots = collections.defaultdict(dict)  # vtable -> slot -> function
introduced = collections.defaultdict(set)  # function -> vtables introducing it
for row in csv.DictReader(slots_path.open(), delimiter="\t"):
    slots[row["vtable"]][row["slot"]] = row["function"]
    if row["status"] != "inherited":
        introduced[row["function"]].add(row["vtable"])


def depth(vtable):
    count = 0
    while classes[vtable]["parent_vtable"]:
        vtable = classes[vtable]["parent_vtable"]
        count += 1
    return count


def ancestors(vtable):
    chain = []
    while vtable:
        chain.append(vtable)
        vtable = classes[vtable]["parent_vtable"]
    return chain


notifier_function = {}
for row in csv.DictReader(notifiers_path.open(), delimiter="\t"):
    notifier_function.setdefault(row["name"], row["function"])


def introducer(function):
    """The class that first has `function` in its vtable."""
    vtables = introduced.get(function, set())
    roots = [v for v in vtables if not (set(ancestors(v)[1:]) & vtables)]
    if len(roots) == 1:
        return roots[0]
    return None


own_fields = collections.defaultdict(dict)  # vtable -> (offset, mask) -> (name, size, source)
methods = {}
skipped = 0
setter_rows = list(csv.DictReader(setters_path.open(), delimiter="\t"))
for row in setter_rows:
    if not row["name"]:
        continue
    owner = introducer(notifier_function.get(row["notifier"], ""))
    if owner is None:
        # Several unrelated vtables carry the notifier (the side views do):
        # fall back on the class the setter's name gives.
        owner = by_name.get(re.split(r"_(?:Set|Sync)", row["name"])[0])
    if owner is None:
        skipped += row["kind"] == "setter"
        continue
    methods[row["function"]] = owner
    if row["kind"] != "setter" or not row["field_offset"]:
        continue
    name = re.sub(r"^.*?Set", "", row["name"], count=1)
    key = (int(row["field_offset"], 16), int(row["mask"], 16) if row["mask"] else 0)
    own_fields[owner].setdefault(key, (name, 4, row["function"]))

for class_name, offset, size, name in SEEDS:
    if class_name in by_name:
        own_fields[by_name[class_name]].setdefault((offset, 0), (name, size, "seed"))

# A plain field and a flags word, or two plain fields, at one offset: keep
# the flags (they come from the code), and drop a seed that a setter names.
field_rows = []
typed = [v for v in classes if any(a in own_fields for a in ancestors(v))]
for vtable in sorted(typed, key=lambda v: (depth(v), v)):
    merged = {}
    for ancestor in reversed(ancestors(vtable)):
        for key, value in own_fields.get(ancestor, {}).items():
            merged[key] = value + (classes[ancestor]["class"],)
    flag_offsets = {offset for offset, mask in merged if mask}
    for (offset, mask), (name, size, source, owner) in sorted(merged.items()):
        if not mask and offset in flag_offsets:
            continue
        field_rows.append((
            classes[vtable]["class"], vtable, f"0x{offset:x}", size, name,
            f"0x{mask:x}" if mask else "", owner, source,
        ))

# Methods: what each typed class's vtable introduces.
for function, vtables in introduced.items():
    owner = introducer(function)
    if owner in typed:
        methods.setdefault(function, owner)

with fields_path.open("w", newline="") as handle:
    writer = csv.writer(handle, delimiter="\t", lineterminator="\n")
    writer.writerow(["class", "vtable", "offset", "size", "name", "mask", "declared_in", "source"])
    writer.writerows(field_rows)
with methods_path.open("w", newline="") as handle:
    writer = csv.writer(handle, delimiter="\t", lineterminator="\n")
    writer.writerow(["function", "class"])
    writer.writerows(sorted((function, classes[owner]["class"]) for function, owner in methods.items() if owner in typed))

own = sum(len(fields) for fields in own_fields.values())
print(f"classes with fields: {len(typed)} ({len(own_fields)} declare their own)")
print(f"fields declared: {own}; rows with inheritance: {len(field_rows)}")
print(f"methods: {sum(1 for o in methods.values() if o in typed)}; setters with no single owner: {skipped}")
for vtable in sorted(own_fields, key=lambda v: -len(own_fields[v]))[:12]:
    print(f"  {classes[vtable]['class']}: {len(own_fields[vtable])}")
