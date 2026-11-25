//! REBEXE.EXE's cursors as the 24-bit keyed BMPs the port draws
//! (`scripts/extract-dll-resources.py --cursors`, read by `BmpCache` as
//! `rebexe-exe/<group>`).
//!
//! port: the staging is the port's own; the pixels are the cursors'.

use std::collections::BTreeMap;

use super::pack::{BuildError, Entry, InstallFiles, KIND_BITMAP};
use super::pe::{self, ResId};

const RT_CURSOR: u32 = 1;
const RT_GROUP_CURSOR: u32 = 12;
/// Masked pixels become this colour key (blue).
const KEY: [u8; 3] = [0, 0, 255];

fn bad(why: String) -> BuildError {
    BuildError::BadBitmap(why)
}

/// The RT_CURSOR id a colour display loads from a cursor group: the entry
/// with the most bits per pixel. hyp: LookupIconIdFromDirectoryEx picks the
/// deepest image on a display of 8 bits or more.
fn group_cursor_image(group: &[u8]) -> Option<u32> {
    let word = |at: usize| group.get(at..at + 2).map(|b| u16::from_le_bytes([b[0], b[1]]));
    if word(2)? != 2 || word(4)? == 0 {
        return None;
    }
    let mut best: Option<(u16, u16)> = None;
    for i in 0..usize::from(word(4)?) {
        let at = 6 + 14 * i;
        let (bits, id) = (word(at + 6)?, word(at + 12)?);
        // Python's max keeps the first of equal keys.
        if best.is_none_or(|(b, _)| bits > b) {
            best = Some((bits, id));
        }
    }
    best.map(|(_, id)| u32::from(id))
}

/// An RT_CURSOR resource (hotspot words, then a DIB covering the XOR image
/// and the AND mask) as a bottom-up 24-bit BMP with masked pixels keyed.
fn cursor_to_bmp(cursor: &[u8]) -> Result<Vec<u8>, BuildError> {
    let dib = cursor.get(4..).ok_or_else(|| bad("cursor is truncated".into()))?;
    let u32_at = |at: usize| dib.get(at..at + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]));
    let header = u32_at(0).ok_or_else(|| bad("cursor DIB is truncated".into()))? as usize;
    let width = u32_at(4).map(|v| v as i32).unwrap_or(0);
    let double_height = u32_at(8).map(|v| v as i32).unwrap_or(0);
    let bits = dib.get(14..16).map_or(0, |b| u16::from_le_bytes([b[0], b[1]]));
    let colors_used = u32_at(32).unwrap_or(0) as usize;
    if ![1, 4, 8].contains(&bits) || width <= 0 || double_height <= 0 || double_height % 2 != 0 {
        return Err(bad(format!("unsupported cursor DIB {width}x{double_height} at {bits} bpp")));
    }
    let (width, height, bits) = (width as usize, (double_height / 2) as usize, usize::from(bits));
    let colors = if colors_used == 0 { 1 << bits } else { colors_used };
    let xor = header + 4 * colors;
    let xor_stride = (width * bits).div_ceil(32) * 4;
    let and = xor + xor_stride * height;
    let and_stride = width.div_ceil(32) * 4;
    if dib.len() < and + and_stride * height {
        return Err(bad("cursor DIB is truncated".into()));
    }
    let out_stride = (width * 3 + 3) & !3;
    let mut pixels = vec![0u8; out_stride * height];
    for row in 0..height {
        for x in 0..width {
            let bit = x * bits;
            let byte = dib[xor + row * xor_stride + bit / 8];
            let index = usize::from((byte >> (8 - bits - bit % 8)) & ((1u16 << bits) - 1) as u8);
            let masked = dib[and + row * and_stride + x / 8] >> (7 - x % 8) & 1 == 1;
            let entry = header + 4 * index;
            let (blue, green, red) = (dib[entry], dib[entry + 1], dib[entry + 2]);
            if masked && (red, green, blue) != (0, 0, 0) {
                return Err(bad(format!("cursor pixel ({x}, {}) inverts the screen", height - 1 - row)));
            }
            let [red, green, blue] = if masked { KEY } else { [red, green, blue] };
            let at = row * out_stride + x * 3;
            pixels[at..at + 3].copy_from_slice(&[blue, green, red]);
        }
    }
    let offset = 14u32 + 40;
    let mut bmp = Vec::with_capacity(offset as usize + pixels.len());
    bmp.extend_from_slice(b"BM");
    bmp.extend_from_slice(&(offset + pixels.len() as u32).to_le_bytes());
    bmp.extend_from_slice(&[0; 4]);
    bmp.extend_from_slice(&offset.to_le_bytes());
    for value in [40u32, width as u32, height as u32] {
        bmp.extend_from_slice(&value.to_le_bytes());
    }
    bmp.extend_from_slice(&1u16.to_le_bytes());
    bmp.extend_from_slice(&24u16.to_le_bytes());
    for value in [0u32, pixels.len() as u32, 0, 0, 0, 0] {
        bmp.extend_from_slice(&value.to_le_bytes());
    }
    bmp.extend_from_slice(&pixels);
    Ok(bmp)
}

