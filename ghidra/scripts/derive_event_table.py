#!/usr/bin/env python3
"""Build the event table: id, factory, class vtable, fire slot, handler,
and the code that raises or arms the event.

FUN_0051ef80 registers a factory per event id (FUN_0054f140). A factory
allocates the event and calls its constructor, which installs the class
vtable; slot +0x20 is the fire function. A timer class keeps the shared
fire function FUN_005862a0 there and its own expiry function in slot +0x30.
The fire function opens a 28-byte context (FUN_004fd450) and hands it to
the handler.

An event is raised through FUN_0053f950 (FUN_0053f9c0, FUN_0053fbd0) with
its id, or by building the event class directly and posting it; a timer is
armed through FUN_0053fa60 and its period set through FUN_0053fab0
(FUN_0053fb00). FUN_0053fe40 is the notify helper and raises nothing.

Senders are found three ways: in the decompiled corpus, by scanning the
code for an id pushed before a call to a raise helper (which also finds
the small vtable stubs Ghidra leaves undefined), and as callers of an
event's constructor other than its factory.

A sender that sits in a class vtable at the slot a named setter calls is
that setter's change notifier, which names the event.

Usage: derive_event_table.py DECOMPILED_DIR REBEXE.EXE notifier-slots.tsv
       setters.tsv function-ranges.tsv OUT.tsv NAMES.tsv
"""
import bisect
import collections
import csv
import pathlib
import re
import struct
import sys

corpus, exe_path, slots_path, setters_path, ranges_path, out_path, names_path = map(
    pathlib.Path, sys.argv[1:8]
)

exe = exe_path.read_bytes()
pe = struct.unpack_from("<I", exe, 0x3C)[0]
sections = []
for index in range(struct.unpack_from("<H", exe, pe + 6)[0]):
    at = pe + 24 + struct.unpack_from("<H", exe, pe + 20)[0] + index * 40
    size, address, raw_size, raw = struct.unpack_from("<4I", exe, at + 8)
    sections.append((0x400000 + address, raw_size, raw))
text_start, text_size, text_raw = sections[0]
text = exe[text_raw : text_raw + text_size]


def pointer(address):
    for start, size, raw in sections:
        if start <= address < start + size - 3:
            return struct.unpack_from("<I", exe, raw + address - start)[0]
    return None


def read(name):
    path = corpus / f"{name}.c"
    return path.read_text(errors="ignore") if path.exists() else ""


# Function bodies as Ghidra has them (DumpFunctionRanges.java).
ranges = sorted(
    (int(row["first"], 16), int(row["last"], 16), int(row["function"], 16))
    for row in csv.DictReader(ranges_path.open(), delimiter="\t")
)
range_starts = [first for first, _, _ in ranges]

CALL = re.compile(r"\b(FUN_[0-9a-f]{8})\(")
CALL_ARGS = re.compile(r"\b(FUN_[0-9a-f]{8})\(([^;]*)\);")
REGISTER = re.compile(r"FUN_0054f140\((0x[0-9a-f]+|\d+),(FUN_[0-9a-f]{8})\)")
VTABLE = re.compile(r"= &PTR_(?:FUN|LAB)_([0-9a-f]{8});")
CONTEXT = re.compile(r"undefined1 (\w+) \[28\];")
RAISES = {0x53F950: "raise", 0x53F9C0: "raise", 0x53FBD0: "raise"}
ARMS = {0x53FA60: "arm", 0x53FAB0: "period", 0x53FB00: "period"}
HELPERS = {"FUN_004fd450", "FUN_004fd4d0", "FUN_00619730", "FUN_004ece30", "FUN_004ece40"}
TIMER_FIRE = 0x005862A0
CLASS_SPAN = 0x400

events = {int(a, 0): factory for a, factory in REGISTER.findall(read("FUN_0051ef80"))}


def stub_start(address):
    """The 16-byte aligned start of an undefined stub holding `address`."""
    at = address & ~0xF
    for _ in range(32):
        before = text[at - text_start - 1]
        if before in (0x90, 0xCC, 0xC3) or text[at - text_start - 3] == 0xC2:
            return at
        at -= 0x10
    return address & ~0xF


def owner(address):
    """The function or stub containing `address`, and whether it is a stub."""
    index = bisect.bisect_right(range_starts, address) - 1
    if index >= 0 and ranges[index][0] <= address <= ranges[index][1]:
        return ranges[index][2], False
    return stub_start(address), True


# Senders by scanning the code: `push id` shortly before a helper call.
senders = collections.defaultdict(set)  # id -> {(address, verb, is_stub)}
for match in re.finditer(rb"\xe8", text):
    at = match.start()
    if at + 5 > len(text):
        continue
    site = text_start + at
    target = (site + 5 + struct.unpack_from("<i", text, at + 1)[0]) & 0xFFFFFFFF
    verb = RAISES.get(target) or ARMS.get(target)
    if verb is None:
        continue
    window = text[max(0, at - 24) : at]
    found = None
    for push in re.finditer(rb"\x68(....)", window, re.S):
        value = struct.unpack("<I", push.group(1))[0]
        if value in events:
            found = value
    if found is not None:
        address, is_stub = owner(site)
        senders[found].add((address, verb, is_stub))

