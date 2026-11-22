#!/usr/bin/env python3
"""Measure code-quality metrics for the cleanup loop (agent_docs/cleanup-loop.md).

Usage:
    python3 scripts/cleanup-metrics.py [--out target/cleanup/metrics-<stamp>.json]
                                       [--skip-clippy] [--skip-wasm]

Writes a JSON report and prints a short summary plus the top candidates for
the next cleanup iteration. Optional tools are used when installed and
skipped otherwise: cargo-machete (unused dependencies), cargo-llvm-cov
(unreached code). Nothing here changes the tree.
"""
import argparse
import collections
import datetime
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CRATES = ["crates", "tools"]
ENV = dict(os.environ, PATH="/usr/bin:/bin:" + os.path.expanduser("~/.cargo/bin") + ":" + os.environ.get("PATH", ""))
FN_RE = re.compile(r"^\s*(pub(\([^)]*\))?\s+)?(const\s+)?(async\s+)?(unsafe\s+)?fn\s+([A-Za-z0-9_]+)")
WINDOW = 12  # lines per duplicate-detection window


def rust_files():
    for base in CRATES:
        for dirpath, _, files in os.walk(os.path.join(ROOT, base)):
            # Relative to the root, so a worktree under target/ still counts.
            if "target" in os.path.relpath(dirpath, ROOT).split(os.sep):
                continue
            for name in files:
                if name.endswith(".rs"):
                    yield os.path.join(dirpath, name)


def is_test_path(path):
    return path.endswith("_tests.rs") or "/tests/" in path


def scan_functions(path, lines):
    """Rough brace-matched function lengths; good enough to rank candidates."""
    out = []
    i = 0
    while i < len(lines):
        m = FN_RE.match(lines[i])
        if m:
            depth = 0
            started = False
            j = i
            while j < len(lines):
                depth += lines[j].count("{") - lines[j].count("}")
                if "{" in lines[j]:
                    started = True
                if started and depth <= 0:
                    break
                if not started and lines[j].rstrip().endswith(";"):
                    break
                j += 1
            if started:
                out.append({"file": os.path.relpath(path, ROOT), "line": i + 1,
                            "name": m.group(6), "lines": j - i + 1})
        i += 1
    return out


def normalise(line):
    line = line.strip()
    if not line or line.startswith("//"):
        return None
    return re.sub(r"\s+", " ", line)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out")
    ap.add_argument("--skip-clippy", action="store_true")
    ap.add_argument("--skip-wasm", action="store_true")
    args = ap.parse_args()
    stamp = datetime.datetime.now().strftime("%Y-%m-%dT%H-%M")
    out = args.out or os.path.join(ROOT, "target", "cleanup", f"metrics-{stamp}.json")
    os.makedirs(os.path.dirname(out), exist_ok=True)

    files = []
    functions = []
    windows = collections.defaultdict(list)
    todo_tags = collections.Counter()
    for path in rust_files():
        with open(path, encoding="utf-8", errors="replace") as fh:
            lines = fh.read().splitlines()
        rel = os.path.relpath(path, ROOT)
        files.append({"file": rel, "lines": len(lines), "test": is_test_path(path)})
        if not is_test_path(path):
            functions.extend(scan_functions(path, lines))
            norm = [(n, normalise(l)) for n, l in enumerate(lines, 1)]
            norm = [(n, l) for n, l in norm if l]
            for k in range(len(norm) - WINDOW + 1):
                chunk = "\n".join(l for _, l in norm[k:k + WINDOW])
                if len(chunk) < 300:
                    continue
                windows[hashlib.sha1(chunk.encode()).hexdigest()].append((rel, norm[k][0]))
        for l in lines:
            for tag in ("port:", "hyp:", "TODO", "FIXME", "#[allow(dead_code)]"):
                if tag in l:
                    todo_tags[tag] += 1

    duplicates = []
    seen = set()
    for places in windows.values():
        uniq = sorted(set(places))
        if len(uniq) > 1:
            key = tuple(f for f, _ in uniq)
            if key in seen:
                continue
            seen.add(key)
            duplicates.append(uniq)

    report = {
        "stamp": stamp,
        "commit": subprocess.run(["git", "rev-parse", "--short", "HEAD"], cwd=ROOT,
                                 capture_output=True, text=True).stdout.strip(),
        "rust_lines_total": sum(f["lines"] for f in files),
        "rust_lines_non_test": sum(f["lines"] for f in files if not f["test"]),
        "largest_files": sorted((f for f in files if not f["test"]), key=lambda f: -f["lines"])[:15],
        "longest_functions": sorted(functions, key=lambda f: -f["lines"])[:25],
        "functions_over_150_lines": sum(1 for f in functions if f["lines"] > 150),
        "duplicate_windows": len(duplicates),
        "duplicate_examples": duplicates[:15],
        "tags": dict(todo_tags),
    }

    if not args.skip_clippy:
        r = subprocess.run(["cargo", "clippy", "--workspace", "--all-targets", "--message-format=short",
                            "--", "-W", "dead_code"], cwd=ROOT, env=ENV, capture_output=True, text=True)
        warn = [l for l in r.stderr.splitlines() if ": warning:" in l]
        kinds = collections.Counter(re.sub(r".*warning: ", "", l).split("`")[0].strip()[:60] for l in warn)
        report["clippy_warnings"] = len(warn)
        report["clippy_top_kinds"] = kinds.most_common(15)
        report["dead_code_warnings"] = sum(1 for l in warn if "never used" in l or "never read" in l
                                           or "never constructed" in l)

    if shutil.which("cargo-machete", path=ENV["PATH"]):
        r = subprocess.run(["cargo", "machete"], cwd=ROOT, env=ENV, capture_output=True, text=True)
        report["unused_dependencies"] = r.stdout.strip().splitlines()
    else:
        report["unused_dependencies"] = "skipped (cargo-machete not installed)"
    report["coverage"] = ("available: run cargo llvm-cov --workspace --summary-only"
                          if shutil.which("cargo-llvm-cov", path=ENV["PATH"])
                          else "skipped (cargo-llvm-cov not installed)")

    wasm = os.path.join(ROOT, "web", "open-rebellion.wasm")
    if not args.skip_wasm and os.path.exists(wasm):
        report["wasm_bytes"] = os.path.getsize(wasm)

    with open(out, "w") as fh:
        json.dump(report, fh, indent=2)

    print(f"metrics: {os.path.relpath(out, ROOT)} (commit {report['commit']})")
    print(f"rust lines: {report['rust_lines_total']} total, {report['rust_lines_non_test']} outside tests")
    print(f"functions over 150 lines: {report['functions_over_150_lines']}")
    print(f"duplicate {WINDOW}-line windows: {report['duplicate_windows']}")
    if "clippy_warnings" in report:
        print(f"clippy warnings: {report['clippy_warnings']} (dead code: {report['dead_code_warnings']})")
    if "wasm_bytes" in report:
        print(f"wasm: {report['wasm_bytes']} bytes")
    print("tags:", report["tags"])
    print("\nlargest files:")
    for f in report["largest_files"][:5]:
        print(f"  {f['lines']:6}  {f['file']}")
    print("longest functions:")
    for f in report["longest_functions"][:8]:
        print(f"  {f['lines']:6}  {f['file']}:{f['line']} {f['name']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
