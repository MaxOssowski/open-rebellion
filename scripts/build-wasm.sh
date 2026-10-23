#!/usr/bin/env bash
# Build Open Rebellion for wasm32-unknown-unknown and stage artifacts in web/.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TARGET_DIR="$ROOT/target/wasm32-unknown-unknown/release"
GDATA="$ROOT/data/base"
WEB_DATA="$ROOT/web/data/base"
WEB_AUDIO="$ROOT/web/data/sounds"
MDATA_DIR="${REBELLION_MDATA_DIR:-$ROOT/../star-wars-rebellion/MDATA}"
ORIGINAL_GAME_DIR="${REBELLION_GAME_DIR:-$(dirname "$MDATA_DIR")}"
EDATA_DIR="${REBELLION_EDATA_DIR:-$ORIGINAL_GAME_DIR/EData}"
# Audio already staged for the native build by tools/stage-ui-assets.
STAGED_AUDIO="$ROOT/data/sounds"

# Print the directory holding an original DLL: the original installation, or
# the GData directory when the installation's DLLs were copied beside the DATs.
original_dll_dir() {
    if [ -f "$ORIGINAL_GAME_DIR/$1" ]; then
        echo "$ORIGINAL_GAME_DIR"
    elif [ -f "$GDATA/$1" ]; then
        echo "$GDATA"
    fi
}

# Print the source of a soundtrack cue: the original MDATA file, or the copy
# tools/stage-ui-assets staged byte for byte under data/sounds/music.
music_source() {
    if [ -f "$MDATA_DIR/$1" ]; then
        echo "$MDATA_DIR/$1"
    elif [ -f "$STAGED_AUDIO/music/$2" ]; then
        echo "$STAGED_AUDIO/music/$2"
    fi
}

# Refuse stale UI staging before compilation; the runtime pack builder repeats this gate.
python3 "$ROOT/scripts/build-runtime-pack.py" --ui "$GDATA/ui" --validate-ui-only

echo "Building rebellion-app for wasm32…"
PATH="/usr/bin:$PATH" cargo build --manifest-path "$ROOT/Cargo.toml" \
    --target wasm32-unknown-unknown \
    -p rebellion-app \
    --release

# The binary name may be rebellion-app or open-rebellion depending on the build
WASM_SRC="$TARGET_DIR/open-rebellion.wasm"
if [ ! -f "$WASM_SRC" ]; then
    WASM_SRC="$TARGET_DIR/rebellion-app.wasm"
fi
cp "$WASM_SRC" "$ROOT/web/open-rebellion.wasm"
echo "Copied open-rebellion.wasm → web/"

# ── wasm-opt: shrink and optimize final artifact ──────────────────────────
# Recovers the size overhead from web-sys/wasm-bindgen etc.
# Install via `brew install binaryen` or the binaryen release binaries.
if command -v wasm-opt >/dev/null 2>&1; then
    BYTES_BEFORE=$(wc -c < "$ROOT/web/open-rebellion.wasm" | tr -d ' ')
    # Feature flags for modern Rust-generated WASM. Rust's LLVM backend
    # emits these by default since ~1.60. Not every packaged wasm-opt build
    # supports every flag (e.g. Debian's apt binaryen lacks
    # --enable-bulk-memory-opt) — write to a staging path and only replace
    # the working .wasm on success, so an unsupported-flag failure degrades
    # to "ship unoptimized" instead of aborting the whole build.
    if wasm-opt -O3 --strip-debug \
        --enable-nontrapping-float-to-int \
        --enable-bulk-memory \
        --enable-bulk-memory-opt \
        --enable-mutable-globals \
        --enable-sign-ext \
        --enable-reference-types \
        --enable-multivalue \
        -o "$ROOT/web/open-rebellion.wasm.opt" \
        "$ROOT/web/open-rebellion.wasm"; then
        mv "$ROOT/web/open-rebellion.wasm.opt" "$ROOT/web/open-rebellion.wasm"
        BYTES_AFTER=$(wc -c < "$ROOT/web/open-rebellion.wasm" | tr -d ' ')
        SAVED=$((BYTES_BEFORE - BYTES_AFTER))
        PCT=$(( (SAVED * 100) / BYTES_BEFORE ))
        echo "wasm-opt -O3: ${BYTES_BEFORE} → ${BYTES_AFTER} bytes (saved ${SAVED}, ${PCT}%)"
    else
        rm -f "$ROOT/web/open-rebellion.wasm.opt"
        echo "WARNING: wasm-opt failed (this binaryen build may not support all requested flags) — keeping unoptimized .wasm (${BYTES_BEFORE} bytes)."
    fi
