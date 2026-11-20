#!/usr/bin/env python3
"""Fail-closed native/WASM replay checkpoint equivalence gate."""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import time
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
WEB = ROOT / "web"
ARTIFACT = ROOT / "crates/rebellion-data/tests/fixtures/replay_seed42_v1.json"
DEFAULT_PORT = 18082
EXPECTED_REQUEST_SUFFIXES = ("/", "/gl.js", "/open-rebellion.wasm", "/data/runtime.orpk")
# Ordinary startup then fetches the intro movies it plays before the menu.
INTRO_MOVIE_PATHS = ("/data/video/000.webm", "/data/video/001.webm")
EQUIVALENCE_FIELDS = (
    "schema_version",
    "fixture_id",
    "status",
    "artifact_text",
    "engine_version",
    "seed",
    "data_input_count",
    "data_bytes",
    "data_fingerprint",
    "initial_fingerprint",
    "observed_checkpoints",
    "final_tick",
    "final_fingerprint",
    "failure_phase",
    "error",
)


def run(command: list[str], *, timeout: int = 300) -> subprocess.CompletedProcess[str]:
    environment = dict(os.environ)
    environment.update(
        {
            "CC": "/usr/bin/cc",
            "CXX": "/usr/bin/c++",
            "CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER": "/usr/bin/cc",
        }
    )
    return subprocess.run(
        command,
        cwd=ROOT,
        env=environment,
        capture_output=True,
        text=True,
        timeout=timeout,
        check=False,
    )


def parse_json_line(output: str) -> dict:
    for line in reversed(output.splitlines()):
        if line.startswith("{"):
            return json.loads(line)
    raise ValueError(f"command emitted no JSON object: {output[-500:]}")


def native_report() -> dict:
    result = run(
        [
            "cargo",
            "run",
            "-q",
            "-p",
            "rebellion-data",
            "--bin",
            "replay-gate",
            "--",
            "data/base",
        ]
    )
    if result.returncode != 0:
        raise RuntimeError(f"native replay gate failed:\n{result.stderr}\n{result.stdout}")
    return parse_json_line(result.stdout)


def browser_report(
    port: int,
    *,
    scenario: str,
    query: str,
    expect_report: bool,
    abort_runtime_pack: bool = False,
) -> dict:
    page_name = f"open-rebellion-replay-{scenario}-{os.getpid()}"
    base = ["agent-browser", "--session", page_name, "--json"]

    def agent(*command: str, timeout: int = 35) -> dict:
        result = subprocess.run(
            [*base, *command],
            cwd=ROOT,
            capture_output=True,
            text=True,
            timeout=timeout,
            check=False,
        )
        if result.returncode != 0:
            raise RuntimeError(
                f"browser command failed ({' '.join(command)}):\n"
                f"{result.stderr}\n{result.stdout}"
            )
        payload = parse_json_line(result.stdout)
        if not payload.get("success"):
            raise RuntimeError(f"browser command reported failure: {payload}")
        return payload.get("data", {})

    try:
        if abort_runtime_pack:
            agent("network", "route", "**/data/runtime.orpk", "--abort")
        agent(
            "open",
            f"http://127.0.0.1:{port}/{query}",
        )
        attempts = 200 if expect_report else 150
        for _ in range(attempts):
            # Some agent-browser Chrome builds instantiate WASM but suppress
            # requestAnimationFrame in headless sessions. Drive the exported
            # macroquad frame only until replay output or the menu is ready.
            evaluated = agent(
                "eval",
                "(() => {"
                "const report = window.__openRebellionReplay || null;"
                "const menuActive = document.getElementById('main-menu-semantics')?.dataset.active || null;"
                "let framePumped = false;"
                "if (report === null && menuActive !== 'true' "
                "&& typeof wasm_exports !== 'undefined' "
                "&& typeof wasm_exports.frame === 'function') {"
                "wasm_exports.frame();"
                "framePumped = true;"
                "}"
                "return JSON.stringify({"
                "report: window.__openRebellionReplay || null,"
                "menuActive: document.getElementById('main-menu-semantics')?.dataset.active || null,"
                "canvasWidth: document.getElementById('glcanvas')?.width || 0,"
                "canvasHeight: document.getElementById('glcanvas')?.height || 0,"
                "video: Boolean(document.querySelector('video')),"
                "framePumped"
                "});"
                "})()",
            )
            page_state = json.loads(evaluated.get("result", "{}"))
            if expect_report and page_state.get("report") is not None:
                break
            if not expect_report and page_state.get("menuActive") == "true":
                break
            if not expect_report and page_state.get("video"):
                # Ordinary startup plays the intro movies first (MDATA.000,
                # 001; FUN_004138a0); Escape skips each one, as the
                # interface-parity gates do (skipToMainMenu).
                agent("press", "Escape")
            time.sleep(0.1)
        if expect_report and page_state.get("report") is None:
            raise RuntimeError(
                "browser replay gate did not report within 20 seconds: "
                f"{json.dumps(page_state, sort_keys=True)}"
            )

        errors = agent("errors").get("errors", [])
        request_records = agent("network", "requests").get("requests", [])
        return {
            "report": page_state.get("report"),
            "page_state": page_state,
            "errors": errors,
            "requests": [record.get("url", "") for record in request_records],
            "responses": [
                {"url": record.get("url", ""), "status": record.get("status", 0)}
                for record in request_records
            ],
        }
    finally:
        subprocess.run(
            [*base, "close"],
            cwd=ROOT,
            capture_output=True,
            text=True,
            timeout=10,
            check=False,
        )


