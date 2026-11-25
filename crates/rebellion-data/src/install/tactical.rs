//! TACTICAL.DLL's 3-D resources as the runtime objects the battle renderer
//! loads: each type-301 binary DirectX mesh as an `ORTMESH` object, each
//! type-303 run-length image as an `ORTINDEX` object and each of its 27
//! palettes as an `ORTPAL00` object (pack kinds 4 and 5).
//!
//! port: the object formats are the port's own; the conversion follows
//! `tools/stage-ui-assets/tactical3d.go`, `tactical3d_decode.go` and
//! `tactical3d_runtime.go` (`stageTactical3DRuntimeResources`) step for step,
//! so the bytes equal the offline pack's.

use super::pack::{BuildError, Entry, InstallFiles};
use super::pe::{self, ResId};

pub const KIND_TACTICAL_MESH: u8 = 4;
pub const KIND_TACTICAL_TEXTURE: u8 = 5;

const MESH_TYPE: u32 = 301;
const TEXTURE_TYPE: u32 = 303;
const MESH_COUNT: usize = 87;
const TEXTURE_COUNT: usize = 397;
/// The one original mesh without normals; its faces' normals are derived
/// (`allowedDerivedTacticalNormals`).
const MESH_WITHOUT_NORMALS: u32 = 2082;
/// The images with four trailing bytes kept verbatim (`allowedTacticalImageTail`).
const IMAGES_WITH_TAILS: std::ops::RangeInclusive<u32> = 4200..=4204;
/// The palettes (`isTacticalPaletteResource`) and the planet images bound to
/// them, 30 below (`tacticalPaletteBinding`).
const PALETTES: std::ops::RangeInclusive<u32> = 5531..=5557;
const PLANET_IMAGES: std::ops::RangeInclusive<u32> = 5501..=5527;

const MAX_X_STRING: u32 = 4096;
const MAX_X_VERTICES: u32 = 1 << 20;
const MAX_X_FACES: u32 = 1 << 20;
const MAX_X_CORNERS: u64 = 1 << 24;
const MAX_PIXELS: u64 = 4096 * 4096;

