//! A Smacker movie decoder for the game's cutscenes (the `SMK2` files
//! `MDATA/MDATA.000`-`.202`), dependency-free so the browser build can play
//! the player's own movies (`docs/plans/2026-11-24-bring-your-own-data.md`).
//!
//! It decodes one frame at a time, so a 49 MB movie is never held decoded,
//! and decodes a whole audio track at once into 16-bit PCM.
//!
//! port: the Smacker format (RAD Game Tools), as FFmpeg reads it:
//! `libavformat/smacker.c` (header, frame chunks, palette changes and the
//! 6-bit to 8-bit colour table) and `libavcodec/smacker.c` (the header
//! trees, the per-frame "last value" caches, the 4x4 block types and the
//! packed DPCM audio). Its output is checked frame by frame and sample by
//! sample against FFmpeg's in the ignored tests below.

use std::fmt;

/// The header's flag for an extra "ring" frame at the end.
const FLAG_RING_FRAME: u32 = 0x01;
/// The header's flags for interlaced and doubled lines.
pub const FLAG_Y_INTERLACE: u32 = 0x02;
pub const FLAG_Y_DOUBLE: u32 = 0x04;
/// A frame's flag: a palette chunk comes first.
const FRAME_PALETTE: u8 = 0x01;
/// The audio flags in a track's rate word (its top byte).
const AUDIO_PACKED: u32 = 0x80;
const AUDIO_16BIT: u32 = 0x20;
const AUDIO_STEREO: u32 = 0x10;
const AUDIO_BINK: u32 = 0x08;
const AUDIO_DCT: u32 = 0x04;
const HEADER_SIZE: usize = 104;
/// Tree recursion limits (FFmpeg's SMKTREE_DECODE_MAX_RECURSION and
/// SMKTREE_DECODE_BIG_MAX_RECURSION).
const SMALL_TREE_DEPTH: usize = 32;
const BIG_TREE_DEPTH: usize = 500;
const NODE: u32 = 0x8000_0000;

/// The block runs a type code's bits 2-7 index (`block_runs`).
const BLOCK_RUNS: [u32; 64] = [
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26,
    27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50,
    51, 52, 53, 54, 55, 56, 57, 58, 59, 128, 256, 512, 1024, 2048,
];

/// A 6-bit palette component as 8 bits (FFmpeg's `smk_pal`).
#[must_use]
pub fn palette_component(six_bits: u8) -> u8 {
    let v = six_bits & 0x3f;
    (v << 2) | (v >> 4)
}

/// Why a movie could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SmackerError {
    NotSmacker,
    Truncated(&'static str),
    BadTree(&'static str),
    BadPalette,
    BadAudio(&'static str),
    UnsupportedAudio(u32),
    NoTrack(usize),
}

impl fmt::Display for SmackerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotSmacker => write!(f, "not a Smacker movie"),
            Self::Truncated(what) => write!(f, "truncated {what}"),
            Self::BadTree(what) => write!(f, "invalid {what} tree"),
            Self::BadPalette => write!(f, "invalid palette chunk"),
            Self::BadAudio(what) => write!(f, "invalid audio: {what}"),
            Self::UnsupportedAudio(flags) => write!(f, "unsupported (Bink) audio, flags {flags:#x}"),
            Self::NoTrack(track) => write!(f, "no audio track {track}"),
        }
    }
}

impl std::error::Error for SmackerError {}

/// A little-endian bit reader: a byte's lowest bit comes first. Past the
/// end it reads zeros, as FFmpeg's padded reader does.
struct Bits<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Bits<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn left(&self) -> isize {
        (self.data.len() * 8) as isize - self.pos as isize
    }

    fn bit(&mut self) -> u32 {
        let byte = self.data.get(self.pos >> 3).copied().unwrap_or(0);
        let bit = u32::from(byte >> (self.pos & 7)) & 1;
        self.pos += 1;
        bit
    }

    /// `n` bits, the first read as the lowest.
    fn bits(&mut self, n: u32) -> u32 {
        let mut value = 0;
        for i in 0..n {
            value |= self.bit() << i;
        }
        value
    }
}

/// A byte tree (`smacker_decode_tree`): leaves hold 8-bit values; a 0 bit
/// goes left, a 1 bit right.
#[derive(Debug, Clone)]
struct ByteTree {
    /// Leaf: value; node: (left, right) as indices.
    nodes: Vec<ByteNode>,
}

#[derive(Debug, Clone, Copy)]
enum ByteNode {
    Leaf(u8),
    Branch(u16, u16),
}

impl ByteTree {
    fn read(bits: &mut Bits<'_>) -> Result<Self, SmackerError> {
        let mut tree = Self { nodes: Vec::new() };
        let mut leaves = 0;
        tree.node(bits, 0, &mut leaves)?;
        Ok(tree)
    }

    fn node(&mut self, bits: &mut Bits<'_>, depth: usize, leaves: &mut usize) -> Result<u16, SmackerError> {
        if depth > SMALL_TREE_DEPTH {
            return Err(SmackerError::BadTree("byte"));
        }
        let index = self.nodes.len() as u16;
        if bits.bit() == 0 {
            if *leaves >= 256 || bits.left() < 8 {
                return Err(SmackerError::BadTree("byte"));
            }
            *leaves += 1;
            self.nodes.push(ByteNode::Leaf(bits.bits(8) as u8));
            return Ok(index);
        }
        self.nodes.push(ByteNode::Branch(0, 0));
        let left = self.node(bits, depth + 1, leaves)?;
        let right = self.node(bits, depth + 1, leaves)?;
        self.nodes[usize::from(index)] = ByteNode::Branch(left, right);
        Ok(index)
    }

    fn decode(&self, bits: &mut Bits<'_>) -> u8 {
        let mut at = 0usize;
        loop {
            match self.nodes[at] {
                ByteNode::Leaf(value) => return value,
                ByteNode::Branch(left, right) => {
                    at = usize::from(if bits.bit() == 0 { left } else { right });
                }
            }
        }
    }
}

/// An optional byte tree: absent ones give a fixed value (0) with no bits.
fn byte_value(tree: &Option<ByteTree>, bits: &mut Bits<'_>) -> u32 {
    tree.as_ref().map_or(0, |t| u32::from(t.decode(bits)))
}

/// A 16-bit tree (`smacker_decode_bigtree`) in FFmpeg's flat layout: a
/// node is `NODE | size of its left subtree`, followed by the left subtree
/// and then the right; three leaves (the escapes) cache the last three
/// values decoded and are reset to 0 at each frame.
#[derive(Debug, Clone)]
struct BigTree {
    values: Vec<u32>,
    last: [usize; 3],
}

impl BigTree {
    /// A missing tree: always 0, its caches in a dummy slot.
    fn absent() -> Self {
        Self { values: vec![0, 0], last: [1, 1, 1] }
    }

    fn read(bits: &mut Bits<'_>, size: u32) -> Result<Self, SmackerError> {
        if size >= u32::MAX >> 4 {
            return Err(SmackerError::BadTree("header"));
        }
        let mut halves = [None, None];
        for half in &mut halves {
            if bits.bit() == 0 {
                continue;
            }
            *half = Some(ByteTree::read(bits)?);
            bits.bit();
        }
        let escapes = [bits.bits(16), bits.bits(16), bits.bits(16)];
        let capacity = ((size + 3) >> 2) as usize;
        let mut builder = BigBuilder {
            halves,
            escapes,
            last: [None; 3],
            values: Vec::with_capacity(capacity + 3),
            capacity,
        };
        builder.node(bits, 0)?;
        bits.bit();
        let mut values = builder.values;
        let mut last = [0; 3];
        for (slot, found) in last.iter_mut().zip(builder.last) {
            *slot = found.unwrap_or_else(|| {
                values.push(0);
                values.len() - 1
            });
        }
        Ok(Self { values, last })
    }

