#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11"
# dependencies = ["pefile"]
# ///
"""Tests for the cursor staging in extract-dll-resources.py.

Run with: uv run scripts/test_extract_dll_resources.py
"""

from __future__ import annotations

import importlib.util
import struct
import sys
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("extract-dll-resources.py")
SPEC = importlib.util.spec_from_file_location("extract_dll_resources", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
EXTRACT = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = EXTRACT
SPEC.loader.exec_module(EXTRACT)


def cursor(width: int, height: int, xor_rows: list[list[int]], and_rows: list[list[int]],
           hotspot: tuple[int, int] = (12, 12)) -> bytes:
    """Build an 8-bit RT_CURSOR resource. Rows are listed top-down."""
    palette = [(0, 0, 0), (255, 255, 255), (10, 20, 30)] + [(0, 0, 0)] * 253
    xor_stride = (width * 8 + 31) // 32 * 4
    and_stride = (width + 31) // 32 * 4
    data = struct.pack('<HH', *hotspot)
    data += struct.pack('<IiiHHIIiiII', 40, width, height * 2, 1, 8, 0, 0, 0, 0, 256, 0)
    data += b''.join(struct.pack('<BBBB', b, g, r, 0) for r, g, b in palette)
    for row in reversed(xor_rows):
        data += bytes(row).ljust(xor_stride, b'\0')
    for row in reversed(and_rows):
        bits = bytearray(and_stride)
        for x, masked in enumerate(row):
            bits[x // 8] |= masked << (7 - x % 8)
        data += bytes(bits)
    return data


def pixel(bmp: bytes, x: int, y: int) -> tuple[int, int, int]:
    """Read a top-down (x, y) pixel from a bottom-up 24-bit BMP as RGB."""
    offset = struct.unpack_from('<I', bmp, 10)[0]
    width, height = struct.unpack_from('<ii', bmp, 18)
    stride = (width * 3 + 3) & ~3
    at = offset + (height - 1 - y) * stride + x * 3
    blue, green, red = bmp[at:at + 3]
    return red, green, blue


class CursorStagingTests(unittest.TestCase):
    def test_masked_pixels_become_the_key_and_unmasked_keep_their_color(self) -> None:
        xor = [[1, 0], [2, 0]]
        mask = [[0, 1], [0, 1]]
        bmp, hotspot = EXTRACT.cursor_to_bmp(cursor(2, 2, xor, mask))

        self.assertEqual(hotspot, (12, 12))
        self.assertEqual(struct.unpack_from('<iiHH', bmp, 18), (2, 2, 1, 24))
        self.assertEqual(pixel(bmp, 0, 0), (255, 255, 255))
        self.assertEqual(pixel(bmp, 0, 1), (10, 20, 30))
        self.assertEqual(pixel(bmp, 1, 0), EXTRACT.CURSOR_KEY)
        self.assertEqual(pixel(bmp, 1, 1), EXTRACT.CURSOR_KEY)

    def test_a_pixel_that_inverts_the_screen_is_rejected(self) -> None:
        with self.assertRaisesRegex(ValueError, r"\(1, 0\) inverts"):
            EXTRACT.cursor_to_bmp(cursor(2, 1, [[0, 1]], [[0, 1]]))

    def test_a_truncated_cursor_is_rejected(self) -> None:
        with self.assertRaisesRegex(ValueError, "truncated"):
            EXTRACT.cursor_to_bmp(cursor(2, 2, [[1, 0], [1, 0]], [[0, 1], [0, 1]])[:-4])

    def test_a_cursor_group_resolves_to_its_deepest_image(self) -> None:
        # REBEXE.EXE group 1002 lists an 8-bit image (id 4) and a 1-bit image (id 5).
        group = struct.pack('<HHH', 0, 2, 2)
        group += struct.pack('<HHHHIH', 32, 64, 1, 8, 2216, 4)
        group += struct.pack('<HHHHIH', 32, 64, 1, 1, 308, 5)
        self.assertEqual(EXTRACT.group_cursor_image(group), 4)

    def test_an_icon_group_is_not_a_cursor_group(self) -> None:
        with self.assertRaisesRegex(ValueError, "not a cursor group"):
            EXTRACT.group_cursor_image(struct.pack('<HHH', 0, 1, 0))


if __name__ == "__main__":
    unittest.main()