# Class vtables and the names of their notifiers; setters by their own slot.
vtable_names = collections.defaultdict(set)
for row in csv.DictReader(slots_path.open(), delimiter="\t"):
    if row["vtable"]:
        vtable_names[int(row["vtable"], 16)].add(row["name"])
setters = collections.defaultdict(list)
for row in csv.DictReader(setters_path.open(), delimiter="\t"):
    if row["kind"] == "setter" and row["name"] and row["master_slot"]:
        setters[int(row["master_slot"], 16)].append(row)


def raised_by_setter(address):
    """Setters whose own notifier slot holds `address` in a class vtable."""
    found = set()
    for vtable, names in vtable_names.items():
        for slot in range(0, 0x400, 4):
            if pointer(vtable + slot) != address:
                continue
            for row in setters.get(slot, ()):
                if row["notifier"] in names:
                    found.add(row["name"])
    return found


# Callers of each function, for constructors posted without an id.
callers = collections.defaultdict(set)
for path in corpus.glob("FUN_*.c"):
    for name in set(CALL.findall(path.read_text(errors="ignore"))):
        callers[name].add(path.stem)

rows = []
names = []
for event_id in sorted(events):
    factory = events[event_id]
    constructors = [
        name
        for name in CALL.findall(read(factory))
        if name not in {"FUN_00618b70"} | HELPERS and name != factory
    ]
    constructor = constructors[-1] if constructors else ""
    vtables = VTABLE.findall(read(constructor))
    vtable = int(vtables[-1], 16) if vtables else None
    fire = pointer(vtable + 0x20) if vtable else None
    kind = "event"
    if fire == TIMER_FIRE:
        kind, fire = "timer", pointer(vtable + 0x30)
    fire_name = f"FUN_{fire:08x}" if fire else ""
    body = read(fire_name)

    # The handler takes the fire function's 28-byte context.
    context = CONTEXT.findall(body)
    handlers = []
    if len(context) == 1:
        handlers = [
            name
            for name, arguments in CALL_ARGS.findall(body)
            if name not in HELPERS and re.search(rf"\b{context[0]}\b", arguments)
        ]
    certain = len(set(handlers)) == 1
    if not handlers:
        handlers = [name for name in CALL.findall(body) if name not in HELPERS and name != fire_name]
    handler = handlers[-1] if handlers else ""

    found = set(senders.get(event_id, ()))
    # The class's own code (its other constructors, its copy and load
    # routines) sits beside the factory; a sender lies elsewhere.
    home = int(factory[4:], 16)
    for caller in callers.get(constructor, ()):
        address = int(caller[4:], 16)
        if caller != factory and abs(address - home) >= CLASS_SPAN:
            found.add((address, "build", False))
    found = sorted(found)
    setter_names = sorted({name for address, _, _ in found for name in raised_by_setter(address)})
    raising = sorted({
        name
        for address, verb, _ in found
        if verb in ("raise", "build")
        for name in raised_by_setter(address)
    })
    label = ""
    if kind == "event" and len(raising) == 1:
        label = raising[0].replace("_Set", "").replace("Set", "", 1) + "Changed"
    rows.append((
        f"0x{event_id:x}", kind, label, factory[4:], constructor[4:],
        f"{vtable:08x}" if vtable else "", fire_name[4:], handler[4:],
        "yes" if certain else "",
        " ".join(f"{address:08x}:{verb}{'*' if is_stub else ''}" for address, verb, is_stub in found),
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
    writer.writerow([
        "id", "kind", "name", "factory", "constructor", "vtable", "fire",
        "handler", "handler_certain", "senders", "sender_setters",
    ])
    writer.writerows(rows)

# A function shared by several events keeps no single name.
counts = collections.Counter(address for address, _ in names)
with names_path.open("w", newline="") as handle:
    writer = csv.writer(handle, delimiter="\t", lineterminator="\n")
    writer.writerow(["function", "name"])
    writer.writerows(sorted((a, n) for a, n in names if counts[a] == 1))

print(f"events: {len(rows)}")
print(f"  timers: {sum(1 for row in rows if row[1] == 'timer')}")
for column, title in (
    (5, "vtable"), (6, "fire"), (7, "handler"), (8, "certain handler"),
    (9, "senders"), (2, "named"),
):
    print(f"  with {title}: {sum(1 for row in rows if row[column])}")
print(f"  names to apply: {sum(1 for a, _ in names if counts[a] == 1)}")
print("  no sender:", " ".join(row[0] for row in rows if not row[9]))
print("  handler not certain:", " ".join(row[0] for row in rows if not row[8]))