    fn reset(&mut self) {
        for at in self.last {
            self.values[at] = 0;
        }
    }

    /// `smk_get_code`.
    fn decode(&mut self, bits: &mut Bits<'_>) -> u32 {
        let mut at = 0;
        while self.values[at] & NODE != 0 {
            if bits.bit() == 1 {
                at += (self.values[at] & !NODE) as usize;
            }
            at += 1;
        }
        let value = self.values[at];
        let [l0, l1, l2] = self.last;
        if value != self.values[l0] {
            self.values[l2] = self.values[l1];
            self.values[l1] = self.values[l0];
            self.values[l0] = value;
        }
        value
    }
}

struct BigBuilder {
    halves: [Option<ByteTree>; 2],
    escapes: [u32; 3],
    last: [Option<usize>; 3],
    values: Vec<u32>,
    capacity: usize,
}

impl BigBuilder {
    /// Returns the number of entries the subtree takes.
    fn node(&mut self, bits: &mut Bits<'_>, depth: usize) -> Result<u32, SmackerError> {
        if depth > BIG_TREE_DEPTH || self.values.len() >= self.capacity || bits.left() <= 0 {
            return Err(SmackerError::BadTree("big"));
        }
        if bits.bit() == 0 {
            let low = byte_value(&self.halves[0], bits);
            let high = byte_value(&self.halves[1], bits);
            let mut value = low | (high << 8);
            if let Some(i) = self.escapes.iter().position(|&e| e == value) {
                self.last[i] = Some(self.values.len());
                value = 0;
            }
            self.values.push(value);
            return Ok(1);
        }
        let at = self.values.len();
        self.values.push(0);
        let left = self.node(bits, depth + 1)?;
        self.values[at] = NODE | left;
        let right = self.node(bits, depth + 1)?;
        Ok(left + 1 + right)
    }
}

/// An audio track the header declares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioTrack {
    pub sample_rate: u32,
    pub channels: u16,
    pub bits: u16,
    pub packed: bool,
}

/// Decoded PCM: interleaved 16-bit samples.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pcm {
    pub sample_rate: u32,
    pub channels: u16,
    pub samples: Vec<i16>,
}

impl Pcm {
    /// A RIFF WAV file (16-bit PCM) of these samples.
    #[must_use]
    pub fn to_wav(&self) -> Vec<u8> {
        let data_len = (self.samples.len() * 2) as u32;
        let block = u32::from(self.channels) * 2;
        let mut out = Vec::with_capacity(44 + data_len as usize);
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(36 + data_len).to_le_bytes());
        out.extend_from_slice(b"WAVEfmt ");
        out.extend_from_slice(&16u32.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&self.channels.to_le_bytes());
        out.extend_from_slice(&self.sample_rate.to_le_bytes());
        out.extend_from_slice(&(self.sample_rate * block).to_le_bytes());
        out.extend_from_slice(&(block as u16).to_le_bytes());
        out.extend_from_slice(&16u16.to_le_bytes());
        out.extend_from_slice(b"data");
        out.extend_from_slice(&data_len.to_le_bytes());
        for sample in &self.samples {
            out.extend_from_slice(&sample.to_le_bytes());
        }
        out
    }
}

/// A decoded frame: palette indices and the palette (RGB triples).
#[derive(Debug, Clone, Copy)]
pub struct Frame<'s> {
    pub width: usize,
    pub height: usize,
    pub indices: &'s [u8],
    pub palette: &'s [u8; 768],
}

impl Frame<'_> {
    /// The frame as RGBA at its stored size, as FFmpeg outputs it.
    #[must_use]
    pub fn to_rgba(&self) -> Vec<u8> {
        let mut out = Vec::new();
        self.write_rgba(&mut out);
        out
    }

    fn write_rgba(&self, out: &mut Vec<u8>) {
        out.clear();
        out.reserve(self.indices.len() * 4);
        for &index in self.indices {
            let at = usize::from(index) * 3;
            out.extend_from_slice(&[self.palette[at], self.palette[at + 1], self.palette[at + 2], 255]);
        }
    }

    /// The frame as shown: doubled lines repeat, interlaced lines leave a
    /// black line between (the header's flags; FFmpeg ignores them).
    #[must_use]
    pub fn to_display_rgba(&self, flags: u32) -> (usize, usize, Vec<u8>) {
        let rgba = self.to_rgba();
        if flags & (FLAG_Y_DOUBLE | FLAG_Y_INTERLACE) == 0 {
            return (self.width, self.height, rgba);
        }
        let row = self.width * 4;
        let mut out = Vec::with_capacity(rgba.len() * 2);
        for line in rgba.chunks_exact(row) {
            out.extend_from_slice(line);
            if flags & FLAG_Y_DOUBLE != 0 {
                out.extend_from_slice(line);
            } else {
                out.extend(std::iter::repeat_n([0, 0, 0, 255], self.width).flatten());
            }
        }
        (self.width, self.height * 2, out)
    }
}

/// A Smacker movie, decoded a frame at a time.
pub struct Smacker {
    data: Vec<u8>,
    version4: bool,
    width: usize,
    height: usize,
    flags: u32,
    pts_inc: i32,
    audio_rates: [u32; 7],
    frame_sizes: Vec<u32>,
    frame_flags: Vec<u8>,
    frames_start: usize,
    trees: [BigTree; 4],
    palette: [u8; 768],
    image: Vec<u8>,
    next: usize,
    offset: usize,
}

const MMAP: usize = 0;
const MCLR: usize = 1;
const FULL: usize = 2;
const TYPE: usize = 3;