fn bad(what: String) -> BuildError {
    BuildError::BadTable(format!("TACTICAL.DLL: {what}"))
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

impl Vec3 {
    fn finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }

    fn sub(self, o: Self) -> Self {
        Self { x: self.x - o.x, y: self.y - o.y, z: self.z - o.z }
    }

    fn cross(self, o: Self) -> Self {
        Self {
            x: self.y * o.z - self.z * o.y,
            y: self.z * o.x - self.x * o.z,
            z: self.x * o.y - self.y * o.x,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
struct Material {
    diffuse: [f32; 4],
    specular_exponent: f32,
    specular: [f32; 3],
    emissive: [f32; 3],
    texture: String,
}

#[derive(Debug, Clone, Default)]
struct Mesh {
    positions: Vec<Vec3>,
    faces: Vec<Vec<u32>>,
    normals: Vec<Vec3>,
    normal_faces: Vec<Vec<u32>>,
    uvs: Vec<[f32; 2]>,
    materials: Vec<Material>,
    face_materials: Vec<u32>,
    bounds_min: Vec3,
    bounds_max: Vec3,
}

#[derive(Debug, Clone, Copy)]
struct Vertex {
    position: Vec3,
    normal: Vec3,
    uv: [f32; 2],
}

#[derive(Debug, Clone, Default)]
struct Chunk {
    material: u32,
    vertices: Vec<Vertex>,
    indices: Vec<u16>,
}

// Binary .X tokens (`tactical3d_decode.go`).
const TOKEN_NAME: u16 = 1;
const TOKEN_STRING: u16 = 2;
const TOKEN_INTEGER: u16 = 3;
const TOKEN_GUID: u16 = 5;
const TOKEN_INTEGER_LIST: u16 = 6;
const TOKEN_FLOAT_LIST: u16 = 7;
const TOKEN_OPEN: u16 = 10;
const TOKEN_CLOSE: u16 = 11;
const TOKEN_COMMA: u16 = 19;
const TOKEN_SEMICOLON: u16 = 20;

#[derive(Debug, Default)]
struct Token {
    kind: u16,
    text: String,
    ints: Vec<u32>,
    floats: Vec<f32>,
}

struct XReader<'a> {
    data: &'a [u8],
    at: usize,
}

impl XReader<'_> {
    fn u16(&mut self) -> Result<u16, BuildError> {
        let b = self.data.get(self.at..self.at + 2).ok_or_else(|| bad(format!("binary X ends at byte {}", self.at)))?;
        self.at += 2;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    fn u32(&mut self) -> Result<u32, BuildError> {
        let b = self.data.get(self.at..self.at + 4).ok_or_else(|| bad(format!("binary X ends at byte {}", self.at)))?;
        self.at += 4;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn next(&mut self) -> Result<Token, BuildError> {
        let kind = self.u16()?;
        let mut token = Token { kind, ..Token::default() };
        match kind {
            TOKEN_NAME | TOKEN_STRING => {
                let len = self.u32()?;
                if len == 0 || len > MAX_X_STRING || len as usize > self.data.len() - self.at {
                    return Err(bad(format!("invalid binary X string length {len}")));
                }
                let text = &self.data[self.at..self.at + len as usize];
                self.at += len as usize;
                if text.iter().any(|&b| !(0x20..0x7f).contains(&b)) {
                    return Err(bad("binary X string contains non-ASCII or control data".into()));
                }
                token.text = String::from_utf8_lossy(text).into_owned();
                if kind == TOKEN_STRING {
                    let end = self.u16()?;
                    if end != TOKEN_SEMICOLON && end != TOKEN_COMMA {
                        return Err(bad(format!("binary X string has invalid terminator {end}")));
                    }
                }
            }
            TOKEN_INTEGER => token.ints = vec![self.u32()?],
            TOKEN_GUID => {
                if self.at + 16 > self.data.len() {
                    return Err(bad("truncated binary X GUID".into()));
                }
                self.at += 16;
            }
            TOKEN_INTEGER_LIST | TOKEN_FLOAT_LIST => {
                let count = self.u32()?;
                if u64::from(count) > MAX_X_CORNERS || u64::from(count) * 4 > (self.data.len() - self.at) as u64 {
                    return Err(bad(format!("invalid binary X list count {count}")));
                }
                for _ in 0..count {
                    let value = self.u32()?;
                    if kind == TOKEN_INTEGER_LIST {
                        token.ints.push(value);
                    } else {
                        token.floats.push(f32::from_bits(value));
                    }
                }
            }
            TOKEN_OPEN | TOKEN_CLOSE => {}
            other => return Err(bad(format!("unsupported binary X token {other} at byte {}", self.at - 2))),
        }
        Ok(token)
    }

    fn expect(&mut self, kind: u16) -> Result<(), BuildError> {
        let token = self.next()?;
        if token.kind == kind {
            Ok(())
        } else {
            Err(bad(format!("expected token {kind}, got {} {:?}", token.kind, token.text)))
        }
    }

    fn expect_object(&mut self, name: &str) -> Result<(), BuildError> {
        let token = self.next()?;
        if token.kind != TOKEN_NAME || token.text != name {
            return Err(bad(format!("expected {name} object, got {} {:?}", token.kind, token.text)));
        }
        self.expect(TOKEN_OPEN)
    }

    fn integers(&mut self) -> Result<Vec<u32>, BuildError> {
        let token = self.next()?;
        if token.kind == TOKEN_INTEGER_LIST || token.kind == TOKEN_INTEGER {
            Ok(token.ints)
        } else {
            Err(bad(format!("expected integer data, got token {}", token.kind)))
        }
    }

    fn floats(&mut self) -> Result<Vec<f32>, BuildError> {
        let token = self.next()?;
        if token.kind == TOKEN_FLOAT_LIST {
            Ok(token.floats)
        } else {
            Err(bad(format!("expected float list, got token {}", token.kind)))
        }
    }
}

fn vec3s(values: &[f32], what: &str) -> Result<Vec<Vec3>, BuildError> {
    values
        .as_chunks::<3>()
        .0
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let v = Vec3 { x: c[0], y: c[1], z: c[2] };
            if v.finite() { Ok(v) } else { Err(bad(format!("{what} {i} is not finite"))) }
        })
        .collect()
}

/// A face list: a count, then each face's corner count and corners
/// (`decodeTacticalFaces`).
fn decode_faces(values: &[u32], vertex_count: u32, label: &str) -> Result<Vec<Vec<u32>>, BuildError> {
    let Some(&count) = values.first().filter(|&&c| c != 0 && c <= MAX_X_FACES) else {
        return Err(bad(format!("{label}: invalid face count")));
    };
    let mut faces = Vec::with_capacity(count as usize);
    let mut at = 1;
    let mut corners_total = 0u64;
    for face in 0..count {
        let corners = *values.get(at).ok_or_else(|| bad(format!("{label}: face {face} is missing its corner count")))? as usize;
        at += 1;
        if corners < 3 || corners > values.len() - at {
            return Err(bad(format!("{label}: face {face} has invalid corner count {corners}")));
        }
        corners_total += corners as u64;
        if corners_total > MAX_X_CORNERS {
            return Err(bad(format!("{label}: face corners exceed the limit")));
        }
        let list = values[at..at + corners].to_vec();
        if list.iter().any(|&i| i >= vertex_count) {
            return Err(bad(format!("{label}: face {face} indexes past {vertex_count} vertices")));
        }
        faces.push(list);
        at += corners;
    }
    if at != values.len() {
        return Err(bad(format!("{label}: {} unused face-list values", values.len() - at)));
    }
    Ok(faces)
}

fn decode_materials(reader: &mut XReader<'_>, mesh: &mut Mesh) -> Result<(), BuildError> {
    reader.expect_object("MeshMaterialList")?;
    let values = reader.integers()?;
    if values.len() < 3 || values[0] == 0 || values[0] > 256 || values[1] == 0 {
        return Err(bad(format!("MeshMaterialList: invalid header {values:?}")));
    }
    let (materials, face_materials) = (values[0] as usize, values[1] as usize);
    if values.len() != 2 + face_materials || (face_materials != 1 && face_materials != mesh.faces.len()) {
        return Err(bad("MeshMaterialList: face-material indices do not match the faces".into()));
    }
    mesh.face_materials = values[2..].to_vec();
    if face_materials == 1 {
        mesh.face_materials.resize(mesh.faces.len(), values[2]);
    }
    if mesh.face_materials.iter().any(|&m| m as usize >= materials) {
        return Err(bad("MeshMaterialList: a face references a missing material".into()));
    }
    for index in 0..materials {
        reader.expect_object("Material")?;
        let f = reader.floats()?;
        if f.len() != 11 || f.iter().any(|v| !v.is_finite()) {
            return Err(bad(format!("Material {index}: expected 11 finite values")));
        }
        let mut material = Material {
            diffuse: [f[0], f[1], f[2], f[3]],
            specular_exponent: f[4],
            specular: [f[5], f[6], f[7]],
            emissive: [f[8], f[9], f[10]],
            texture: String::new(),
        };
        let mut next = reader.next()?;
        if next.kind == TOKEN_NAME && (next.text == "TextureFilename" || next.text == "TextureFileName") {
            reader.expect(TOKEN_OPEN)?;
            let texture = reader.next()?;
            if texture.kind != TOKEN_STRING || matches!(texture.text.as_str(), "." | "..") || texture.text.contains(['/', '\\']) {
                return Err(bad(format!("Material {index}: invalid TextureFilename")));
            }
            material.texture = texture.text;
            reader.expect(TOKEN_CLOSE)?;
            next = reader.next()?;
        }
        if next.kind != TOKEN_CLOSE {
            return Err(bad(format!("Material {index}: expected closing brace, got token {}", next.kind)));
        }
        mesh.materials.push(material);
    }
    reader.expect(TOKEN_CLOSE)
}

/// A binary DirectX mesh (`decodeTacticalBinaryX`).
fn decode_mesh(data: &[u8]) -> Result<Mesh, BuildError> {
    let header = data.get(..16).ok_or_else(|| bad("binary X file is shorter than its header".into()))?;
    if header != b"xof 0302bin 0032" && header != b"xof 0303bin 0032" {
        return Err(bad(format!("unsupported binary X header {:?}", String::from_utf8_lossy(header))));
    }
    let mut reader = XReader { data, at: 16 };
    reader.expect_object("Header")?;
    if reader.integers()? != [1, 0, 0] {
        return Err(bad("Header: unsupported values".into()));
    }
    reader.expect(TOKEN_CLOSE)?;
    reader.expect_object("Mesh")?;
    let count = reader.integers()?;
    let vertex_count = match count.as_slice() {
        [n] if *n != 0 && *n <= MAX_X_VERTICES => *n as usize,
        _ => return Err(bad(format!("Mesh: invalid vertex count {count:?}"))),
    };
    let positions = reader.floats()?;
    if positions.len() != vertex_count * 3 {
        return Err(bad("Mesh: position count does not match the vertices".into()));
    }
    let mut mesh = Mesh { positions: vec3s(&positions, "position")?, ..Mesh::default() };
    let (mut lo, mut hi) = (mesh.positions[0], mesh.positions[0]);
    for p in &mesh.positions[1..] {
        // Go's min/max on floats.
        lo = Vec3 { x: lo.x.min(p.x), y: lo.y.min(p.y), z: lo.z.min(p.z) };
        hi = Vec3 { x: hi.x.max(p.x), y: hi.y.max(p.y), z: hi.z.max(p.z) };
    }
    (mesh.bounds_min, mesh.bounds_max) = (lo, hi);
    let faces = reader.integers()?;
    mesh.faces = decode_faces(&faces, vertex_count as u32, "Mesh")?;
    decode_materials(&mut reader, &mut mesh)?;
    let mut next = reader.next()?;
    if next.kind == TOKEN_NAME && next.text == "MeshNormals" {
        reader.expect(TOKEN_OPEN)?;
        let count = reader.integers()?;
        let normal_count = match count.as_slice() {
            [n] if *n != 0 && *n <= MAX_X_VERTICES => *n as usize,
            _ => return Err(bad(format!("MeshNormals: invalid count {count:?}"))),
        };
        let normals = reader.floats()?;
        if normals.len() != normal_count * 3 {
            return Err(bad("MeshNormals: value count does not match".into()));
        }
        mesh.normals = vec3s(&normals, "normal")?;
        let normal_faces = reader.integers()?;
        mesh.normal_faces = decode_faces(&normal_faces, normal_count as u32, "MeshNormals")?;
        if mesh.normal_faces.len() != mesh.faces.len()
            || mesh.normal_faces.iter().zip(&mesh.faces).any(|(n, f)| n.len() != f.len())
        {
            return Err(bad("MeshNormals: faces do not match the mesh's".into()));
        }
        reader.expect(TOKEN_CLOSE)?;
        next = reader.next()?;
    }
    if next.kind != TOKEN_NAME || next.text != "MeshTextureCoords" {
        return Err(bad(format!("expected MeshTextureCoords, got token {}", next.kind)));
    }
    reader.expect(TOKEN_OPEN)?;
    if reader.integers()? != [vertex_count as u32] {
        return Err(bad("MeshTextureCoords: count does not match the vertices".into()));
    }
    let uvs = reader.floats()?;
    if uvs.len() != vertex_count * 2 || uvs.iter().any(|v| !v.is_finite()) {
        return Err(bad("MeshTextureCoords: invalid coordinates".into()));
    }
    mesh.uvs = uvs.as_chunks::<2>().0.to_vec();
    reader.expect(TOKEN_CLOSE)?;
    reader.expect(TOKEN_CLOSE)?;
    if reader.at != data.len() {
        return Err(bad(format!("binary X file has {} trailing bytes", data.len() - reader.at)));
    }
    Ok(mesh)
}

/// A face's normal from its first non-degenerate corner fan
/// (`tacticalFaceNormal`): the length in f64 of the f32 sum of squares.
fn face_normal(positions: &[Vec3], face: &[u32]) -> Option<Vec3> {
    let origin = positions[face[0] as usize];
    for corner in 1..face.len() - 1 {
        let left = positions[face[corner] as usize].sub(origin);
        let right = positions[face[corner + 1] as usize].sub(origin);
        let n = left.cross(right);
        let length = f64::from(n.x * n.x + n.y * n.y + n.z * n.z).sqrt();
        if length > 1e-12 && length.is_finite() {
            let scale = (1.0 / length) as f32;
            return Some(Vec3 { x: n.x * scale, y: n.y * scale, z: n.z * scale });
        }
    }
    None
}

/// The mesh's triangles, fanned per face and grouped in runs of one
/// material of at most 65,536 vertices (`buildTacticalMeshChunks`).
fn build_chunks(mesh: &Mesh) -> Result<Vec<Chunk>, BuildError> {
    let mut chunks: Vec<Chunk> = Vec::with_capacity(mesh.materials.len());
    for (face_index, face) in mesh.faces.iter().enumerate() {
        let material = mesh.face_materials[face_index];
        let face_normal = face_normal(&mesh.positions, face);
        if face_normal.is_none() && mesh.normals.is_empty() {
            return Err(bad(format!("face {face_index}: degenerate face has no usable normal")));
        }
        for corner in 1..face.len() - 1 {
            let start_new = chunks
                .last()
                .is_none_or(|c| c.material != material || c.vertices.len() + 3 > usize::from(u16::MAX) + 1);
            if start_new {
                chunks.push(Chunk { material, ..Chunk::default() });
            }
            let chunk = chunks.last_mut().expect("a chunk was just ensured");
            for face_corner in [0, corner, corner + 1] {
                let position = face[face_corner] as usize;
                let normal = if mesh.normals.is_empty() {
                    face_normal.unwrap_or_default()
                } else {
                    mesh.normals[mesh.normal_faces[face_index][face_corner] as usize]
                };
                chunk.vertices.push(Vertex { position: mesh.positions[position], normal, uv: mesh.uvs[position] });
                chunk.indices.push((chunk.vertices.len() - 1) as u16);
            }
        }
    }
    Ok(chunks)
}

fn put_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put_f32(out: &mut Vec<u8>, value: f32) {
    put_u32(out, value.to_bits());
}

/// An `ORTMESH` object (`encodeTacticalMeshObject`).
fn encode_mesh(mesh: &Mesh, chunks: &[Chunk]) -> Vec<u8> {
    let mut out = b"ORTMESH\0".to_vec();
    for value in [
        1,
        mesh.materials.len() as u32,
        chunks.len() as u32,
        mesh.positions.len() as u32,
        mesh.faces.len() as u32,
        u32::from(mesh.normals.is_empty()),
    ] {
        put_u32(&mut out, value);
    }
    let (lo, hi) = (mesh.bounds_min, mesh.bounds_max);
    for value in [lo.x, lo.y, lo.z, hi.x, hi.y, hi.z] {
        put_f32(&mut out, value);
    }
    for material in &mesh.materials {
        for &value in material.diffuse.iter().chain([&material.specular_exponent]).chain(&material.specular).chain(&material.emissive) {
            put_f32(&mut out, value);
        }
        put_u32(&mut out, material.texture.len() as u32);
        out.extend_from_slice(material.texture.as_bytes());
    }
    for chunk in chunks {
        put_u32(&mut out, chunk.material);
        put_u32(&mut out, chunk.vertices.len() as u32);
        put_u32(&mut out, chunk.indices.len() as u32);
        for v in &chunk.vertices {
            for value in [v.position.x, v.position.y, v.position.z, v.normal.x, v.normal.y, v.normal.z, v.uv[0], v.uv[1]] {
                put_f32(&mut out, value);
            }
        }
        for &index in &chunk.indices {
            out.extend_from_slice(&index.to_le_bytes());
        }
    }
    out
}

/// A type-303 image: width, height, then (run, colour) pairs, a run of 0
/// meaning 256; any bytes after the last pixel are kept
/// (`decodeTacticalIndexedImage`, `encodeTacticalIndexObject`).
fn encode_image(data: &[u8], palette_id: u32, palette_rule: u32, tail_allowed: bool) -> Result<Vec<u8>, BuildError> {
    if data.len() < 10 {
        return Err(bad("type-303 image is shorter than its header and first run".into()));
    }
    let width = u32::from_le_bytes(data[0..4].try_into().expect("four bytes"));
    let height = u32::from_le_bytes(data[4..8].try_into().expect("four bytes"));
    if width == 0 || height == 0 || u64::from(width) * u64::from(height) > MAX_PIXELS {
        return Err(bad(format!("type-303 image has invalid dimensions {width}x{height}")));
    }
    let count = (width * height) as usize;
    let mut pixels = Vec::with_capacity(count);
    let mut at = 8;
    while pixels.len() < count {
        let [run, colour] = data.get(at..at + 2).map(|b| [b[0], b[1]]).ok_or_else(|| bad("type-303 image ends early".into()))?;
        at += 2;
        let run = if run == 0 { 256 } else { usize::from(run) };
        if run > count - pixels.len() {
            return Err(bad("type-303 run exceeds the image".into()));
        }
        pixels.resize(pixels.len() + run, colour);
    }
    let tail = &data[at..];
    if !tail.is_empty() && !tail_allowed {
        return Err(bad(format!("type-303 image has {} unsupported trailing bytes", tail.len())));
    }
    let mut out = b"ORTINDEX".to_vec();
    for value in [1, width, height, palette_id, palette_rule, pixels.len() as u32, tail.len() as u32] {
        put_u32(&mut out, value);
    }
    out.extend_from_slice(&pixels);
    out.extend_from_slice(tail);
    Ok(out)
}

/// TACTICAL.DLL's meshes (kind 4, `"{id}/{language}"`) and textures and
/// palettes (kind 5, `"{id or name}/{language}"`).
///
/// # Errors
/// A missing or unreadable TACTICAL.DLL, counts other than the original
/// release's, or a resource the converter refuses.
pub fn tactical_entries(install: &InstallFiles, out: &mut Vec<Entry>) -> Result<(), BuildError> {
    let dll = install.get("TACTICAL.DLL").ok_or_else(|| BuildError::Missing("TACTICAL.DLL".into()))?;
    let resources = pe::resources(dll).map_err(|error| BuildError::Pe { file: "TACTICAL.DLL".into(), error })?;
    let (mut meshes, mut textures) = (0, 0);
    for resource in &resources {
        let language = resource.language;
        if resource.data.is_empty() {
            return Err(bad("an empty tactical resource".into()));
        }
        match resource.kind {
            ResId::Id(MESH_TYPE) => {
                let ResId::Id(id) = resource.name else {
                    return Err(bad("a type-301 mesh must use a numeric id".into()));
                };
                let mesh = decode_mesh(resource.data).map_err(|e| bad(format!("type-301 mesh {id}: {e}")))?;
                if mesh.normals.is_empty() && id != MESH_WITHOUT_NORMALS {
                    return Err(bad(format!("type-301 mesh {id} has unsupported missing normals")));
                }
                let chunks = build_chunks(&mesh)?;
                out.push(Entry { kind: KIND_TACTICAL_MESH, key: format!("{id}/{language}"), data: encode_mesh(&mesh, &chunks) });
                meshes += 1;
            }
            ResId::Id(TEXTURE_TYPE) => {
                let (identifier, data) = match &resource.name {
                    ResId::Id(id) if PALETTES.contains(id) => {
                        if resource.data.len() != 768 {
                            return Err(bad(format!("type-303 palette {id} has {} bytes, expected 768", resource.data.len())));
                        }
                        let mut data = b"ORTPAL00".to_vec();
                        put_u32(&mut data, 1);
                        put_u32(&mut data, *id);
                        data.extend_from_slice(resource.data);
                        (id.to_string(), data)
                    }
                    ResId::Id(id) => {
                        // Planet images take their pair's palette; the rest
                        // the battle's active one (rule codes 2 and 1).
                        let (palette, rule) = if PLANET_IMAGES.contains(id) { (id + 30, 2) } else { (0, 1) };
                        let tail = IMAGES_WITH_TAILS.contains(id) && language == 1033;
                        (id.to_string(), encode_image(resource.data, palette, rule, tail)?)
                    }
                    ResId::Name(name) => {
                        if name.is_empty() || name == "." || name == ".." || name.contains(['/', '\\']) || name.chars().any(char::is_control) {
                            return Err(bad(format!("unsafe tactical resource name {name:?}")));
                        }
                        (name.clone(), encode_image(resource.data, 0, 1, false)?)
                    }
                };
                out.push(Entry { kind: KIND_TACTICAL_TEXTURE, key: format!("{identifier}/{language}"), data });
                textures += 1;
            }
            _ => {}
        }
    }
    if meshes != MESH_COUNT || textures != TEXTURE_COUNT {
        return Err(bad(format!("found {meshes} meshes and {textures} textures, expected {MESH_COUNT} and {TEXTURE_COUNT}")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Binary X tokens: each a little-endian kind word, then its payload.
    fn x(parts: &[&[u8]]) -> Vec<u8> {
        parts.concat()
    }

    fn word(value: u16) -> Vec<u8> {
        value.to_le_bytes().to_vec()
    }

    fn dword(value: u32) -> Vec<u8> {
        value.to_le_bytes().to_vec()
    }

    fn read(data: &[u8]) -> Result<Token, BuildError> {
        XReader { data, at: 0 }.next()
    }

    #[test]
    fn a_name_token_must_fit_and_be_printable() {
        let name = x(&[&word(TOKEN_NAME), &dword(4), b"Mesh"]);
        assert_eq!(read(&name).unwrap().text, "Mesh");
        assert!(read(&x(&[&word(TOKEN_NAME), &dword(0)])).is_err(), "empty");
        assert!(read(&x(&[&word(TOKEN_NAME), &dword(5), b"Mesh"])).is_err(), "longer than the data");
        assert!(read(&x(&[&word(TOKEN_NAME), &dword(4), b"Me\x01h"])).is_err(), "control byte");
        let long = vec![b'a'; MAX_X_STRING as usize + 1];
        assert!(read(&x(&[&word(TOKEN_NAME), &dword(MAX_X_STRING + 1), &long])).is_err(), "over the cap");
        let most = vec![b'a'; MAX_X_STRING as usize];
        assert!(read(&x(&[&word(TOKEN_NAME), &dword(MAX_X_STRING), &most])).is_ok(), "at the cap");
    }

    #[test]
    fn a_string_token_ends_with_a_semicolon_or_a_comma() {
        for (end, ok) in [(TOKEN_SEMICOLON, true), (TOKEN_COMMA, true), (TOKEN_OPEN, false)] {
            let data = x(&[&word(TOKEN_STRING), &dword(2), b"ab", &word(end)]);
            assert_eq!(read(&data).is_ok(), ok, "terminator {end}");
        }
    }

    #[test]
    fn a_guid_is_skipped_whole_and_a_short_one_is_refused() {
        let data = x(&[&word(TOKEN_GUID), &[7; 16], &word(TOKEN_OPEN)]);
        let mut reader = XReader { data: &data, at: 0 };
        assert_eq!(reader.next().unwrap().kind, TOKEN_GUID);
        assert_eq!(reader.at, 18);
        assert_eq!(reader.next().unwrap().kind, TOKEN_OPEN);
        assert!(read(&x(&[&word(TOKEN_GUID), &[7; 15]])).is_err());
        assert!(read(&x(&[&word(TOKEN_GUID), &[7; 16]])).is_ok(), "a GUID ending the data");
    }

    #[test]
    fn lists_read_their_count_of_values_and_no_more_than_the_data_holds() {
        let ints = x(&[&word(TOKEN_INTEGER_LIST), &dword(2), &dword(5), &dword(9)]);
        assert_eq!(read(&ints).unwrap().ints, vec![5, 9]);
        let trailing = x(&[&ints, &word(TOKEN_CLOSE)]);
        assert_eq!(read(&trailing).unwrap().ints, vec![5, 9], "data left after the list");
        let floats = x(&[&word(TOKEN_FLOAT_LIST), &dword(1), &1.5f32.to_bits().to_le_bytes()]);
        assert_eq!(read(&floats).unwrap().floats, vec![1.5]);
        assert!(read(&x(&[&word(TOKEN_INTEGER_LIST), &dword(3), &dword(5), &dword(9)])).is_err());
        let over = u32::try_from(MAX_X_CORNERS + 1).unwrap();
        assert!(read(&x(&[&word(TOKEN_INTEGER_LIST), &dword(over)])).is_err());
        assert!(read(&x(&[&word(TOKEN_INTEGER), &dword(4)])).unwrap().ints == vec![4]);
        assert!(read(&word(99)).is_err(), "an unknown token");
    }

    #[test]
    fn objects_and_integers_are_checked_by_kind() {
        let object = x(&[&word(TOKEN_NAME), &dword(4), b"Mesh", &word(TOKEN_OPEN)]);
        assert!(XReader { data: &object, at: 0 }.expect_object("Mesh").is_ok());
        assert!(XReader { data: &object, at: 0 }.expect_object("Frame").is_err());
        let string = x(&[&word(TOKEN_STRING), &dword(4), b"Mesh", &word(TOKEN_COMMA), &word(TOKEN_OPEN)]);
        assert!(XReader { data: &string, at: 0 }.expect_object("Mesh").is_err(), "a string is not a name");
        let one = x(&[&word(TOKEN_INTEGER), &dword(4)]);
        assert_eq!(XReader { data: &one, at: 0 }.integers().unwrap(), vec![4]);
        let list = x(&[&word(TOKEN_INTEGER_LIST), &dword(1), &dword(6)]);
        assert_eq!(XReader { data: &list, at: 0 }.integers().unwrap(), vec![6]);
        let floats = x(&[&word(TOKEN_FLOAT_LIST), &dword(2), &2.5f32.to_bits().to_le_bytes(), &(-1.0f32).to_bits().to_le_bytes()]);
        assert!(XReader { data: &floats, at: 0 }.integers().is_err());
        assert_eq!(XReader { data: &floats, at: 0 }.floats().unwrap(), vec![2.5, -1.0]);
        assert!(XReader { data: &list, at: 0 }.floats().is_err());
        let close = word(TOKEN_CLOSE);
        assert!(XReader { data: &close, at: 0 }.expect(TOKEN_CLOSE).is_ok());
        assert!(XReader { data: &close, at: 0 }.expect(TOKEN_OPEN).is_err());
    }

    fn name(text: &str) -> Vec<u8> {
        x(&[&word(TOKEN_NAME), &dword(text.len() as u32), text.as_bytes()])
    }

    fn string(text: &str) -> Vec<u8> {
        x(&[&word(TOKEN_STRING), &dword(text.len() as u32), text.as_bytes(), &word(TOKEN_SEMICOLON)])
    }

    fn ints(values: &[u32]) -> Vec<u8> {
        let mut out = x(&[&word(TOKEN_INTEGER_LIST), &dword(values.len() as u32)]);
        values.iter().for_each(|v| out.extend(dword(*v)));
        out
    }

    fn floats(values: &[f32]) -> Vec<u8> {
        let mut out = x(&[&word(TOKEN_FLOAT_LIST), &dword(values.len() as u32)]);
        values.iter().for_each(|v| out.extend(dword(v.to_bits())));
        out
    }

    fn open() -> Vec<u8> {
        word(TOKEN_OPEN)
    }

    fn close() -> Vec<u8> {
        word(TOKEN_CLOSE)
    }

    /// A Material object: 11 values and, when given, a texture token.
    fn material(values: &[f32], texture: Option<Vec<u8>>) -> Vec<u8> {
        let texture = texture.map(|t| x(&[&name("TextureFilename"), &open(), &t, &close()])).unwrap_or_default();
        x(&[&name("Material"), &open(), &floats(values), &texture, &close()])
    }

    const PLAIN: [f32; 11] = [1.0; 11];

    fn material_list(header: &[u32], materials: &[Vec<u8>]) -> Vec<u8> {
        x(&[&name("MeshMaterialList"), &open(), &ints(header), &materials.concat(), &close()])
    }

    /// A why of a refusal.
    fn why<T: std::fmt::Debug>(result: Result<T, BuildError>) -> String {
        match result {
            Err(BuildError::BadTable(why)) => why,
            other => panic!("{other:?}"),
        }
    }

    fn materials_for(faces: usize, data: &[u8]) -> Result<Mesh, BuildError> {
        let mut mesh = Mesh { faces: vec![vec![0, 1, 2]; faces], ..Mesh::default() };
        decode_materials(&mut XReader { data, at: 0 }, &mut mesh)?;
        Ok(mesh)
    }

    #[test]
    fn a_material_list_names_one_to_256_materials_and_its_faces_ones() {
        let one = vec![material(&PLAIN, None)];
        assert_eq!(materials_for(2, &material_list(&[1, 1, 0], &one)).unwrap().face_materials, [0, 0], "one for all");
        assert_eq!(materials_for(2, &material_list(&[1, 2, 0, 0], &one)).unwrap().face_materials, [0, 0]);
        for header in [&[1, 1][..], &[0, 1, 0], &[257, 1, 0], &[1, 0, 0]] {
            assert!(why(materials_for(1, &material_list(header, &one))).contains("invalid header"), "{header:?}");
        }
        let all = vec![material(&PLAIN, None); 256];
        assert_eq!(materials_for(1, &material_list(&[256, 1, 255], &all)).unwrap().materials.len(), 256);
        for header in [&[1, 2, 0, 0][..], &[1, 1, 0, 0]] {
            assert!(why(materials_for(3, &material_list(header, &one))).contains("do not match the faces"), "{header:?}");
        }
        assert!(why(materials_for(1, &material_list(&[1, 1, 1], &one))).contains("missing material"));
    }

    #[test]
    fn a_material_is_eleven_finite_values_and_an_optional_plain_texture_name() {
        let list = |m: Vec<u8>| materials_for(1, &material_list(&[1, 1, 0], &[m]));
        assert!(why(list(material(&PLAIN[..10], None))).contains("11 finite values"));
        let mut nan = PLAIN;
        nan[4] = f32::NAN;
        assert!(why(list(material(&nan, None))).contains("11 finite values"));
        assert_eq!(list(material(&PLAIN, Some(string("a.bmp")))).unwrap().materials[0].texture, "a.bmp");
        let other = x(&[&name("Material"), &open(), &floats(&PLAIN), &name("TextureFileName"), &open(), &string("b.bmp"),
            &close(), &close()]);
        assert_eq!(list(other).unwrap().materials[0].texture, "b.bmp", "either spelling");
        for texture in [name("a.bmp"), string("."), string(".."), string("a/b"), string("a\\b")] {
            assert!(why(list(material(&PLAIN, Some(texture)))).contains("invalid TextureFilename"));
        }
        // A string that reads TextureFilename is not the texture object.
        let string_token = x(&[&name("Material"), &open(), &floats(&PLAIN), &string("TextureFilename"), &open(),
            &string("c.bmp"), &close(), &close()]);
        assert!(why(list(string_token)).contains("expected closing brace"));
    }

    /// A one-triangle binary X mesh with `normals` and `coords` objects.
    fn mesh_file(count: &[u32], positions: &[f32], normals: &[u8], coords: &[u8]) -> Vec<u8> {
        x(&[b"xof 0302bin 0032", &name("Header"), &open(), &ints(&[1, 0, 0]), &close(), &name("Mesh"), &open(),
            &ints(count), &floats(positions), &ints(&[1, 3, 0, 1, 2]),
            &material_list(&[1, 1, 0], &[material(&PLAIN, None)]), normals, coords, &close()])
    }

    const TRIANGLE: [f32; 9] = [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];

    fn coords(values: &[f32]) -> Vec<u8> {
        x(&[&name("MeshTextureCoords"), &open(), &ints(&[3]), &floats(values), &close()])
    }

    fn normals(count: u32, faces: &[u32]) -> Vec<u8> {
        x(&[&name("MeshNormals"), &open(), &ints(&[count]), &floats(&vec![0.0; count as usize * 3]), &ints(faces), &close()])
    }

    #[test]
    fn a_mesh_reads_its_vertices_faces_normals_and_coordinates() {
        let uv = coords(&[0.0; 6]);
        let mesh = decode_mesh(&mesh_file(&[3], &TRIANGLE, &normals(1, &[1, 3, 0, 0, 0]), &uv)).unwrap();
        assert_eq!((mesh.positions.len(), mesh.faces.len(), mesh.normals.len()), (3, 1, 1));
        assert_eq!(mesh.bounds_max, Vec3 { x: 1.0, y: 1.0, z: 0.0 });
        assert!(decode_mesh(&mesh_file(&[3], &TRIANGLE, &[], &uv)).unwrap().normals.is_empty());
        for count in [0, MAX_X_VERTICES + 1] {
            assert!(why(decode_mesh(&mesh_file(&[count], &TRIANGLE, &[], &uv))).contains("invalid vertex count"), "{count}");
        }
        for count in [0, MAX_X_VERTICES + 1] {
            let odd = x(&[&name("MeshNormals"), &open(), &ints(&[count]), &floats(&[0.0; 3]), &ints(&[1, 3, 0, 0, 0]), &close()]);
            assert!(why(decode_mesh(&mesh_file(&[3], &TRIANGLE, &odd, &uv))).contains("MeshNormals: invalid count"), "{count}");
        }
    }

    #[test]
    fn a_mesh_s_normal_faces_and_coordinates_must_match_its_own() {
        let uv = coords(&[0.0; 6]);
        let quad_normals = normals(1, &[1, 4, 0, 0, 0, 0]);
        assert!(why(decode_mesh(&mesh_file(&[3], &TRIANGLE, &quad_normals, &uv))).contains("faces do not match"));
        let two_faces = normals(1, &[2, 3, 0, 0, 0, 3, 0, 0, 0]);
        assert!(why(decode_mesh(&mesh_file(&[3], &TRIANGLE, &two_faces, &uv))).contains("faces do not match"));
        let renamed = x(&[&name("MeshTextureCoordz"), &open(), &ints(&[3]), &floats(&[0.0; 6]), &close()]);
        assert!(why(decode_mesh(&mesh_file(&[3], &TRIANGLE, &[], &renamed))).contains("expected MeshTextureCoords"));
        let as_string = x(&[&string("MeshTextureCoords"), &open(), &ints(&[3]), &floats(&[0.0; 6]), &close()]);
        assert!(why(decode_mesh(&mesh_file(&[3], &TRIANGLE, &[], &as_string))).contains("expected MeshTextureCoords"));
        let mut nan = [0.0; 6];
        nan[3] = f32::NAN;
        assert!(why(decode_mesh(&mesh_file(&[3], &TRIANGLE, &[], &coords(&nan)))).contains("invalid coordinates"));
        assert!(why(decode_mesh(&mesh_file(&[3], &TRIANGLE, &[], &coords(&[0.0; 4])))).contains("invalid coordinates"));
    }

    #[test]
    fn a_position_or_normal_with_a_non_finite_coordinate_is_refused() {
        for at in 0..3 {
            let mut values = [0.0; 3];
            values[at] = f32::INFINITY;
            assert!(vec3s(&values, "position").is_err(), "{at}");
        }
        assert!(vec3s(&[0.0; 3], "position").is_ok());
    }

    #[test]
    fn a_face_list_counts_one_to_a_million_faces_and_corners_to_its_limit() {
        assert!(why(decode_faces(&[0], 3, "t")).contains("invalid face count"));
        assert!(why(decode_faces(&[MAX_X_FACES + 1, 3, 0, 1, 2], 3, "t")).contains("invalid face count"));
        assert!(why(decode_faces(&[1, 4, 0, 1, 2], 3, "t")).contains("invalid corner count 4"));
        // Two faces: the second's corners push the total past the limit.
        let big = MAX_X_CORNERS as u32 - 3;
        let mut values = vec![2, 3, 0, 1, 2, big];
        values.resize(values.len() + big as usize, 0);
        assert_eq!(decode_faces(&values, 3, "t").unwrap().len(), 2, "at the limit, 2^24 corners");
        let mut over = vec![2, 3, 0, 1, 2, big + 1];
        over.resize(over.len() + big as usize + 1, 0);
        assert!(why(decode_faces(&over, 3, "t")).contains("exceed the limit"));
    }

    #[test]
    fn a_degenerate_face_has_no_normal_and_the_fan_tries_the_next_corner() {
        let p = |x, y, z| Vec3 { x, y, z };
        let flat = [p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0), p(2.0, 0.0, 0.0), p(0.0, 1.0, 0.0)];
        assert_eq!(face_normal(&flat, &[0, 1, 2]), None, "collinear");
        assert_eq!(face_normal(&flat, &[0, 1, 2, 3]), Some(p(0.0, 0.0, 1.0)));
        let huge = [p(0.0, 0.0, 0.0), p(1e30, 0.0, 0.0), p(0.0, 1e30, 0.0)];
        assert_eq!(face_normal(&huge, &[0, 1, 2]), None, "an infinite length");
    }

    #[test]
    fn chunks_hold_one_material_and_at_most_65536_vertices() {
        let p = |x: f32, y: f32| Vec3 { x, y, z: 0.0 };
        // A fan of 21,846 triangles: 21,845 fill 65,535 vertices.
        let corners = 21_848;
        let positions: Vec<_> = (0..corners).map(|i| p((i as f32).cos(), (i as f32).sin())).collect();
        let mesh = Mesh {
            uvs: vec![[0.0; 2]; corners],
            faces: vec![(0..corners as u32).collect()],
            normals: vec![p(0.0, 0.0)],
            normal_faces: vec![vec![0; corners]],
            positions,
            materials: vec![Material::default(); 2],
            face_materials: vec![1],
            ..Mesh::default()
        };
        let chunks = build_chunks(&mesh).unwrap();
        assert_eq!(chunks.iter().map(|c| c.vertices.len()).collect::<Vec<_>>(), [65_535, 3]);
        assert!(chunks.iter().all(|c| c.material == 1));
        let two = Mesh {
            positions: vec![p(0.0, 0.0), p(1.0, 0.0), p(0.0, 1.0)],
            faces: vec![vec![0, 1, 2]; 3],
            uvs: vec![[0.0; 2]; 3],
            materials: vec![Material::default(); 2],
            face_materials: vec![0, 1, 1],
            ..Mesh::default()
        };
        let chunks = build_chunks(&two).unwrap();
        assert_eq!(chunks.iter().map(|c| (c.material, c.vertices.len())).collect::<Vec<_>>(), [(0, 3), (1, 6)]);
    }

    #[test]
    fn a_type_303_image_is_at_most_4096_square_and_its_runs_stay_inside_it() {
        let image = |w: u32, h: u32, runs: &[u8]| x(&[&dword(w), &dword(h), runs]);
        for (w, h) in [(0, 1), (1, 0), (4097, 4096)] {
            assert!(why(encode_image(&image(w, h, &[1, 0]), 0, 1, false)).contains("invalid dimensions"), "{w}x{h}");
        }
        let full: Vec<u8> = [0u8, 3].repeat(65_536);
        assert_eq!(encode_image(&image(4096, 4096, &full), 0, 1, false).unwrap().len(), 36 + 4096 * 4096);
        assert!(why(encode_image(&image(2, 1, &[3, 0]), 0, 1, false)).contains("run exceeds"));
        assert_eq!(encode_image(&image(2, 1, &[1, 5, 1, 6]), 0, 1, false).unwrap()[36..], [5, 6]);
    }

    #[test]
    fn a_run_of_zero_paints_256_pixels_and_a_tail_is_refused_unless_allowed() {
        // 16x16: one run of 256 pixels of colour 7.
        let mut data = vec![16, 0, 0, 0, 16, 0, 0, 0, 0, 7];
        let object = encode_image(&data, 0, 1, false).unwrap();
        assert_eq!(&object[..8], b"ORTINDEX");
        assert_eq!(object.len(), 8 + 7 * 4 + 256);
        assert!(object[36..].iter().all(|&p| p == 7));
        data.extend_from_slice(&[1, 2, 3, 4]);
        assert!(encode_image(&data, 0, 1, false).is_err());
        assert_eq!(encode_image(&data, 0, 1, true).unwrap().len(), 8 + 7 * 4 + 256 + 4);
    }

    #[test]
    fn a_face_list_needs_three_corners_inside_the_vertices() {
        assert_eq!(decode_faces(&[1, 3, 0, 1, 2], 3, "t").unwrap(), vec![vec![0, 1, 2]]);
        assert!(decode_faces(&[1, 2, 0, 1], 3, "t").is_err());
        assert!(decode_faces(&[1, 3, 0, 1, 3], 3, "t").is_err());
        assert!(decode_faces(&[1, 3, 0, 1, 2, 9], 3, "t").is_err(), "unused values");
    }

    #[test]
    fn a_quad_fans_into_two_triangles_with_its_face_normal() {
        let p = |x, y| Vec3 { x, y, z: 0.0 };
        let mesh = Mesh {
            positions: vec![p(0.0, 0.0), p(1.0, 0.0), p(1.0, 1.0), p(0.0, 1.0)],
            faces: vec![vec![0, 1, 2, 3]],
            uvs: vec![[0.0; 2]; 4],
            materials: vec![Material::default()],
            face_materials: vec![0],
            ..Mesh::default()
        };
        let chunks = build_chunks(&mesh).unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].indices, [0, 1, 2, 3, 4, 5]);
        assert_eq!(chunks[0].vertices[4].position, p(1.0, 1.0));
        assert_eq!(chunks[0].vertices[0].normal, Vec3 { x: 0.0, y: 0.0, z: 1.0 });
    }
}
