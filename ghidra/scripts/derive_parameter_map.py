#!/usr/bin/env python3
"""Map every GNPRTB and SDPRTB parameter to the global it loads into and
the functions that read that global.

FUN_0053e390(id, &global) loads a GNPRTB entry (table 0x3c) and
FUN_0053e3e0(id, side, &global) an SDPRTB entry for side 1 or 2 (table
0x3d). Readers are found by scanning the code for the global's absolute
address and attributing each site through function-ranges.tsv; a site in a
stub Ghidra leaves undefined is marked `*`. A reader no longer than
ACCESSOR_BYTES only returns the value, so its callers are listed as the
users.

Usage: derive_parameter_map.py DECOMPILED_DIR REBEXE.EXE DATA_DIR
       function-ranges.tsv PORT_CRATES_DIR OUT.tsv LABELS.tsv NAMES.tsv
"""
import bisect
import collections
import csv
import pathlib
import re
import struct
import sys

corpus, exe_path, data_dir, ranges_path, crates, out_path, labels_path, names_path = map(
    pathlib.Path, sys.argv[1:9]
)

exe = exe_path.read_bytes()
pe = struct.unpack_from("<I", exe, 0x3C)[0]
optional = struct.unpack_from("<H", exe, pe + 20)[0]
_, text_address, text_size, text_raw = struct.unpack_from("<4I", exe, pe + 24 + optional + 8)
text_start = 0x400000 + text_address
text = exe[text_raw : text_raw + text_size]

ranges = sorted(
    (int(row["first"], 16), int(row["last"], 16), int(row["function"], 16))
    for row in csv.DictReader(ranges_path.open(), delimiter="\t")
)
range_starts = [first for first, _, _ in ranges]


def owner(address):
    index = bisect.bisect_right(range_starts, address) - 1
    if index >= 0 and ranges[index][0] <= address <= ranges[index][1]:
        return f"{ranges[index][2]:08x}"
    return f"{address & ~0xF:08x}*"


ACCESSOR_BYTES = 0x20
sizes = collections.defaultdict(int)
for first, last, function in ranges:
    sizes[function] += last - first + 1

# Direct calls: target -> the functions calling it.
callers = collections.defaultdict(set)
for match in re.finditer(rb"\xe8", text):
    at = match.start()
    if at + 5 <= len(text):
        target = (text_start + at + 5 + struct.unpack_from("<i", text, at + 1)[0]) & 0xFFFFFFFF
        if target in sizes:
            callers[target].add(owner(text_start + at))


def table(name, columns):
    """parameter_id -> the distinct values of its record, in column order."""
    data = (data_dir / name).read_bytes()
    _, count, info = struct.unpack_from("<3I", data, 0)
    size = 4 * (3 + columns)
    values = {}
    for index in range(count):
        record = struct.unpack_from(f"<3I{columns}i", data, 12 + info + index * size)
        values[record[2]] = record[3:]
    return values


gnprtb = table("GNPRTB.DAT", 8)
sdprtb = table("SDPRTB.DAT", 16)

N = r"(0x[0-9a-f]+|\d+)"
GN = re.compile(rf"FUN_0053e390\({N},&DAT_([0-9a-f]{{8}})\)")
SD = re.compile(rf"FUN_0053e3e0\({N},([12]),&DAT_([0-9a-f]{{8}})\)")

loads = []  # (file, id, side, global, loader)
for path in sorted(corpus.glob("FUN_*.c")):
    body = path.read_text(errors="ignore")
    for value, address in GN.findall(body):
        loads.append(("GNPRTB", int(value, 0), 0, int(address, 16), path.stem[4:]))
    for value, side, address in SD.findall(body):
        loads.append(("SDPRTB", int(value, 0), int(side), int(address, 16), path.stem[4:]))

# Every code site holding one of the globals' addresses.
wanted = {entry[3] for entry in loads}
sites = collections.defaultdict(set)
for match in re.finditer(rb"[\x00-\xff]{2}\x6b\x00", text):
    value = struct.unpack("<I", match.group())[0]
    if value in wanted:
        sites[value].add(owner(text_start + match.start()))
# Addresses outside 0x006bxxxx, if any.
for value in wanted:
    if value >> 16 != 0x6B:
        for match in re.finditer(re.escape(struct.pack("<I", value)), text):
            sites[value].add(owner(text_start + match.start()))