fn u32_at(data: &[u8], at: usize, what: &'static str) -> Result<u32, SmackerError> {
    data.get(at..at + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .ok_or(SmackerError::Truncated(what))
}

impl Smacker {
    /// Parse a movie's header and trees.
    ///
    /// # Errors
    /// A file that is not Smacker 2 or 4, or whose header or trees are
    /// truncated or invalid.
    pub fn open(data: Vec<u8>) -> Result<Self, SmackerError> {
        let version4 = match data.get(0..4) {
            Some(b"SMK2") => false,
            Some(b"SMK4") => true,
            _ => return Err(SmackerError::NotSmacker),
        };
        let header = |at| u32_at(&data, at, "header");
        let width = header(4)? as usize;
        let height = header(8)? as usize;
        let mut frames = header(12)? as usize;
        let pts_inc = header(16)? as i32;
        let flags = header(20)?;
        let tree_size = header(52)? as usize;
        let sizes = [header(56)?, header(60)?, header(64)?, header(68)?];
        let mut audio_rates = [0; 7];
        for (i, rate) in audio_rates.iter_mut().enumerate() {
            *rate = header(72 + i * 4)?;
        }
        if width == 0 || height == 0 || width > 4096 || height > 4096 || frames > 1_000_000 {
            return Err(SmackerError::NotSmacker);
        }
        if flags & FLAG_RING_FRAME != 0 {
            frames += 1;
        }
        let mut at = HEADER_SIZE;
        let frame_sizes = (0..frames)
            .map(|i| u32_at(&data, at + i * 4, "frame sizes"))
            .collect::<Result<Vec<_>, _>>()?;
        at += frames * 4;
        let frame_flags = data.get(at..at + frames).ok_or(SmackerError::Truncated("frame flags"))?.to_vec();
        at += frames;
        let tree_bytes = data.get(at..at + tree_size).ok_or(SmackerError::Truncated("trees"))?;
        let mut bits = Bits::new(tree_bytes);
        let mut read_tree = |size| -> Result<BigTree, SmackerError> {
            if bits.bit() == 0 {
                Ok(BigTree::absent())
            } else {
                BigTree::read(&mut bits, size)
            }
        };
        let trees = [read_tree(sizes[0])?, read_tree(sizes[1])?, read_tree(sizes[2])?, read_tree(sizes[3])?];
        let frames_start = at + tree_size;
        Ok(Self {
            data,
            version4,
            width,
            height,
            flags,
            pts_inc,
            audio_rates,
            frame_sizes,
            frame_flags,
            frames_start,
            trees,
            palette: [0; 768],
            image: vec![0; width * height],
            next: 0,
            offset: frames_start,
        })
    }

    #[must_use]
    pub fn width(&self) -> usize {
        self.width
    }

    #[must_use]
    pub fn height(&self) -> usize {
        self.height
    }

    /// The header's flags (`FLAG_Y_INTERLACE`, `FLAG_Y_DOUBLE`).
    #[must_use]
    pub fn flags(&self) -> u32 {
        self.flags
    }

    /// Frames, the ring frame included.
    #[must_use]
    pub fn frame_count(&self) -> usize {
        self.frame_sizes.len()
    }

    /// A frame's duration: the header's value in milliseconds when
    /// positive, in 10-microsecond units when negative, 10 fps when 0.
    #[must_use]
    pub fn frame_duration_us(&self) -> u64 {
        frame_duration(self.pts_inc)
    }

    /// The audio tracks the header declares, by track number.
    #[must_use]
    pub fn audio_tracks(&self) -> Vec<(usize, AudioTrack)> {
        self.audio_rates
            .iter()
            .enumerate()
            .filter(|(_, &rate)| rate & 0x00ff_ffff != 0)
            .map(|(i, &rate)| {
                let flags = rate >> 24;
                (
                    i,
                    AudioTrack {
                        sample_rate: rate & 0x00ff_ffff,
                        channels: if flags & AUDIO_STEREO != 0 { 2 } else { 1 },
                        bits: if flags & AUDIO_16BIT != 0 { 16 } else { 8 },
                        packed: flags & AUDIO_PACKED != 0,
                    },
                )
            })
            .collect()
    }

    /// Frame `index`'s bytes.
    fn frame_bytes(&self, index: usize, offset: usize) -> Result<&[u8], SmackerError> {
        let size = (self.frame_sizes[index] & !3) as usize;
        self.data.get(offset..offset + size).ok_or(SmackerError::Truncated("frame"))
    }

    /// The next frame, or `None` after the last.
    pub fn next_frame(&mut self) -> Option<Result<Frame<'_>, SmackerError>> {
        if self.next >= self.frame_sizes.len() {
            return None;
        }
        Some(self.decode_next().map(|()| Frame {
            width: self.width,
            height: self.height,
            indices: &self.image,
            palette: &self.palette,
        }))
    }

    /// The next frame as `width * height * 4` RGBA bytes (alpha 255) in a
    /// reused buffer, or `None` after the last.
    pub fn next_frame_rgba(&mut self, out: &mut Vec<u8>) -> Option<Result<(), SmackerError>> {
        Some(match self.next_frame()? {
            Ok(frame) => {
                frame.write_rgba(out);
                Ok(())
            }
            Err(error) => Err(error),
        })
    }

    fn decode_next(&mut self) -> Result<(), SmackerError> {
        let index = self.next;
        let offset = self.offset;
        self.next += 1;
        let size = (self.frame_sizes[index] & !3) as usize;
        self.offset += size;
        let frame = self.frame_bytes(index, offset)?.to_vec();
        let mut rest: &[u8] = &frame;
        if self.frame_flags[index] & FRAME_PALETTE != 0 {
            let chunk = usize::from(*rest.first().ok_or(SmackerError::BadPalette)?) * 4;
            if chunk == 0 || chunk > rest.len() {
                return Err(SmackerError::BadPalette);
            }
            update_palette(&mut self.palette, &rest[1..chunk])?;
            rest = &rest[chunk..];
        }
        for track in 0..7 {
            if self.frame_flags[index] & (0x02 << track) != 0 {
                let size = u32_at(rest, 0, "audio chunk")? as usize;
                if size < 8 || size > rest.len() {
                    return Err(SmackerError::BadAudio("chunk size"));
                }
                rest = &rest[size..];
            }
        }
        self.decode_video(rest);
        Ok(())
    }

    /// `decode_frame`'s block loop.
    fn decode_video(&mut self, data: &[u8]) {
        for tree in &mut self.trees {
            tree.reset();
        }
        let mut bits = Bits::new(data);
        let (stride, bw) = (self.width, self.width >> 2);
        let blocks = bw * (self.height >> 2);
        let mut block = 0;
        while block < blocks {
            let kind = self.trees[TYPE].decode(&mut bits);
            let mut run = BLOCK_RUNS[((kind >> 2) & 0x3f) as usize];
            let mode = match kind & 3 {
                1 if self.version4 => {
                    if bits.bit() == 1 {
                        1
                    } else if bits.bit() == 1 {
                        2
                    } else {
                        0
                    }
                }
                _ => 0,
            };
            while run > 0 && block < blocks {
                run -= 1;
                let base = (block / bw) * stride * 4 + (block % bw) * 4;
                match kind & 3 {
                    0 => {
                        let colour = self.trees[MCLR].decode(&mut bits);
                        let mut map = self.trees[MMAP].decode(&mut bits);
                        let (hi, lo) = ((colour >> 8) as u8, colour as u8);
                        for row in 0..4 {
                            let at = base + row * stride;
                            for x in 0..4 {
                                self.image[at + x] = if map & (1 << x) != 0 { hi } else { lo };
                            }
                            map >>= 4;
                        }
                    }
                    1 => self.full_block(&mut bits, base, mode),
                    2 => {}
                    _ => {
                        let colour = (kind >> 8) as u8;
                        for row in 0..4 {
                            let at = base + row * stride;
                            self.image[at..at + 4].fill(colour);
                        }
                    }
                }
                block += 1;
            }
        }
    }

    fn full_block(&mut self, bits: &mut Bits<'_>, base: usize, mode: u8) {
        let stride = self.width;
        let put = |image: &mut Vec<u8>, at: usize, pix: u32| {
            image[at] = pix as u8;
            image[at + 1] = (pix >> 8) as u8;
        };
        match mode {
            0 => {
                for row in 0..4 {
                    let at = base + row * stride;
                    let right = self.trees[FULL].decode(bits);
                    put(&mut self.image, at + 2, right);
                    let left = self.trees[FULL].decode(bits);
                    put(&mut self.image, at, left);
                }
            }
            1 => {
                for pair in 0..2 {
                    let pix = self.trees[FULL].decode(bits);
                    for row in 0..2 {
                        let at = base + (pair * 2 + row) * stride;
                        self.image[at..at + 2].fill(pix as u8);
                        self.image[at + 2..at + 4].fill((pix >> 8) as u8);
                    }
                }
            }
            _ => {
                for pair in 0..2 {
                    let right = self.trees[FULL].decode(bits);
                    let left = self.trees[FULL].decode(bits);
                    for row in 0..2 {
                        let at = base + (pair * 2 + row) * stride;
                        put(&mut self.image, at, left);
                        put(&mut self.image, at + 2, right);
                    }
                }
            }
        }
    }

    /// The whole of audio track `track`, decoded.
    ///
    /// # Errors
    /// A track the header does not declare, Bink audio, or a chunk that
    /// does not decode.
    pub fn audio(&self, track: usize) -> Result<Pcm, SmackerError> {
        let rate = *self.audio_rates.get(track).ok_or(SmackerError::NoTrack(track))?;
        if rate & 0x00ff_ffff == 0 {
            return Err(SmackerError::NoTrack(track));
        }
        let flags = rate >> 24;
        if flags & (AUDIO_BINK | AUDIO_DCT) != 0 {
            return Err(SmackerError::UnsupportedAudio(flags));
        }
        let channels: u16 = if flags & AUDIO_STEREO != 0 { 2 } else { 1 };
        let sixteen = flags & AUDIO_16BIT != 0;
        let mut samples = Vec::new();
        let mut offset = self.frames_start;
        for index in 0..self.frame_sizes.len() {
            let frame = self.frame_bytes(index, offset)?;
            offset += frame.len();
            let mut rest = frame;
            if self.frame_flags[index] & FRAME_PALETTE != 0 {
                let chunk = usize::from(*rest.first().ok_or(SmackerError::BadPalette)?) * 4;
                rest = rest.get(chunk..).ok_or(SmackerError::BadPalette)?;
            }
            for t in 0..7 {
                if self.frame_flags[index] & (0x02 << t) == 0 {
                    continue;
                }
                let size = u32_at(rest, 0, "audio chunk")? as usize;
                if size < 8 || size > rest.len() {
                    return Err(SmackerError::BadAudio("chunk size"));
                }
                if t == track {
                    let payload = &rest[4..size];
                    if flags & AUDIO_PACKED != 0 {
                        unpack_audio(payload, channels == 2, sixteen, &mut samples)?;
                    } else if sixteen {
                        samples.extend(payload.chunks_exact(2).map(|b| i16::from_le_bytes([b[0], b[1]])));
                    } else {
                        samples.extend(payload.iter().map(|&b| widen(b)));
                    }
                }
                rest = &rest[size..];
            }
        }
        Ok(Pcm { sample_rate: rate & 0x00ff_ffff, channels, samples })
    }

    /// Track `track` as a complete 16-bit RIFF WAV, or `None` when the
    /// header declares no such track.
    #[must_use]
    pub fn audio_wav(&self, track: usize) -> Option<Result<Vec<u8>, SmackerError>> {
        if self.audio_rates.get(track).is_none_or(|rate| rate & 0x00ff_ffff == 0) {
            return None;
        }
        Some(self.audio(track).map(|pcm| pcm.to_wav()))
    }
}