def request_paths(browser: dict, port: int) -> list[str]:
    paths = []
    for url in browser.get("requests", []):
        path = url.split("?", 1)[0]
        paths.append(path.removeprefix(f"http://127.0.0.1:{port}"))
    return paths


def verify(
    native: dict,
    browser: dict,
    invalid_query: dict,
    missing_pack: dict,
    normal_startup: dict,
    port: int,
) -> dict:
    failures: list[str] = []
    browser_gate = browser.get("report")
    if not isinstance(browser_gate, dict):
        failures.append("browser did not publish window.__openRebellionReplay")
        browser_gate = {}

    artifact_text = ARTIFACT.read_text(encoding="utf-8")
    if native.get("artifact_text") != artifact_text:
        failures.append("native runner did not execute the exact committed artifact text")
    if browser_gate.get("artifact_text") != artifact_text:
        failures.append("WASM runner did not execute the exact committed artifact text")
    for field in EQUIVALENCE_FIELDS:
        if native.get(field) != browser_gate.get(field):
            failures.append(f"native/WASM mismatch in {field}")

    errors = browser.get("errors", [])
    if errors:
        failures.append(f"browser emitted errors: {errors}")

    requests = browser.get("requests", [])
    replay_paths = request_paths(browser, port)
    if tuple(replay_paths) != EXPECTED_REQUEST_SUFFIXES:
        failures.append(f"expected four replay-gate requests, saw {replay_paths}")
    failed_responses = [
        response for response in browser.get("responses", []) if response.get("status", 0) >= 400
    ]
    if failed_responses:
        failures.append(f"browser responses failed: {failed_responses}")

    invalid_report = invalid_query.get("report") or {}
    if invalid_report.get("status") != "failed" or invalid_report.get("failure_phase") != "query":
        failures.append("invalid replay query did not fail closed in the query phase")
    if request_paths(invalid_query, port) != ["/", "/gl.js", "/open-rebellion.wasm"]:
        failures.append("invalid replay query fell through to runtime-pack or production loading")
    if (
        invalid_report.get("initial_fingerprint") is not None
        or invalid_report.get("observed_checkpoints")
        or invalid_report.get("final_fingerprint") is not None
    ):
        failures.append("invalid replay query retained partial replay state")
    if invalid_query.get("errors"):
        failures.append(f"invalid replay query emitted browser errors: {invalid_query['errors']}")

    missing_report = missing_pack.get("report") or {}
    if (
        missing_report.get("status") != "failed"
        or missing_report.get("failure_phase") != "runtime_pack_fetch"
    ):
        failures.append("missing runtime pack did not produce a structured fetch failure")
    if (
        missing_report.get("initial_fingerprint") is not None
        or missing_report.get("observed_checkpoints")
        or missing_report.get("final_fingerprint") is not None
    ):
        failures.append("missing runtime pack retained partial replay state")
    if missing_pack.get("errors"):
        failures.append(f"missing runtime pack emitted browser errors: {missing_pack['errors']}")

    normal_state = normal_startup.get("page_state", {})
    if normal_state.get("report") is not None:
        failures.append("normal startup unexpectedly executed the replay gate")
    if normal_state.get("menuActive") != "true":
        failures.append("normal startup did not reach the semantic main menu")
    if normal_state.get("canvasWidth", 0) <= 0 or normal_state.get("canvasHeight", 0) <= 0:
        failures.append("normal startup canvas is blank-sized")
    normal_paths = request_paths(normal_startup, port)
    if (
        tuple(normal_paths[: len(EXPECTED_REQUEST_SUFFIXES)]) != EXPECTED_REQUEST_SUFFIXES
        or any(
            path not in INTRO_MOVIE_PATHS
            for path in normal_paths[len(EXPECTED_REQUEST_SUFFIXES) :]
        )
    ):
        failures.append(
            f"expected four normal-startup requests and the intro movies, saw {normal_paths}"
        )
    if normal_startup.get("errors"):
        failures.append(f"normal startup emitted browser errors: {normal_startup['errors']}")

    return {
        "status": "passed" if not failures else "failed",
        "native_platform": native.get("platform"),
        "browser_platform": browser_gate.get("platform"),
        "artifact_bytes": len(artifact_text.encode("utf-8")),
        "checkpoint_count": len(native.get("observed_checkpoints", [])),
        "initial_fingerprint": native.get("initial_fingerprint"),
        "final_tick": native.get("final_tick"),
        "final_fingerprint": native.get("final_fingerprint"),
        "request_count": len(requests),
        "browser_errors": len(errors),
        "invalid_query": invalid_report.get("failure_phase"),
        "missing_pack": missing_report.get("failure_phase"),
        "normal_startup_requests": len(normal_startup.get("requests", [])),
        "normal_menu_active": normal_state.get("menuActive"),
        "failures": failures,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--skip-build", action="store_true")
    parser.add_argument("--port", type=int, default=DEFAULT_PORT)
    parser.add_argument("--json", action="store_true")
    parser.add_argument(
        "--site",
        type=Path,
        default=WEB,
        help="directory to serve (default web/); with --skip-build, a staged copy",
    )
    args = parser.parse_args()

    if not args.skip_build:
        built = run(["bash", "scripts/build-wasm.sh"])
        if built.returncode != 0:
            print(built.stderr, file=sys.stderr)
            return 1

    server = subprocess.Popen(
        [sys.executable, "-m", "http.server", str(args.port), "--bind", "127.0.0.1"],
        cwd=args.site,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
        text=True,
    )
    try:
        time.sleep(1)
        if server.poll() is not None:
            detail = server.stderr.read() if server.stderr is not None else ""
            raise RuntimeError(f"local replay-gate server failed to start: {detail}")
        native = native_report()
        browser = browser_report(
            args.port,
            scenario="success",
            query=f"?replay-check=seed42-v1&gate-run={os.getpid()}",
            expect_report=True,
        )
        invalid_query = browser_report(
            args.port,
            scenario="invalid-query",
            query="?replay-check=unknown",
            expect_report=True,
        )
        missing_pack = browser_report(
            args.port,
            scenario="missing-pack",
            query="?replay-check=seed42-v1",
            expect_report=True,
            abort_runtime_pack=True,
        )
        normal_startup = browser_report(
            args.port,
            scenario="normal",
            query="",
            expect_report=False,
        )
        result = verify(
            native,
            browser,
            invalid_query,
            missing_pack,
            normal_startup,
            args.port,
        )
    finally:
        if server.poll() is None:
            server.terminate()
            server.wait(timeout=5)

    if args.json:
        print(json.dumps(result, indent=2))
    else:
        print(
            f"Replay equivalence: {result['status'].upper()} | "
            f"{result['checkpoint_count']} checkpoints | "
            f"{result['request_count']} requests | "
            f"{result['browser_errors']} browser errors"
        )
        for failure in result["failures"]:
            print(f"- {failure}")
    return 0 if result["status"] == "passed" else 1


if __name__ == "__main__":
    raise SystemExit(main())
