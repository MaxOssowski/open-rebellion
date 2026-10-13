#!/usr/bin/env python3
"""Build the event table: id, factory, class vtable, fire slot, handler,
and the functions that raise or arm the event.

FUN_0051ef80 registers a factory per event id (FUN_0054f140). A factory
allocates the event and calls its constructor, which installs the class
vtable; slot +0x20 is the fire function, which calls the handler between
FUN_004fd450 and FUN_004fd4d0. A timer class keeps the shared fire
function FUN_005862a0 there and its own expiry function in slot +0x30.

An event is raised through FUN_0053f950 (FUN_0053f9c0, FUN_0053fbd0); a
timer is armed through FUN_0053fa60 and its period set through FUN_0053fab0
(FUN_0053fb00). FUN_0053fe40 is the notify helper and raises nothing.

A sender that sits in a class vtable at the slot a named setter calls is
that setter's change notifier, which names the event.

Usage: derive_event_table.py DECOMPILED_DIR REBEXE.EXE notifier-slots.tsv
       setters.tsv OUT.tsv NAMES.tsv
"""
import collections
import csv
import pathlib
import re
import struct
import sys

corpus, exe_path, slots_path, setters_path, out_path, names_path = map(pathlib.Path, sys.argv[1:7])

exe = exe_path.read_bytes()
pe = struct.unpack_from("<I", exe, 0x3C)[0]
sections = []
for index in range(struct.unpack_from("<H", exe, pe + 6)[0]):
    at = pe + 24 + struct.unpack_from("<H", exe, pe + 20)[0] + index * 40
    size, address, raw_size, raw = struct.unpack_from("<4I", exe, at + 8)
    sections.append((0x400000 + address, raw_size, raw))


def pointer(address):
    for start, size, raw in sections:
        if start <= address < start + size - 3:
            return struct.unpack_from("<I", exe, raw + address - start)[0]
    return None


def read(name):
    path = corpus / f"{name}.c"
    return path.read_text(errors="ignore") if path.exists() else ""


CALL = re.compile(r"\b(FUN_[0-9a-f]{8})\(")
REGISTER = re.compile(r"FUN_0054f140\((0x[0-9a-f]+|\d+),(FUN_[0-9a-f]{8})\)")
VTABLE = re.compile(r"= &PTR_(?:FUN|LAB)_([0-9a-f]{8});")
RAISE = re.compile(r"\b(FUN_0053f(?:950|9c0|bd0|a60|ab0|b00))\((0x[0-9a-f]+|\d+)[,)]")
TIMER_FIRE = 0x005862A0
ARMS = {"FUN_0053fa60", "FUN_0053fab0", "FUN_0053fb00"}
HELPERS = {"FUN_004fd450", "FUN_004fd4d0", "FUN_00619730", "FUN_004ece30", "FUN_004ece40"}

events = {int(a, 0): factory for a, factory in REGISTER.findall(read("FUN_0051ef80"))}

# Who raises or arms each id.
senders = collections.defaultdict(set)
for path in corpus.glob("FUN_*.c"):
    if path.stem == "FUN_0051ef80":
        continue
    for helper, value in RAISE.findall(path.read_text(errors="ignore")):
        if int(value, 0) in events:
            senders[int(value, 0)].add((path.stem, helper))

# Class vtables and the names of their notifiers; setters by their own slot.
vtable_names = collections.defaultdict(set)
for row in csv.DictReader(slots_path.open(), delimiter="\t"):
    if row["vtable"]:
        vtable_names[int(row["vtable"], 16)].add(row["name"])
setters = collections.defaultdict(list)
for row in csv.DictReader(setters_path.open(), delimiter="\t"):
    if row["kind"] == "setter" and row["name"] and row["master_slot"]:
        setters[int(row["master_slot"], 16)].append(row)


def raised_by_setter(sender):
    """Setters whose own notifier slot holds `sender` in a class vtable."""
    address = int(sender[4:], 16)
    found = set()
    for vtable, names in vtable_names.items():
        for slot in range(0, 0x400, 4):
            if pointer(vtable + slot) != address:
                continue
            for row in setters.get(slot, ()):
                if row["notifier"] in names:
                    found.add(row["name"])
    return found


rows = []
names = []
for event_id in sorted(events):
    factory = events[event_id]
    constructors = [name for name in CALL.findall(read(factory)) if name not in {"FUN_00618b70"} | HELPERS and name != factory]
    constructor = constructors[-1] if constructors else ""
    vtables = VTABLE.findall(read(constructor))
    vtable = int(vtables[-1], 16) if vtables else None
    fire = pointer(vtable + 0x20) if vtable else None
    kind = "event"
    if fire == TIMER_FIRE:
        kind, fire = "timer", pointer(vtable + 0x30)
    fire_name = f"FUN_{fire:08x}" if fire else ""
    handlers = [name for name in CALL.findall(read(fire_name)) if name not in HELPERS and name != fire_name]
    # The handler is the fire function's last call; a lone call is certain.
    handler = handlers[-1] if handlers else ""
    certain = len(set(handlers)) == 1
    raised = sorted(senders.get(event_id, ()))
    setter_names = sorted({name for sender, _ in raised for name in raised_by_setter(sender)})
    raising = sorted({
        name
        for sender, helper in raised
        if helper not in ARMS
        for name in raised_by_setter(sender)
    })
    label = ""
    if kind == "event" and len(raising) == 1:
        label = raising[0].replace("_Set", "").replace("Set", "", 1) + "Changed"
    rows.append((
        f"0x{event_id:x}", kind, label, factory[4:], constructor[4:],
        f"{vtable:08x}" if vtable else "", fire_name[4:], handler[4:],
        "yes" if certain else "",
        " ".join(f"{sender[4:]}:{helper[4:]}" for sender, helper in raised),
        " ".join(setter_names),
    ))
    tag = ("Timer" if kind == "timer" else "Event") + f"{event_id:03X}" + (f"_{label}" if label else "")
    roles = [(factory, "Create"), (constructor, "Init"), (fire_name, "Expire" if kind == "timer" else "Fire")]
    if certain:
        roles.append((handler, "Handle"))
    for address, role in roles:
        if address and read(address):
            names.append((address[4:], f"{tag}_{role}"))

with out_path.open("w", newline="") as handle:
    writer = csv.writer(handle, delimiter="\t", lineterminator="\n")
    writer.writerow(["id", "kind", "name", "factory", "constructor", "vtable", "fire", "handler", "handler_certain", "senders", "sender_setters"])
    writer.writerows(rows)

# A function shared by several events keeps no single name.
counts = collections.Counter(address for address, _ in names)
with names_path.open("w", newline="") as handle:
    writer = csv.writer(handle, delimiter="\t", lineterminator="\n")
    writer.writerow(["function", "name"])
    writer.writerows(sorted((a, n) for a, n in names if counts[a] == 1))

print(f"events: {len(rows)}")
print(f"  timers: {sum(1 for row in rows if row[1] == 'timer')}")
for column, title in ((5, "vtable"), (6, "fire"), (7, "handler"), (8, "certain handler"), (9, "senders"), (2, "named")):
    print(f"  with {title}: {sum(1 for row in rows if row[column])}")
print(f"  names to apply: {sum(1 for a, _ in names if counts[a] == 1)}")