/// The header's frame rate as a frame's length in microseconds:
/// milliseconds when positive, 10-microsecond units when negative, 10 fps
/// when 0.
fn frame_duration(pts_inc: i32) -> u64 {
    match pts_inc {
        0 => 100_000,
        v if v > 0 => u64::from(v.unsigned_abs()) * 1000,
        v => u64::from(v.unsigned_abs()) * 10,
    }
}

/// An unsigned 8-bit sample as 16 bits, as FFmpeg converts it.
fn widen(sample: u8) -> i16 {
    ((i16::from(sample) - 128) << 8) as i16
}

/// A palette chunk's body (after its size byte): runs of kept entries,
/// copies from the previous palette, and new 6-bit colours.
fn update_palette(palette: &mut [u8; 768], chunk: &[u8]) -> Result<(), SmackerError> {
    let old = *palette;
    let mut bytes = chunk.iter().copied();
    let mut next = || bytes.next().ok_or(SmackerError::BadPalette);
    let mut entry = 0usize;
    while entry < 256 {
        let t = next()?;
        if t & 0x80 != 0 {
            entry += usize::from(t & 0x7f) + 1;
        } else if t & 0x40 != 0 {
            let from = usize::from(next()?);
            let count = usize::from(t & 0x3f) + 1;
            if from + count > 256 {
                return Err(SmackerError::BadPalette);
            }
            for i in 0..count {
                if entry >= 256 {
                    break;
                }
                palette[entry * 3..entry * 3 + 3].copy_from_slice(&old[(from + i) * 3..(from + i) * 3 + 3]);
                entry += 1;
            }
        } else {
            let (g, b) = (next()?, next()?);
            palette[entry * 3..entry * 3 + 3]
                .copy_from_slice(&[palette_component(t), palette_component(g), palette_component(b)]);
            entry += 1;
        }
    }
    Ok(())
}