else
    echo "WARNING: wasm-opt not found. Install binaryen for a smaller release build."
fi

# gl.js comes from macroquad/miniquad. Must be vendored in repo.
if [ ! -f "$ROOT/web/gl.js" ]; then
    echo "ERROR: web/gl.js not found. It should be committed in the repo."
    exit 1
fi
echo "gl.js present (vendored)."

# ── Copy DAT files for WASM HTTP fetch ──────────────────────────────────
echo "Copying DAT files to web/data/base/…"
mkdir -p "$WEB_DATA"
cp "$GDATA"/*.DAT "$WEB_DATA/" 2>/dev/null || true
# Copy DLL too (for string lookup if native-style loading is ever ported)
cp "$GDATA"/*.DLL "$WEB_DATA/" 2>/dev/null || true

# Extract TEXTSTRA strings to JSON for WASM (pelite can't target WASM)
echo "Extracting TEXTSTRA.DLL strings to textstra.json…"
DAT_DUMPER="${ROOT}/target/release/dat-dumper"
echo "Building dat-dumper for resource extraction…"
PATH="/usr/bin:$PATH" cargo build --manifest-path "$ROOT/Cargo.toml" -p dat-dumper --release
if [ -f "$GDATA/TEXTSTRA.DLL" ]; then
    "$DAT_DUMPER" --gdata "$GDATA" --extract-strings --output "$WEB_DATA"
else
    echo "{}" > "$WEB_DATA/textstra.json"
    echo "{}" > "$WEB_DATA/textstra-messages.json"
    echo "{}" > "$WEB_DATA/gid-legends.json"
    echo "WARNING: TEXTSTRA.DLL not found. Entity names will use fallback format."
fi

# The Encyclopedia's topic source (texts and artwork names), extracted by
# stage-ui-assets; the original window falls back to the replacement without it.
if [ -f "$GDATA/encyclopedia/source.json" ] && [ -f "$GDATA/encyclopedia/source.json.manifest.json" ]; then
    cp "$GDATA/encyclopedia/source.json" "$WEB_DATA/encyclopedia-source.json"
    cp "$GDATA/encyclopedia/source.json.manifest.json" "$WEB_DATA/encyclopedia-source.manifest.json"
else
    echo "WARNING: data/base/encyclopedia/source.json not found; the original Encyclopedia stays unavailable in the browser."
fi

DAT_COUNT=$(ls -1 "$WEB_DATA"/*.DAT 2>/dev/null | wc -l | tr -d ' ')
echo "Staged $DAT_COUNT DAT files + textstra.json in web/data/base/"

# Stage licensed music without adding it to version control.
# REBELLION_MDATA_DIR may point at an original installation's MDATA directory.
mkdir -p "$WEB_AUDIO/music"
MAIN_THEME_SRC="$(music_source MDATA.300 main_theme.wav)"
if [ -n "$MAIN_THEME_SRC" ]; then
    cp "$MAIN_THEME_SRC" "$WEB_AUDIO/music/main_theme.wav"
    echo "Staged MDATA.300 (Return of the Jedi/Battle of Endor cue) as the shuttle-cockpit main theme from $MAIN_THEME_SRC."
else
    rm -f "$WEB_AUDIO/music/main_theme.wav"
    echo "WARNING: MDATA.300 not found in $MDATA_DIR or $STAGED_AUDIO/music; the menu will remain silent."
fi
BATTLE_THEME_SRC="$(music_source MDATA.307 battle.wav)"
if [ -n "$BATTLE_THEME_SRC" ]; then
    cp "$BATTLE_THEME_SRC" "$WEB_AUDIO/music/battle.wav"
    echo "Staged MDATA.307 as the tactical battle score from $BATTLE_THEME_SRC."
else
    rm -f "$WEB_AUDIO/music/battle.wav"
    echo "WARNING: MDATA.307 not found in $MDATA_DIR or $STAGED_AUDIO/music; tactical battles will remain silent."
fi
# The battle windows' original score (REBEXE table 0x006a8478, tracks 10..15):
# the Battle Alert plays MDATA.307 or 313, the Battle Summary 308, 309, 314 or 315.
for MDATA_NUMBER in 307 313 308 309 314 315; do
    SCORE_SRC="$(music_source "MDATA.$MDATA_NUMBER" "$MDATA_NUMBER.wav")"
    if [ -n "$SCORE_SRC" ]; then
        cp "$SCORE_SRC" "$WEB_AUDIO/music/$MDATA_NUMBER.wav"
    else
        rm -f "$WEB_AUDIO/music/$MDATA_NUMBER.wav"
        echo "WARNING: MDATA.$MDATA_NUMBER not found; that battle window will remain silent."
    fi
done
echo "Staged the battle windows' original score (MDATA.307..315)."

mkdir -p "$WEB_AUDIO/sfx"
COMMON_DLL_DIR="$(original_dll_dir COMMON.DLL)"
if [ -n "$COMMON_DLL_DIR" ]; then
    "$DAT_DUMPER" --gdata "$COMMON_DLL_DIR" --extract-menu-sfx --output "$WEB_AUDIO/sfx"
else
    rm -f "$WEB_AUDIO/sfx/menu_galaxy_size.wav" \
        "$WEB_AUDIO/sfx/menu_load_options.wav" \
        "$WEB_AUDIO/sfx/menu_quit.wav" \
        "$WEB_AUDIO/sfx/menu_select.wav"
    echo "WARNING: COMMON.DLL not found in $ORIGINAL_GAME_DIR or $GDATA; cockpit SFX will remain silent."
fi
TACTICAL_DLL_DIR="$(original_dll_dir TACTICAL.DLL)"
if [ -n "$TACTICAL_DLL_DIR" ]; then
    rm -f "$WEB_AUDIO/sfx/tactical_ship_destroyed.wav"
    "$DAT_DUMPER" --gdata "$TACTICAL_DLL_DIR" --extract-tactical-sfx --output "$WEB_AUDIO/sfx"
else
    rm -f "$WEB_AUDIO/sfx/tactical_ship_destroyed.wav" \
        "$WEB_AUDIO/sfx/tactical_event_0d_0.wav" \
        "$WEB_AUDIO/sfx/tactical_event_0d_1.wav" \
        "$WEB_AUDIO/sfx/tactical_event_0d_2.wav" \
        "$WEB_AUDIO/sfx/tactical_event_0e_0.wav" \
        "$WEB_AUDIO/sfx/tactical_event_0e_1.wav" \
        "$WEB_AUDIO/sfx/tactical_event_0e_2.wav" \
        "$WEB_AUDIO/sfx/tactical_event_0f_0.wav" \
        "$WEB_AUDIO/sfx/tactical_event_0f_1.wav" \
        "$WEB_AUDIO/sfx/tactical_event_0f_2.wav" \
        "$WEB_AUDIO/sfx/tactical_event_10_0.wav" \
        "$WEB_AUDIO/sfx/tactical_event_10_1.wav" \
        "$WEB_AUDIO/sfx/tactical_event_10_2.wav" \
        "$WEB_AUDIO/sfx/tactical_event_11_0.wav" \
        "$WEB_AUDIO/sfx/tactical_event_11_1.wav" \
        "$WEB_AUDIO/sfx/tactical_event_11_2.wav" \
        "$WEB_AUDIO/sfx/tactical_event_12_0.wav" \
        "$WEB_AUDIO/sfx/tactical_event_12_1.wav" \
        "$WEB_AUDIO/sfx/tactical_event_12_2.wav" \
        "$WEB_AUDIO/sfx/tactical_event_13_0.wav" \
        "$WEB_AUDIO/sfx/tactical_event_13_1.wav" \
        "$WEB_AUDIO/sfx/tactical_event_13_2.wav" \
        "$WEB_AUDIO/sfx/tactical_event_14_0.wav"
    echo "WARNING: TACTICAL.DLL not found in $ORIGINAL_GAME_DIR or $GDATA; tactical event cues will remain silent."
fi

mkdir -p "$WEB_AUDIO/voice/alliance" "$WEB_AUDIO/voice/empire"
rm -f "$WEB_AUDIO/voice/alliance"/*.wav "$WEB_AUDIO/voice/empire"/*.wav
VOICE_DLL_DIR="$(original_dll_dir VOICEFXA.DLL)"
if [ -n "$VOICE_DLL_DIR" ] && [ -f "$VOICE_DLL_DIR/VOICEFXE.DLL" ]; then
    "$DAT_DUMPER" --gdata "$VOICE_DLL_DIR" --extract-tactical-voice \
        --output "$WEB_AUDIO/voice"
else
    echo "WARNING: VOICEFXA.DLL or VOICEFXE.DLL not found in $ORIGINAL_GAME_DIR or $GDATA; tactical voices will remain silent."
fi

# ── Stage UI resources into web/data/ui/ ────────────────────────────────────
UI_SRC="$ROOT/data/base/ui"
WEB_UI="$ROOT/web/data/ui"
if [ -d "$UI_SRC" ]; then
    echo "Copying staged UI resources to web/data/ui/…"
    mkdir -p "$WEB_UI"
    cp -r "$UI_SRC"/. "$WEB_UI/"
    UI_COUNT=$(find "$WEB_UI" -name "*.bmp" 2>/dev/null | wc -l | tr -d ' ')
    echo "Staged $UI_COUNT UI BMPs in web/data/ui/"
    ADVISOR_FRAME_COUNT=$(find "$WEB_UI" -path "*/TYPE302/*.bin" 2>/dev/null | wc -l | tr -d ' ')
    echo "Staged $ADVISOR_FRAME_COUNT advisor frames in web/data/ui/"

    # Generate BMP manifest for WASM pre-fetch (HTTP can't enumerate dirs)
    echo "Generating BMP manifest for WASM…"
    MANIFEST="$WEB_UI/bmp-manifest.json"
    python3 -c "
import json, pathlib, sys
root = pathlib.Path('$WEB_UI')
entries, skipped = [], 0
for dll_dir in sorted(root.iterdir()):
    if not dll_dir.is_dir():
        continue
    bmp_dir = dll_dir / 'BMP'
    if not bmp_dir.is_dir():
        continue
    dll_name = dll_dir.name
    for f in sorted(bmp_dir.glob('*.bmp')):
        try:
            entries.append({'dll': dll_name, 'id': int(f.stem)})
        except ValueError:
            skipped += 1
            print(f'  WARNING: skipping non-numeric BMP: {f}', file=sys.stderr)
if skipped:
    print(f'  Skipped {skipped} non-numeric BMP files', file=sys.stderr)
with open('$MANIFEST', 'w') as fh:
    json.dump(entries, fh, separators=(',', ':'))
print(f'  {len(entries)} entries in bmp-manifest.json')
if not entries:
    print('  WARNING: manifest is empty — no BMPs found', file=sys.stderr)
"
else
    echo "ERROR: data/base/ui/ not found — run 'go run ./tools/stage-ui-assets' first."
    exit 1
fi

# ── Build deterministic single-request runtime pack ─────────────────────────
# The WASM client prefers this self-describing pack and retains the loose-file
# loader only as a development fallback. This removes thousands of serial HTTP
# requests without changing the resource keys consumed by BmpCache.
echo "Building browser runtime asset pack…"
RUNTIME_PACK_ARGS=(
    --base "$WEB_DATA"
    --ui "$WEB_UI"
    --audio "$WEB_AUDIO"
    --output "$ROOT/web/data/runtime.orpk"
)
if [ -d "$EDATA_DIR" ]; then
    RUNTIME_PACK_ARGS+=(--edata "$EDATA_DIR")
    echo "Including original encyclopedia artwork from $EDATA_DIR."
else
    echo "WARNING: EData not found at $EDATA_DIR; encyclopedia artwork will remain unavailable."
fi
python3 "$ROOT/scripts/build-runtime-pack.py" "${RUNTIME_PACK_ARGS[@]}"

WASM_SIZE=$(du -h "$ROOT/web/open-rebellion.wasm" | cut -f1)
echo "Done. WASM size: $WASM_SIZE"
echo "Serve web/ with any HTTP server, e.g.: python3 -m http.server 8080 -d web/"