/// Each numbered resource of `kind`, its first language's bytes.
fn first_language<'a>(resources: &[pe::Resource<'a>], kind: u32) -> BTreeMap<u32, &'a [u8]> {
    let mut out = BTreeMap::new();
    for resource in resources.iter().filter(|r| r.kind == ResId::Id(kind)) {
        if let ResId::Id(id) = resource.name {
            out.entry(id).or_insert(resource.data);
        }
    }
    out
}

/// REBEXE.EXE's cursor groups as `rebexe-exe/<group>` bitmaps.
///
/// # Errors
/// A missing or unreadable REBEXE.EXE, or a cursor the port cannot key.
pub fn cursor_entries(install: &InstallFiles, out: &mut Vec<Entry>) -> Result<(), BuildError> {
    let exe = install.get("REBEXE.EXE").ok_or_else(|| BuildError::Missing("REBEXE.EXE".into()))?;
    let resources = pe::resources(exe).map_err(|error| BuildError::Pe { file: "REBEXE.EXE".into(), error })?;
    let cursors = first_language(&resources, RT_CURSOR);
    for (group_id, group) in first_language(&resources, RT_GROUP_CURSOR) {
        let image = group_cursor_image(group).ok_or_else(|| bad(format!("REBEXE.EXE cursor group {group_id} is not one")))?;
        let cursor = cursors.get(&image).ok_or_else(|| bad(format!("REBEXE.EXE cursor {image} is missing")))?;
        out.push(Entry { kind: KIND_BITMAP, key: format!("rebexe-exe/{group_id}"), data: cursor_to_bmp(cursor)? });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A cursor resource: hotspot words, a `bits`-deep DIB `width` wide and
    /// `rows` high (its header says twice that), `palette` (BGR0 entries),
    /// then `xor` and `and` image bytes.
    fn cursor(width: u32, rows: u32, bits: u16, palette: &[[u8; 4]], xor: &[u8], and: &[u8]) -> Vec<u8> {
        let mut out = vec![0u8; 4];
        let mut dib = vec![0u8; 40];
        dib[0..4].copy_from_slice(&40u32.to_le_bytes());
        dib[4..8].copy_from_slice(&width.to_le_bytes());
        dib[8..12].copy_from_slice(&(rows * 2).to_le_bytes());
        dib[14..16].copy_from_slice(&bits.to_le_bytes());
        dib[32..36].copy_from_slice(&(palette.len() as u32).to_le_bytes());
        out.extend(dib);
        out.extend(palette.iter().flatten());
        out.extend_from_slice(xor);
        out.extend_from_slice(and);
        out
    }

    #[test]
    fn each_cursor_pixel_takes_its_palette_entry_row_by_row() {
        // 4 bpp, 2x2: indices 1, 2 on the bottom row, 3, 1 on the top.
        let palette = [[0, 0, 0, 0], [10, 20, 30, 0], [40, 50, 60, 0], [70, 80, 90, 0]];
        let xor = [0x12, 0, 0, 0, 0x31, 0, 0, 0];
        let bmp = cursor_to_bmp(&cursor(2, 2, 4, &palette, &xor, &[0; 8])).unwrap();
        // 24-bit rows padded to 8 bytes, bottom row first, BGR.
        assert_eq!(&bmp[54..], &[10, 20, 30, 40, 50, 60, 0, 0, 70, 80, 90, 10, 20, 30, 0, 0]);
        assert_eq!(u32::from_le_bytes(bmp[18..22].try_into().unwrap()), 2);
        assert_eq!(u32::from_le_bytes(bmp[22..26].try_into().unwrap()), 2);
    }

    #[test]
    fn a_cursor_that_is_not_a_whole_one_bit_four_or_eight_bit_image_is_refused() {
        let palette = [[0, 0, 0, 0], [255, 255, 255, 0]];
        let whole = cursor(1, 1, 1, &palette, &[0; 4], &[0; 4]);
        assert!(cursor_to_bmp(&whole).is_ok());
        assert!(cursor_to_bmp(&whole[..whole.len() - 1]).is_err(), "one byte short");
        // Two rows need both AND mask rows, 4 bytes each.
        let tall = cursor(1, 2, 1, &palette, &[0; 8], &[0; 8]);
        assert!(cursor_to_bmp(&tall).is_ok());
        assert!(cursor_to_bmp(&tall[..tall.len() - 4]).is_err(), "a mask row short");
        assert!(cursor_to_bmp(&cursor(1, 1, 24, &palette, &[0; 4], &[0; 4])).is_err(), "24 bpp");
        assert!(cursor_to_bmp(&cursor(0, 1, 1, &palette, &[0; 4], &[0; 4])).is_err(), "no width");
        assert!(cursor_to_bmp(&cursor(1, 0, 1, &palette, &[0; 4], &[0; 4])).is_err(), "no height");
        let mut odd = whole.clone();
        odd[12..16].copy_from_slice(&3u32.to_le_bytes());
        assert!(cursor_to_bmp(&odd).is_err(), "an odd double height");
    }

    /// A cursor group: type 2, then (bits, id) entries of 14 bytes.
    fn group(kind: u16, entries: &[(u16, u16)]) -> Vec<u8> {
        let mut out = [0u16, kind, entries.len() as u16].iter().flat_map(|w| w.to_le_bytes()).collect::<Vec<_>>();
        for &(bits, id) in entries {
            let mut entry = [0u8; 14];
            entry[6..8].copy_from_slice(&bits.to_le_bytes());
            entry[12..14].copy_from_slice(&id.to_le_bytes());
            out.extend(entry);
        }
        out
    }

    #[test]
    fn a_cursor_group_names_its_deepest_image_the_first_of_equals() {
        assert_eq!(group_cursor_image(&group(2, &[(1, 5), (8, 6), (4, 7)])), Some(6));
        assert_eq!(group_cursor_image(&group(2, &[(8, 5), (8, 6)])), Some(5));
        assert_eq!(group_cursor_image(&group(1, &[(1, 5)])), None, "an icon group");
        assert_eq!(group_cursor_image(&group(2, &[])), None, "no image");
    }

    #[test]
    fn a_masked_black_cursor_pixel_becomes_the_key_and_an_inverting_one_is_refused() {
        // A 1x1 1-bit cursor: palette black, white; XOR row then AND row.
        let mut cursor = vec![0u8; 4];
        let mut dib = vec![0u8; 40];
        dib[0..4].copy_from_slice(&40u32.to_le_bytes());
        dib[4..8].copy_from_slice(&1u32.to_le_bytes());
        dib[8..12].copy_from_slice(&2u32.to_le_bytes());
        dib[14..16].copy_from_slice(&1u16.to_le_bytes());
        dib.extend_from_slice(&[0, 0, 0, 0, 255, 255, 255, 0]);
        dib.extend_from_slice(&[0x00, 0, 0, 0]); // XOR: index 0 (black)
        dib.extend_from_slice(&[0x80, 0, 0, 0]); // AND: masked
        cursor.extend_from_slice(&dib);
        let bmp = cursor_to_bmp(&cursor).unwrap();
        assert_eq!(&bmp[54..57], &[255, 0, 0], "blue key in BGR order");
        let xor = 4 + 48;
        cursor[xor] = 0x80; // index 1 (white) under the mask
        assert!(cursor_to_bmp(&cursor).is_err());
    }
}