/// One packed audio chunk (`smka_decode_frame`): DPCM through byte trees.
fn unpack_audio(chunk: &[u8], stereo: bool, sixteen: bool, out: &mut Vec<i16>) -> Result<(), SmackerError> {
    let unpacked = u32_at(chunk, 0, "audio chunk")? as usize;
    if unpacked > 1 << 24 {
        return Err(SmackerError::BadAudio("unpacked size"));
    }
    let mut bits = Bits::new(&chunk[4..]);
    if bits.bit() == 0 {
        return Ok(());
    }
    if (bits.bit() == 1) != stereo || (bits.bit() == 1) != sixteen {
        return Err(SmackerError::BadAudio("format differs from the header"));
    }
    let channels = if stereo { 2 } else { 1 };
    let width = if sixteen { 2 } else { 1 };
    if unpacked % (channels * width) != 0 {
        return Err(SmackerError::BadAudio("unpacked size"));
    }
    let mut trees: Vec<Option<ByteTree>> = Vec::new();
    for _ in 0..(1usize << (usize::from(sixteen) + usize::from(stereo))) {
        bits.bit();
        let tree = ByteTree::read(&mut bits)?;
        bits.bit();
        trees.push(Some(tree));
    }
    let mut pred = [0u32; 2];
    if sixteen {
        for i in (0..channels).rev() {
            pred[i] = u32::from((bits.bits(16) as u16).swap_bytes());
        }
        out.extend(pred[..channels].iter().map(|&p| p as u16 as i16));
        for i in channels..unpacked / 2 {
            let idx = 2 * (i & usize::from(stereo));
            if bits.left() < 0 {
                return Err(SmackerError::BadAudio("chunk ends early"));
            }
            let low = byte_value(&trees[idx], &mut bits);
            let high = byte_value(&trees[idx + 1], &mut bits);
            let p = &mut pred[idx / 2];
            *p = p.wrapping_add(low | (high << 8));
            out.push(*p as u16 as i16);
        }
    } else {
        for i in (0..channels).rev() {
            pred[i] = bits.bits(8);
        }
        out.extend(pred[..channels].iter().map(|&p| widen(p as u8)));
        for i in channels..unpacked {
            let idx = i & usize::from(stereo);
            if bits.left() < 0 {
                return Err(SmackerError::BadAudio("chunk ends early"));
            }
            let value = byte_value(&trees[idx], &mut bits);
            pred[idx] = pred[idx].wrapping_add(value);
            out.push(widen(pred[idx] as u8));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_frame_rate_reads_as_milliseconds_ten_microseconds_or_ten_fps() {
        // The game's movies say -6666: 66,660 microseconds, 15 fps.
        assert_eq!(frame_duration(-6666), 66_660);
        assert_eq!(frame_duration(100), 100_000);
        assert_eq!(frame_duration(1), 1000);
        assert_eq!(frame_duration(0), 100_000);
    }

    #[test]
    fn eight_bit_samples_widen_around_their_midpoint() {
        assert_eq!(widen(128), 0);
        assert_eq!(widen(0), -32768);
        assert_eq!(widen(255), 127 << 8);
        assert_eq!(widen(129), 256);
    }

    #[test]
    fn decoded_sound_becomes_a_16_bit_pcm_wave() {
        let wav = Pcm { sample_rate: 11_025, channels: 2, samples: vec![1, -2, 3, -4] }.to_wav();
        let u32_at = |at: usize| u32::from_le_bytes(wav[at..at + 4].try_into().unwrap());
        let u16_at = |at: usize| u16::from_le_bytes(wav[at..at + 2].try_into().unwrap());
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(u32_at(4), 36 + 8);
        assert_eq!(&wav[8..16], b"WAVEfmt ");
        assert_eq!((u32_at(16), u16_at(20), u16_at(22)), (16, 1, 2));
        assert_eq!((u32_at(24), u32_at(28), u16_at(32), u16_at(34)), (11_025, 44_100, 4, 16));
        assert_eq!(&wav[36..40], b"data");
        assert_eq!(u32_at(40), 8);
        assert_eq!(&wav[44..], &[1, 0, 0xfe, 0xff, 3, 0, 0xfc, 0xff]);
    }

    /// Bits written lowest first into bytes.
    fn pack(bits: &[u32]) -> Vec<u8> {
        let mut out = vec![0u8; bits.len().div_ceil(8)];
        for (i, &b) in bits.iter().enumerate() {
            out[i / 8] |= (b as u8) << (i % 8);
        }
        out
    }

    fn byte_bits(value: u8) -> Vec<u32> {
        (0..8).map(|i| u32::from(value >> i) & 1).collect()
    }

    #[test]
    fn the_bit_reader_takes_each_bytes_lowest_bit_first_and_zeros_past_the_end() {
        let mut bits = Bits::new(&[0b1000_0001, 0x34]);
        assert_eq!(bits.bit(), 1);
        assert_eq!(bits.bits(7), 0b100_0000);
        assert_eq!(bits.bits(8), 0x34);
        assert_eq!(bits.bits(4), 0);
    }

    #[test]
    fn a_byte_tree_goes_left_on_0_and_right_on_1() {
        // Node(Leaf 7, Node(Leaf 9, Leaf 11)), then codes 0, 11, 10.
        let mut stream = vec![1, 0];
        stream.extend(byte_bits(7));
        stream.extend([1, 0]);
        stream.extend(byte_bits(9));
        stream.push(0);
        stream.extend(byte_bits(11));
        stream.extend([0, 1, 1, 1, 0]);
        let data = pack(&stream);
        let mut bits = Bits::new(&data);
        let tree = ByteTree::read(&mut bits).unwrap();
        let decoded: Vec<u8> = (0..3).map(|_| tree.decode(&mut bits)).collect();
        assert_eq!(decoded, [7, 11, 9]);
    }

    #[test]
    fn a_big_trees_escape_leaves_repeat_the_last_three_values() {
        let mut tree = BigTree {
            // Node(left 1 entry: leaf 5, right: Node(escape0, escape1)).
            values: vec![NODE | 1, 5, NODE | 1, 0, 0, 0],
            last: [3, 4, 5],
        };
        let data = pack(&[0, 1, 0, 1, 1]);
        let mut bits = Bits::new(&data);
        assert_eq!(tree.decode(&mut bits), 5); // leaf 5; cache 5
        assert_eq!(tree.decode(&mut bits), 5); // escape 0: last value
        assert_eq!(tree.decode(&mut bits), 0); // escape 1: second last (0)
        tree.reset();
        assert_eq!(&tree.values[3..], &[0, 0, 0]);
    }

    #[test]
    fn a_palette_chunk_keeps_copies_and_sets_entries() {
        let mut palette = [0u8; 768];
        palette[3..6].copy_from_slice(&[10, 20, 30]);
        // New entry 0 = (63, 32, 1); copy 1 entry from old 1 into 1;
        // skip the remaining 254.
        update_palette(&mut palette, &[63, 32, 1, 0x40, 1, 0x80 | 127, 0x80 | 126]).unwrap();
        assert_eq!(&palette[0..6], &[255, 130, 4, 10, 20, 30]);
        assert_eq!(palette_component(16), 0x41);
        assert!(update_palette(&mut palette, &[0x40 | 3, 254]).is_err());
    }

    #[test]
    fn a_mono_wave_s_block_is_two_bytes() {
        let wav = Pcm { sample_rate: 11_025, channels: 1, samples: vec![7] }.to_wav();
        assert_eq!(u32::from_le_bytes(wav[28..32].try_into().unwrap()), 22_050);
        assert_eq!(u16::from_le_bytes(wav[32..34].try_into().unwrap()), 2);
    }

    #[test]
    fn errors_say_what_failed() {
        let cases = [
            (SmackerError::NotSmacker, "not a Smacker movie"),
            (SmackerError::Truncated("trees"), "truncated trees"),
            (SmackerError::BadTree("big"), "invalid big tree"),
            (SmackerError::BadPalette, "invalid palette chunk"),
            (SmackerError::BadAudio("chunk size"), "invalid audio: chunk size"),
            (SmackerError::UnsupportedAudio(8), "unsupported (Bink) audio, flags 0x8"),
            (SmackerError::NoTrack(3), "no audio track 3"),
        ];
        for (error, text) in cases {
            assert_eq!(error.to_string(), text);
        }
    }

    #[test]
    fn the_bits_left_go_negative_past_the_end() {
        let mut bits = Bits::new(&[0, 0]);
        assert_eq!(bits.left(), 16);
        bits.bits(3);
        assert_eq!(bits.left(), 13);
        bits.bits(17);
        assert_eq!(bits.left(), -4);
    }

    fn leaf_bits(value: u8) -> Vec<u32> {
        let mut out = vec![0];
        out.extend(byte_bits(value));
        out
    }

    /// A byte tree whose leaves all lie `depth` deep.
    fn full_tree(depth: usize) -> Vec<u32> {
        if depth == 0 {
            return leaf_bits(0);
        }
        let mut out = vec![1];
        out.extend(full_tree(depth - 1));
        out.extend(full_tree(depth - 1));
        out
    }

    fn read_byte_tree(stream: &[u32]) -> Result<ByteTree, SmackerError> {
        ByteTree::read(&mut Bits::new(&pack(stream)))
    }

    #[test]
    fn a_byte_tree_is_at_most_32_deep_on_either_side() {
        let left_chain = |depth: usize| {
            let mut out = vec![1; depth];
            for _ in 0..=depth {
                out.extend(leaf_bits(0));
            }
            out
        };
        let right_chain = |depth: usize| {
            let mut out = Vec::new();
            for _ in 0..depth {
                out.push(1);
                out.extend(leaf_bits(0));
            }
            out.extend(leaf_bits(0));
            out
        };
        assert!(read_byte_tree(&left_chain(32)).is_ok());
        assert_eq!(read_byte_tree(&left_chain(33)).err(), Some(SmackerError::BadTree("byte")));
        assert!(read_byte_tree(&right_chain(32)).is_ok());
        assert_eq!(read_byte_tree(&right_chain(33)).err(), Some(SmackerError::BadTree("byte")));
    }

    #[test]
    fn a_byte_tree_holds_at_most_256_leaves() {
        assert_eq!(read_byte_tree(&full_tree(8)).unwrap().nodes.len(), 511);
        let mut more = vec![1];
        more.extend(full_tree(8));
        more.extend(leaf_bits(0));
        assert_eq!(read_byte_tree(&more).err(), Some(SmackerError::BadTree("byte")));
    }

    #[test]
    fn a_byte_tree_leaf_needs_its_eight_bits() {
        // Two zero bytes: a leaf flag at bit 7 leaves exactly 8 bits, at
        // bit 8 only 7.
        let data = [0, 0];
        let mut bits = Bits { data: &data, pos: 7 };
        assert!(ByteTree::read(&mut bits).is_ok());
        let mut bits = Bits { data: &data, pos: 8 };
        assert_eq!(ByteTree::read(&mut bits).err(), Some(SmackerError::BadTree("byte")));
    }

    fn word_bits(value: u32) -> Vec<u32> {
        (0..16).map(|i| (value >> i) & 1).collect()
    }

    /// A big tree with both byte trees absent and escapes 1, 2 and 3, then `nodes`.
    fn big_stream(nodes: &[u32]) -> Vec<u32> {
        let mut out = vec![0, 0];
        for escape in [1, 2, 3] {
            out.extend(word_bits(escape));
        }
        out.extend_from_slice(nodes);
        out
    }

    fn read_big(stream: &[u32], size: u32) -> Result<BigTree, SmackerError> {
        BigTree::read(&mut Bits::new(&pack(stream)), size)
    }

    #[test]
    fn a_big_tree_fits_the_size_its_header_gives() {
        // A quarter of the size, rounded up, is the entry count.
        let one = read_big(&big_stream(&[0]), 1).unwrap();
        assert_eq!(one.values, vec![0, 0, 0, 0], "a leaf and three escape slots");
        assert_eq!(one.last, [1, 2, 3]);
        assert_eq!(read_big(&big_stream(&[0]), 0).err(), Some(SmackerError::BadTree("big")));
        assert!(read_big(&big_stream(&[1, 0, 0]), 9).is_ok());
        assert_eq!(read_big(&big_stream(&[1, 0, 0]), 8).err(), Some(SmackerError::BadTree("big")));
        let mut none = Bits::new(&[]);
        assert_eq!(BigTree::read(&mut none, u32::MAX >> 4).err(), Some(SmackerError::BadTree("header")));
    }

    #[test]
    fn a_big_tree_is_at_most_500_deep_on_either_side() {
        let left_chain = |depth: usize| {
            let mut nodes = vec![1; depth];
            nodes.extend(vec![0; depth + 1]);
            big_stream(&nodes)
        };
        let right_chain = |depth: usize| {
            let mut nodes = [1, 0].repeat(depth);
            nodes.push(0);
            big_stream(&nodes)
        };
        let size = |depth: usize| 4 * (2 * depth as u32 + 1);
        assert!(read_big(&left_chain(500), size(500)).is_ok());
        assert_eq!(read_big(&left_chain(501), size(501)).err(), Some(SmackerError::BadTree("big")));
        assert!(read_big(&right_chain(500), size(500)).is_ok());
        assert_eq!(read_big(&right_chain(501), size(501)).err(), Some(SmackerError::BadTree("big")));
    }

    #[test]
    fn a_big_tree_cut_short_is_refused() {
        // 50 header bits and six branches fill seven bytes exactly.
        let stream = big_stream(&[1; 6]);
        assert_eq!(pack(&stream).len() * 8, stream.len());
        assert_eq!(read_big(&stream, 100).err(), Some(SmackerError::BadTree("big")));
    }

    #[test]
    fn a_frame_is_its_palette_colours_and_shows_doubled_or_interlaced() {
        let mut palette = [0u8; 768];
        palette[..9].copy_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8, 9]);
        let frame = Frame { width: 2, height: 2, indices: &[1, 2, 0, 1], palette: &palette };
        let (a, b, c) = ([4, 5, 6, 255], [7, 8, 9, 255], [1, 2, 3, 255]);
        let black = [0, 0, 0, 255];
        let rows = |rows: &[[[u8; 4]; 2]]| rows.iter().flatten().flatten().copied().collect::<Vec<u8>>();
        assert_eq!(frame.to_rgba(), rows(&[[a, b], [c, a]]));
        assert_eq!(frame.to_display_rgba(0), (2, 2, rows(&[[a, b], [c, a]])));
        assert_eq!(frame.to_display_rgba(FLAG_RING_FRAME), (2, 2, rows(&[[a, b], [c, a]])));
        let doubled = rows(&[[a, b], [a, b], [c, a], [c, a]]);
        assert_eq!(frame.to_display_rgba(FLAG_Y_DOUBLE), (2, 4, doubled.clone()));
        assert_eq!(frame.to_display_rgba(FLAG_Y_DOUBLE | FLAG_Y_INTERLACE), (2, 4, doubled));
        let interlaced = rows(&[[a, b], [black, black], [c, a], [black, black]]);
        assert_eq!(frame.to_display_rgba(FLAG_Y_INTERLACE), (2, 4, interlaced));
    }

    fn put_u32(out: &mut [u8], at: usize, value: u32) {
        out[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }

    /// A movie with absent trees: `magic`, `size`, header `flags`, audio
    /// `rates`, then each frame's (flags, bytes), padded to 4 bytes.
    fn smk_file(magic: &[u8; 4], (width, height): (u32, u32), flags: u32, rates: [u32; 7], frames: &[(u8, Vec<u8>)]) -> Vec<u8> {
        let mut out = vec![0u8; HEADER_SIZE];
        out[0..4].copy_from_slice(magic);
        put_u32(&mut out, 4, width);
        put_u32(&mut out, 8, height);
        put_u32(&mut out, 12, frames.len() as u32 - (flags & FLAG_RING_FRAME));
        put_u32(&mut out, 16, (-6666i32) as u32);
        put_u32(&mut out, 20, flags);
        put_u32(&mut out, 52, 4);
        for (i, rate) in rates.iter().enumerate() {
            put_u32(&mut out, 72 + i * 4, *rate);
        }
        let padded: Vec<Vec<u8>> = frames
            .iter()
            .map(|(_, bytes)| {
                let mut bytes = bytes.clone();
                bytes.resize(bytes.len().div_ceil(4) * 4, 0);
                bytes
            })
            .collect();
        for bytes in &padded {
            out.extend((bytes.len() as u32).to_le_bytes());
        }
        out.extend(frames.iter().map(|(flags, _)| *flags));
        out.extend([0u8; 4]);
        out.extend(padded.concat());
        out
    }

    #[test]
    fn the_header_gives_the_size_flags_frames_and_audio_tracks() {
        let stereo = (AUDIO_PACKED | AUDIO_16BIT | AUDIO_STEREO) << 24;
        let rates = [22_050 | stereo, 0, 11_025, AUDIO_16BIT << 24, 0, 0, 0];
        let file = smk_file(b"SMK2", (8, 4), FLAG_Y_DOUBLE, rates, &[(0, vec![0; 4]), (0, vec![0; 8])]);
        let smk = Smacker::open(file).unwrap();
        assert_eq!((smk.width(), smk.height(), smk.flags(), smk.frame_count()), (8, 4, FLAG_Y_DOUBLE, 2));
        assert_eq!(smk.frame_sizes, vec![4, 8]);
        assert!(!smk.version4);
        let packed = AudioTrack { sample_rate: 22_050, channels: 2, bits: 16, packed: true };
        let plain = AudioTrack { sample_rate: 11_025, channels: 1, bits: 8, packed: false };
        assert_eq!(smk.audio_tracks(), vec![(0, packed), (2, plain)]);
        assert!(smk.audio_wav(3).is_none(), "flags without a rate");
        assert!(smk.audio_wav(1).is_none());
        assert!(smk.audio_wav(7).is_none());
    }

    #[test]
    fn a_ring_frame_adds_a_frame_and_smk4_reads_too() {
        let file = smk_file(b"SMK4", (4, 4), FLAG_RING_FRAME, [0; 7], &[(0, vec![]), (0, vec![])]);
        let smk = Smacker::open(file).unwrap();
        assert_eq!(smk.frame_count(), 2);
        assert!(smk.version4);
    }

    #[test]
    fn a_header_past_the_size_limits_is_refused() {
        let open = |width, height| Smacker::open(smk_file(b"SMK2", (width, height), 0, [0; 7], &[])).map(|_| ());
        assert_eq!(open(4096, 4), Ok(()));
        assert_eq!(open(4, 4096), Ok(()));
        for (width, height) in [(0, 4), (4, 0), (4097, 4), (4, 4097)] {
            assert_eq!(open(width, height), Err(SmackerError::NotSmacker), "{width}x{height}");
        }
        let mut file = smk_file(b"SMK2", (4, 4), 0, [0; 7], &[]);
        put_u32(&mut file, 12, 1_000_000);
        assert_eq!(Smacker::open(file.clone()).err(), Some(SmackerError::Truncated("frame sizes")));
        put_u32(&mut file, 12, 1_000_001);
        assert_eq!(Smacker::open(file).err(), Some(SmackerError::NotSmacker));
    }

    fn first_frame(file: Vec<u8>) -> Result<(Vec<u8>, [u8; 768]), SmackerError> {
        let mut smk = Smacker::open(file)?;
        let frame = smk.next_frame().unwrap()?;
        Ok((frame.indices.to_vec(), *frame.palette))
    }

    #[test]
    fn a_palette_chunk_must_fit_its_frame() {
        let movie = |bytes: Vec<u8>| smk_file(b"SMK2", (4, 4), 0, [0; 7], &[(FRAME_PALETTE, bytes)]);
        // Size byte 2 (8 bytes): entry 0 = (63, 0, 0), then skip 128 and 127.
        let (_, palette) = first_frame(movie(vec![2, 63, 0, 0, 0xff, 0xfe, 0, 0])).unwrap();
        assert_eq!(&palette[..6], &[255, 0, 0, 0, 0, 0]);
        assert_eq!(first_frame(movie(vec![0, 0, 0, 0])).err(), Some(SmackerError::BadPalette), "no size");
        assert_eq!(first_frame(movie(vec![3, 0xff, 0xff, 0, 0, 0, 0, 0])).err(), Some(SmackerError::BadPalette));
    }

    #[test]
    fn an_audio_chunk_is_at_least_8_bytes_and_inside_its_frame() {
        let movie = |size: u32| {
            let mut bytes = size.to_le_bytes().to_vec();
            bytes.extend([0; 4]);
            smk_file(b"SMK2", (4, 4), 0, [4, 0, 0, 0, 0, 0, 0], &[(0x02, bytes)])
        };
        assert!(first_frame(movie(8)).is_ok());
        for size in [4, 7, 12] {
            assert_eq!(first_frame(movie(size)).err(), Some(SmackerError::BadAudio("chunk size")), "size {size}");
            assert_eq!(Smacker::open(movie(size)).unwrap().audio(0).err(), Some(SmackerError::BadAudio("chunk size")));
        }
    }

    #[test]
    fn unpacked_audio_tracks_are_read_past_the_palette_and_other_tracks() {
        let rates = [
            4 | (AUDIO_16BIT | AUDIO_STEREO) << 24,
            4,
            4 | AUDIO_BINK << 24,
            AUDIO_16BIT << 24,
            4 | AUDIO_DCT << 24,
            0,
            0,
        ];
        let mut bytes = vec![1, 0xff, 0xff, 0];
        bytes.extend(12u32.to_le_bytes());
        bytes.extend([1, 0, 0xfe, 0xff, 3, 0, 0xfc, 0xff]);
        bytes.extend(8u32.to_le_bytes());
        bytes.extend([128, 0, 255, 129]);
        let smk = Smacker::open(smk_file(b"SMK2", (4, 4), 0, rates, &[(FRAME_PALETTE | 0x02 | 0x04, bytes)])).unwrap();
        assert_eq!(smk.audio(0), Ok(Pcm { sample_rate: 4, channels: 2, samples: vec![1, -2, 3, -4] }));
        assert_eq!(smk.audio(1), Ok(Pcm { sample_rate: 4, channels: 1, samples: vec![0, -32768, 32512, 256] }));
        assert_eq!(smk.audio(2).err(), Some(SmackerError::UnsupportedAudio(AUDIO_BINK)));
        assert_eq!(smk.audio(3).err(), Some(SmackerError::NoTrack(3)));
        assert_eq!(smk.audio(4).err(), Some(SmackerError::UnsupportedAudio(AUDIO_DCT)));
        assert_eq!(smk.audio(9).err(), Some(SmackerError::NoTrack(9)));
        let wav = smk.audio_wav(1).unwrap().unwrap();
        assert_eq!(&wav[44..], &[0, 0, 0, 0x80, 0, 0x7f, 0, 1]);
    }

    fn leaf(value: u32) -> BigTree {
        BigTree { values: vec![value, 0, 0, 0], last: [1, 2, 3] }
    }

    /// A tree giving `zero` on a 0 bit and `one` on a 1.
    fn pair(zero: u32, one: u32) -> BigTree {
        BigTree { values: vec![NODE | 1, zero, one, 0, 0, 0], last: [3, 4, 5] }
    }

    #[test]
    fn solid_and_skipped_blocks_run_to_the_frame_s_last_block_and_no_further() {
        let mut smk = Smacker::open(smk_file(b"SMK2", (8, 4), 0, [0; 7], &[(0, vec![]), (0, vec![]), (0, vec![])])).unwrap();
        // Type 3 (solid) with run 2 in colour 0x55; skip (2) with run 3;
        // solid 0x66 with run 3 over the two blocks.
        smk.trees[TYPE] = leaf(3 | 1 << 2 | 0x55 << 8);
        assert_eq!(smk.next_frame().unwrap().unwrap().indices, &[0x55; 32]);
        smk.trees[TYPE] = leaf(2 | 2 << 2);
        assert_eq!(smk.next_frame().unwrap().unwrap().indices, &[0x55; 32]);
        smk.trees[TYPE] = leaf(3 | 2 << 2 | 0x66 << 8);
        assert_eq!(smk.next_frame().unwrap().unwrap().indices, &[0x66; 32]);
    }

    #[test]
    fn full_blocks_take_smk4_s_doubled_and_paired_modes() {
        let (p, q) = ([0x11, 0x22], [0x33, 0x44]);
        let row = |left: [u8; 2], right: [u8; 2]| [left, right].concat();
        let rows_mode0 = [row(q, p), row(p, q), row(q, q), row(p, p)].concat();
        let mode0 = [0, 1, 1, 0, 1, 1, 0, 0];
        let frames = |magic: &[u8; 4], streams: &[Vec<u32>]| {
            let frames: Vec<(u8, Vec<u8>)> = streams.iter().map(|s| (0, pack(s))).collect();
            let mut smk = Smacker::open(smk_file(magic, (4, 4), 0, [0; 7], &frames)).unwrap();
            smk.trees[TYPE] = leaf(1);
            smk.trees[FULL] = pair(0x2211, 0x4433);
            streams.iter().map(|_| smk.next_frame().unwrap().unwrap().indices.to_vec()).collect::<Vec<_>>()
        };
        // SMK2: every full block is mode 0, rows' right half first.
        assert_eq!(frames(b"SMK2", &[mode0.to_vec()]), vec![rows_mode0.clone()]);
        let smk4 = frames(
            b"SMK4",
            &[[&[0, 0][..], &mode0].concat(), vec![1, 0, 1], vec![0, 1, 0, 1, 1, 0]],
        );
        assert_eq!(smk4[0], rows_mode0, "00: mode 0");
        let doubled = [[0x11, 0x11, 0x22, 0x22]; 2].concat();
        let doubled_q = [[0x33, 0x33, 0x44, 0x44]; 2].concat();
        assert_eq!(smk4[1], [doubled, doubled_q].concat(), "1: a pixel pair per two rows");
        assert_eq!(smk4[2], [row(q, p), row(q, p), row(p, q), row(p, q)].concat(), "01: right, left per two rows");
    }

    #[test]
    fn palette_copies_take_consecutive_old_entries_up_to_entry_255() {
        let mut old = [0u8; 768];
        for (i, value) in old.iter_mut().enumerate() {
            *value = i as u8;
        }
        // Copy 3 from 5 into 0, skip 253.
        let mut palette = old;
        update_palette(&mut palette, &[0x40 | 2, 5, 0x80 | 127, 0x80 | 124]).unwrap();
        assert_eq!(&palette[..9], &old[15..24]);
        assert_eq!(&palette[9..], &old[9..]);
        // Copy 2 from 254 (the last two), skip 254.
        let mut palette = old;
        update_palette(&mut palette, &[0x40 | 1, 254, 0x80 | 127, 0x80 | 125]).unwrap();
        assert_eq!(&palette[..6], &old[762..768]);
        // Skip 250, then a copy of 10 stops at entry 255.
        let mut palette = old;
        update_palette(&mut palette, &[0x80 | 127, 0x80 | 121, 0x40 | 9, 0]).unwrap();
        assert_eq!(&palette[750..], &old[..18]);
    }

    /// A packed chunk: the unpacked size, then `stream`.
    fn packed(unpacked: u32, stream: &[u32]) -> Vec<u8> {
        let mut out = unpacked.to_le_bytes().to_vec();
        out.extend(pack(stream));
        out
    }

    /// A byte tree: 0 adds 1, 10 subtracts 1 (255), 11 adds 2, framed by
    /// the unused bits around each tree.
    fn delta_tree() -> Vec<u32> {
        let mut out = vec![0, 1];
        out.extend(leaf_bits(1));
        out.push(1);
        out.extend(leaf_bits(255));
        out.extend(leaf_bits(2));
        out.push(0);
        out
    }

    #[test]
    fn packed_audio_s_unpacked_size_is_capped_and_whole_samples() {
        let mut out = Vec::new();
        assert_eq!(unpack_audio(&packed(1 << 24, &[0]), false, false, &mut out), Ok(()));
        assert_eq!(
            unpack_audio(&packed((1 << 24) + 1, &[0]), false, false, &mut out).err(),
            Some(SmackerError::BadAudio("unpacked size"))
        );
        assert!(out.is_empty());
        // Mono 16-bit, one 2-byte sample: two one-leaf trees, then the
        // first sample big-endian.
        let mut stream = vec![1, 0, 1];
        for _ in 0..2 {
            stream.push(0);
            stream.extend(leaf_bits(0));
            stream.push(0);
        }
        stream.extend(word_bits(0x3412));
        assert_eq!(unpack_audio(&packed(2, &stream), false, true, &mut out), Ok(()));
        assert_eq!(out, vec![0x1234]);
        // Stereo 8-bit, one 2-byte frame: right then left first samples.
        let mut stream = vec![1, 1, 0];
        for _ in 0..2 {
            stream.push(0);
            stream.extend(leaf_bits(0));
            stream.push(0);
        }
        stream.extend(byte_bits(129));
        stream.extend(byte_bits(130));
        out.clear();
        assert_eq!(unpack_audio(&packed(2, &stream), true, false, &mut out), Ok(()));
        assert_eq!(out, vec![widen(130), widen(129)]);
    }

    #[test]
    fn packed_8_bit_audio_decodes_deltas_until_the_chunk_is_overread() {
        // 3 flag bits, a 31-bit tree, the first sample: 42 bits; codes 11,
        // 10, 11 fill 48, so the fourth delta reads at the very end.
        let mut stream = vec![1, 0, 0];
        stream.extend(delta_tree());
        stream.extend(byte_bits(128));
        stream.extend([1, 1, 1, 0, 1, 1]);
        assert_eq!(stream.len(), 48);
        let mut out = Vec::new();
        assert_eq!(unpack_audio(&packed(5, &stream), false, false, &mut out), Ok(()));
        assert_eq!(out, vec![0, 512, 256, 768, 1024]);
        out.clear();
        assert_eq!(
            unpack_audio(&packed(6, &stream), false, false, &mut out).err(),
            Some(SmackerError::BadAudio("chunk ends early"))
        );
    }

    #[test]
    fn packed_16_bit_audio_decodes_deltas_until_the_chunk_is_overread() {
        // 3 flag bits, the delta tree (low bytes) and a one-leaf tree
        // (high bytes), the first sample: 61 bits; codes 11, 0 fill 64.
        let mut stream = vec![1, 0, 1];
        stream.extend(delta_tree());
        stream.push(0);
        stream.extend(leaf_bits(0));
        stream.push(0);
        stream.extend(word_bits(0x3412));
        stream.extend([1, 1, 0]);
        assert_eq!(stream.len(), 64);
        let mut out = Vec::new();
        assert_eq!(unpack_audio(&packed(8, &stream), false, true, &mut out), Ok(()));
        assert_eq!(out, vec![0x1234, 0x1236, 0x1237, 0x1238]);
        out.clear();
        assert_eq!(
            unpack_audio(&packed(10, &stream), false, true, &mut out).err(),
            Some(SmackerError::BadAudio("chunk ends early"))
        );
    }

    #[test]
    fn a_file_that_is_not_smacker_is_refused() {
        assert_eq!(Smacker::open(b"RIFF....".to_vec()).err(), Some(SmackerError::NotSmacker));
    }

    fn movie(name: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(std::env::var("HOME").unwrap())
            .join("Games/Heroic/Star Wars - Rebellion/MDATA")
            .join(name)
    }

    /// FFmpeg's output of `args` for the movie.
    fn ffmpeg(path: &std::path::Path, args: &[&str]) -> Vec<u8> {
        let out = std::process::Command::new("ffmpeg")
            .args(["-v", "error", "-i"])
            .arg(path)
            .args(args)
            .arg("-")
            .output()
            .unwrap();
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        out.stdout
    }

    /// Every frame (up to `limit`) and the whole audio track equal FFmpeg's.
    fn compare_with_ffmpeg(name: &str, limit: Option<usize>) {
        let path = movie(name);
        let mut smk = Smacker::open(std::fs::read(&path).unwrap()).unwrap();
        let mut video_args = vec!["-map", "0:v", "-fps_mode", "passthrough"];
        let limit_text = limit.map(|n| n.to_string());
        if let Some(n) = &limit_text {
            video_args.extend(["-frames:v", n.as_str()]);
        }
        video_args.extend(["-f", "rawvideo", "-pix_fmt", "rgb24"]);
        let video = ffmpeg(&path, &video_args);
        let frame_len = smk.width() * smk.height() * 3;
        let expected_frames = video.len() / frame_len;
        assert_eq!(video.len() % frame_len, 0);
        if limit.is_none() {
            assert_eq!(expected_frames, smk.frame_count(), "{name} frame count");
        }
        let mut rgba = Vec::new();
        for (index, expected) in video.chunks_exact(frame_len).enumerate() {
            smk.next_frame_rgba(&mut rgba).unwrap().unwrap();
            let rgb: Vec<u8> = rgba.chunks_exact(4).flat_map(|p| [p[0], p[1], p[2]]).collect();
            assert!(rgb == expected, "{name} frame {index} differs from FFmpeg's");
        }
        for (track, info) in smk.audio_tracks() {
            let pcm = smk.audio(track).unwrap();
            assert_eq!((pcm.sample_rate, pcm.channels), (info.sample_rate, info.channels));
            let map = format!("0:a:{}", smk.audio_tracks().iter().position(|(t, _)| *t == track).unwrap());
            let raw = ffmpeg(&path, &["-map", &map, "-f", "s16le", "-acodec", "pcm_s16le"]);
            let expected: Vec<i16> = raw.chunks_exact(2).map(|b| i16::from_le_bytes([b[0], b[1]])).collect();
            assert_eq!(pcm.samples.len(), expected.len(), "{name} track {track} sample count");
            assert!(pcm.samples == expected, "{name} track {track} samples differ from FFmpeg's");
        }
        eprintln!("{name}: {expected_frames} frames and {} audio tracks equal FFmpeg's", smk.audio_tracks().len());
    }

    #[test]
    #[ignore = "requires the original movies and ffmpeg"]
    fn the_logo_movie_decodes_as_ffmpeg_decodes_it() {
        compare_with_ffmpeg("MDATA.000", None);
    }

    #[test]
    #[ignore = "requires the original movies and ffmpeg"]
    fn a_long_movie_decodes_as_ffmpeg_decodes_it() {
        compare_with_ffmpeg("MDATA.103", None);
    }

    #[test]
    #[ignore = "requires the original movies"]
    fn the_longest_movie_decodes_faster_than_it_plays() {
        let mut smk = Smacker::open(std::fs::read(movie("MDATA.001")).unwrap()).unwrap();
        let mut rgba = Vec::new();
        let start = std::time::Instant::now();
        let mut frames = 0;
        while let Some(frame) = smk.next_frame_rgba(&mut rgba) {
            frame.unwrap();
            frames += 1;
        }
        let audio = smk.audio_wav(0).unwrap().unwrap();
        let seconds = start.elapsed().as_secs_f64();
        let fps = frames as f64 / seconds;
        eprintln!("MDATA.001: {frames} frames in {seconds:.2} s ({fps:.0} fps), audio {} bytes", audio.len());
        assert_eq!(frames, smk.frame_count());
        assert_eq!(smk.frame_duration_us(), 66_660);
    }
}