# Parameter ids the port names beside a gnprtb or sdprtb mention.
port = collections.defaultdict(set)
LITERAL = re.compile(r"\b(0x[0-9a-fA-F_]+|\d[\d_]*)\b")
for path in crates.rglob("*.rs"):
    for line in path.read_text(errors="ignore").splitlines():
        lowered = line.lower()
        for key in ("gnprtb", "sdprtb"):
            if key in lowered:
                for literal in LITERAL.findall(line):
                    try:
                        port[key.upper()].add(int(literal.replace("_", ""), 0))
                    except ValueError:
                        pass

SIDES = {0: "", 1: "Alliance", 2: "Empire"}
accessor_reads = collections.defaultdict(set)
rows = []
labels = []
for file, parameter, side, address, loader in sorted(loads):
    values = (gnprtb if file == "GNPRTB" else sdprtb).get(parameter)
    if values is None:
        shown = "missing"
    elif file == "SDPRTB":
        shown = "/".join(str(v) for v in values[side - 1 :: 2])
    else:
        shown = "/".join(str(v) for v in values)
    if values is not None and len(set(shown.split("/"))) == 1:
        shown = shown.split("/")[0]
    readers = sorted(sites.get(address, set()) - {loader})
    accessors = [
        reader for reader in readers
        if not reader.endswith("*") and sizes[int(reader, 16)] <= ACCESSOR_BYTES
    ]
    users = sorted({user for reader in accessors for user in callers[int(reader, 16)]})
    for reader in accessors:
        accessor_reads[reader].add((file, parameter, side))
    rows.append((
        file, parameter, f"0x{parameter:x}", SIDES[side], f"{address:08x}", loader, shown,
        "yes" if parameter in port[file] else "", len(readers), " ".join(readers),
        " ".join(accessors), len(users), " ".join(users),
    ))
    name = f"{file.capitalize()}_{parameter}" + (f"_{SIDES[side]}" if side else "")
    labels.append((f"{address:08x}", name))

with out_path.open("w", newline="") as handle:
    writer = csv.writer(handle, delimiter="\t", lineterminator="\n")
    writer.writerow([
        "file", "parameter", "hex", "side", "global", "loader", "values",
        "port_uses", "reader_count", "readers", "accessors", "user_count", "users",
    ])
    writer.writerows(rows)

counts = collections.Counter(address for address, _ in labels)
with labels_path.open("w", newline="") as handle:
    writer = csv.writer(handle, delimiter="\t", lineterminator="\n")
    writer.writerow(["address", "label"])
    writer.writerows(sorted((a, n) for a, n in labels if counts[a] == 1))

# An accessor that returns one parameter takes its name.
with names_path.open("w", newline="") as handle:
    writer = csv.writer(handle, delimiter="\t", lineterminator="\n")
    writer.writerow(["function", "name"])
    for reader, reads in sorted(accessor_reads.items()):
        if len(reads) == 1:
            file, parameter, side = next(iter(reads))
            suffix = f"_{SIDES[side]}" if side else ""
            writer.writerow([reader, f"Get{file.capitalize()}_{parameter}{suffix}"])
        elif len({(file, parameter) for file, parameter, _ in reads}) == 1:
            file, parameter, _ = next(iter(reads))
            writer.writerow([reader, f"Get{file.capitalize()}_{parameter}_BySide"])

for file, catalogue in (("GNPRTB", gnprtb), ("SDPRTB", sdprtb)):
    mine = [row for row in rows if row[0] == file]
    loaded = {row[1] for row in mine}
    print(f"{file}: {len(catalogue)} entries, {len(loaded)} loaded, {len(mine)} loads")
    print(f"  not loaded: {sorted(set(catalogue) - loaded)}")
    print(f"  loaded but absent from the file: {sorted(loaded - set(catalogue))}")
    print(f"  never read after loading: {sorted({row[1] for row in mine if row[8] == 0})}")
    used = {row[1] for row in mine if row[7]}
    print(f"  port names {len(used)}; port names with no reader: "
          f"{sorted({row[1] for row in mine if row[7] and row[8] == 0})}")
    print(f"  port ids not in the file: {sorted(i for i in port[file] if i > 255 and i not in catalogue and i < 0x10000)[:20]}")
print(f"globals shared by several loads: {sum(1 for c in counts.values() if c > 1)}")
print(f"accessor functions: {len(accessor_reads)}; "
      f"parameters reached only through an accessor: {sum(1 for row in rows if row[10] and row[8] == len(row[10].split()))}; "
      f"with no user at all: {sum(1 for row in rows if row[10] and row[8] == len(row[10].split()) and row[11] == 0)}")
