use super::*;

fn assert_near(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 1.0e-5,
        "expected {expected}, got {actual}"
    );
}

// Source: d3dim.dll FUN_7403e4f0 ((c * 5 + 128) >> 8 per channel, cell
// r * 36 + g * 6 + b) and FUN_74029f67 / FUN_7402aba0 (cube colours
// level * 51 to the nearest palette entry, lowest index on ties).
#[test]
fn the_rgb_device_maps_colour_through_the_six_level_cube() {
    assert_eq!(original_rgb_cube_cell([0, 0, 0]), 0);
    assert_eq!(original_rgb_cube_cell([255, 255, 255]), 215);
    // 25 -> (125 + 128) >> 8 = 0; 26 -> 1; 77 -> 2.
    assert_eq!(original_rgb_cube_cell([25, 26, 77]), 6 + 2);
    assert_eq!(original_rgb_cube_cell([128, 0, 0]), 3 * 36);
    let mut palette = [[0_u8, 0, 0, 255]; 256];
    palette[1] = [255, 255, 255, 255];
    palette[2] = [100, 0, 0, 255];
    palette[3] = [104, 0, 0, 255];
    palette[4] = [255, 255, 255, 255];
    palette[5] = [0, 102, 0, 255];
    let cube = original_rgb_cube(&palette);
    // Cell (0, 2, 0) is (0, 102, 0): its green level picks entry 5.
    assert_eq!(cube[2 * 6], 5);
    assert_eq!(cube[0], 0);
    // White's two equal entries: the lower index.
    assert_eq!(cube[215], 1);
    // Cell (2, 0, 0) is (102, 0, 0): 100 and 104 tie at 4, so 2.
    assert_eq!(cube[2 * 36], 2);
    // Cell (3, 0, 0) is (153, 0, 0): 104 is nearer than 255.
    assert_eq!(cube[3 * 36], 3);
    // Cell (0, 0, 5) is (0, 0, 255): black (65025) is nearer than white (130050).
    assert_eq!(cube[5], 0);
}

#[test]
fn reader_rejects_non_finite_and_truncated_values() {
    let encoded_nan = f32::NAN.to_bits().to_le_bytes();
    let mut non_finite = Reader::new(&encoded_nan);
    assert!(non_finite.f32().is_err());
    let mut truncated = Reader::new(&[1, 2, 3]);
    assert!(truncated.u32().is_err());
}

// Our own conversion from the authored left-handed frame; no source claim.
#[test]
fn authored_mesh_positions_keep_source_scale_and_reflect_only_z() {
    assert_eq!(
        authored_position(vec3(12.5, -3.25, 8.0)),
        vec3(12.5, -3.25, -8.0)
    );
}

// Source: FUN_005ab0e0 installs the two radii from authored mesh bounds.
#[test]
fn collision_envelope_uses_source_y_radius_xz_diagonal_and_half_unit_floor() {
    let envelope = tactical_collision_envelope([-3.0, -2.0, -4.0, 5.0, 6.0, 2.0]);
    assert_near(envelope.vertical_radius, 4.0);
    assert_near(envelope.planar_diameter, 10.0);

    let floored = tactical_collision_envelope([0.0, 0.0, 0.0, 0.1, 0.2, 0.1]);
    assert_near(floored.vertical_radius, 0.5);
    assert_near(floored.planar_diameter, 0.5);
}

// Source: FUN_005ee590 constructs the retained projectile mesh and variants.
#[test]
fn retained_projectile_mesh_preserves_source_vertices_faces_and_longitudinal_scale() {
    let object = TacticalProjectileRenderObject {
        object_id: 1,
        origin: Vec3::ZERO,
        target: vec3(0.0, 0.0, 10.0),
        progress: 0.5,
        longitudinal_scale: 0.5,
        shape_variant: 1,
        color_selector: 0,
        color: RED,
    };
    let first = original_projectile_mesh(object, vec3(0.0, 0.0, 5.0));
    assert_eq!(first.vertices.len(), 12);
    assert_eq!(first.indices, ORIGINAL_PROJECTILE_VARIANT_1);
    assert_near(first.vertices[1].position.z, 5.5);
    assert_near(first.vertices[0].position.y, -0.001);

    let second = original_projectile_mesh(
        TacticalProjectileRenderObject {
            shape_variant: 2,
            ..object
        },
        Vec3::ZERO,
    );
    assert_eq!(second.indices, ORIGINAL_PROJECTILE_VARIANT_2);
    let third = original_projectile_mesh(
        TacticalProjectileRenderObject {
            shape_variant: 3,
            ..object
        },
        Vec3::ZERO,
    );
    assert_eq!(third.indices, ORIGINAL_PROJECTILE_VARIANT_3);
    assert_near(third.vertices[6].position.x, 0.1);
    assert_near(third.vertices[9].position.x, -0.1);
}

// Source: FUN_005d4d10 creates the directional light frame at (5,5,-1)
// and the 0.5 ambient light; d3drm packs the ambient as trunc(0.5 x 256)
// = 128 (FUN_6d83df1e); d3dim lights on a 0-255 scale and rounds and
// clamps the vertex colour before the texture (FUN_7402029b,
// 0x74025ff6), adding specular only when n.l > 0 (FUN_74020945).
// A0's 255 peaks are that specular, not an unclamped light.
// Source: d3dim FUN_7402029b with light flag 1 (FUN_74017fc4, the 0x4c
// D3DLIGHT from d3drm FUN_6d8410d6): the eye taken into the object's
// frame, so the view direction runs from the object's origin to the eye.
#[test]
fn the_view_direction_runs_from_each_object_to_the_eye() {
    let eye = vec3(0.0, 10.0, 0.0);
    assert_eq!(original_view_direction(eye, Vec3::ZERO), Vec3::Y);
    let side = original_view_direction(eye, vec3(10.0, 0.0, 0.0));
    assert!((side - vec3(-1.0, 1.0, 0.0).normalize()).length() < 1e-6);
    assert_eq!(original_view_direction(eye, eye), Vec3::ZERO);
}

#[test]
fn source_light_rig_preserves_retained_mode_direction_and_intensity() {
    let direction = original_surface_to_light_direction();
    let inverse_length = 51.0_f32.sqrt().recip();
    assert_near(direction.x, 5.0 * inverse_length);
    assert_near(direction.y, 5.0 * inverse_length);
    assert_near(direction.z, inverse_length);
    assert_eq!(ORIGINAL_AMBIENT_LIGHT_BYTE, (ORIGINAL_AMBIENT_LIGHT_RGB * 256.0).trunc());
    let white = Vec3::ONE;
    let hull = [1.0, 1.0, 1.0, 15.0];
    // Facing away: the ambient alone, no specular.
    let (diffuse, shine) = original_face_light(white, Vec3::ZERO, hull, -direction, direction);
    assert_eq!(diffuse, Vec3::splat(128.0));
    assert_eq!(shine, Vec3::ZERO);
    // Facing the light: 128 + 204 clamps to 255; with the eye along the
    // light, n.h = 1 and the specular is 204.
    let (diffuse, shine) = original_face_light(white, Vec3::ZERO, hull, direction, direction);
    assert_eq!(diffuse, Vec3::splat(255.0));
    assert_eq!(shine, Vec3::splat(204.0));
    // Half lit, a grey material: round(128 x 0.5 + 204 x 0.5 x n.l).
    let tilted = (direction + Vec3::Y * 0.2).normalize();
    let nl = tilted.dot(direction);
    let grey = Vec3::splat(0.5);
    let (diffuse, _) = original_face_light(grey, Vec3::ZERO, [0.0; 4], tilted, direction);
    assert_eq!(diffuse.x, (64.0 + 102.0 * nl + 0.5).floor());
    // Emissive adds 255 x emissive.
    let (diffuse, _) =
        original_face_light(grey, Vec3::splat(0.1), [0.0; 4], -direction, direction);
    assert_eq!(diffuse.x, 90.0);
    // A specular power of 15 falls off away from the half vector.
    let view = (direction + Vec3::X).normalize();
    let (_, shine) = original_face_light(white, Vec3::ZERO, hull, direction, view);
    let nh = direction.dot((direction + view).normalize());
    assert_eq!(shine.x, (204.0 * nh.powf(15.0) + 0.5).floor());
    // The texel's channel: (t x d) >> 8, plus the specular, at most 255.
    assert_eq!(original_hull_channel(165, 255.0, 0.0), 164.0);
    assert_eq!(original_hull_channel(165, 128.0, 0.0), 82.0);
    assert_eq!(original_hull_channel(165, 255.0, 40.0), 204.0);
    assert_eq!(original_hull_channel(228, 255.0, 204.0), 255.0);
}

// Source: FUN_005d3770 dots the hull's position in camera coordinates
// (GetPosition against the camera frame) with the camera's world
// direction (+0x14, GetOrientation against the scene, 0x005d977f), made
// positive, and divides its thresholds by the camera's +0x48
// (0x005d98f0), the field: 0.2 at the battle's start.
#[test]
fn a_hull_s_detail_follows_its_depth_along_the_view_and_the_field() {
    let pose = OriginalCameraPose {
        position: vec3(0.0, 0.0, -10.0),
        up: Vec3::Y,
        field: 0.2,
        fovy_radians: 0.4,
        near: 1.0,
        far: 100.0,
        pitch_degrees: 0,
        yaw_degrees: 0,
        zoom_step: 5,
        orbit_step: 5,
        target_object_id: None,
        target: Vec3::ZERO,
    };
    let view = original_participant_lod_view(&pose, vec3(3.0, 4.0, 5.0), true);
    assert_near(view.view_depth, 15.0);
    assert_near(view.projection_scale, 0.2);
    // Behind the camera the depth is made positive.
    assert_near(original_participant_lod_view(&pose, vec3(0.0, 0.0, -30.0), true).view_depth, 20.0);
    // 15 / 0.2 = 75 and 40 / 0.2 = 200: close, medium and far.
    let at = |depth: f32| {
        select_original_tactical_lod(
            OriginalTacticalLod::Medium,
            original_participant_lod_view(&pose, vec3(0.0, 0.0, depth - 10.0), true),
        )
    };
    assert_eq!(at(74.0), OriginalTacticalLod::Close);
    assert_eq!(at(150.0), OriginalTacticalLod::Medium);
    assert_eq!(at(213.0), OriginalTacticalLod::Far);
    // Turned 45 degrees, the target lies 200^0.5 straight ahead in camera
    // coordinates, (0, 0, 14.14), and its dot with the world direction
    // (0.71, 0, -0.71 in source coordinates) is 10, not 14.14.
    let aside = OriginalCameraPose {
        target: vec3(10.0, 0.0, 0.0),
        ..pose
    };
    assert_near(
        original_participant_lod_view(&aside, vec3(10.0, 0.0, 0.0), true).view_depth,
        10.0,
    );
    // Off every axis: the camera at (0, 0, -10) looks at (10, 5, 0).
    let raised = OriginalCameraPose {
        target: vec3(10.0, 5.0, 0.0),
        ..pose
    };
    assert!(
        (original_participant_lod_view(&raised, vec3(0.0, 0.0, 5.0), true).view_depth
            - 0.774_110_2)
            .abs()
            < 1e-4
    );
}

// Source: FUN_005c1c10 initializes the retained-mode device state.
#[test]
fn source_device_state_uses_nearest_gouraud_backface_depth_contract() {
    assert!(!ORIGINAL_DEVICE_DITHER);
    // d3drm FUN_6d837223 enables specular for every group with a
    // material.
    assert!(ORIGINAL_SPECULAR_ENABLED);
    assert_eq!(original_tactical_texture_filter(), FilterMode::Nearest);
    let pipeline = original_tactical_pipeline_params();
    assert_eq!(pipeline.cull_face, CullFace::Back);
    assert_eq!(pipeline.front_face_order, FrontFaceOrder::CounterClockwise);
    assert_eq!(pipeline.depth_test, Comparison::LessOrEqual);
    assert!(pipeline.depth_write);
    assert!(pipeline.color_blend.is_none());
    assert!(pipeline.alpha_blend.is_none());
}

// Source: TACTICAL.DLL palette resources 5531-5557.
#[test]
fn tactical_palette_object_preserves_rgb_and_uses_opaque_texture_alpha() {
    let resource_id = 5537_u32;
    let mut bytes = Vec::from(PALETTE_MAGIC.as_slice());
    bytes.extend_from_slice(&1_u32.to_le_bytes());
    bytes.extend_from_slice(&resource_id.to_le_bytes());
    for index in 0_u16..256 {
        let component = u8::try_from(index).unwrap();
        bytes.extend_from_slice(&[component, 255 - component, 17]);
    }
    let palette = decode_palette_object(&bytes, resource_id).unwrap();
    assert_eq!(palette[0], [0, 255, 17, 255]);
    assert_eq!(palette[255], [255, 0, 17, 255]);
    assert!(decode_palette_object(&bytes, resource_id + 1).is_err());
}

// Source: TACTICAL.DLL type-303 indexed texture format.
#[test]
fn tactical_effect_decoder_keys_the_uniform_border_index_only() {
    let mut bytes = Vec::from(TEXTURE_MAGIC.as_slice());
    for value in [1_u32, 2, 2, 0, 1, 4, 0] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.extend_from_slice(&[7, 8, 8, 7]);
    let mut palette = [[0_u8; 4]; 256];
    palette[7] = [0, 0, 0, 255];
    palette[8] = [12, 34, 56, 255];
    let (width, height, rgba, transparent_index) =
        decode_indexed_effect_rgba(&bytes, &palette).unwrap();
    assert_eq!((width, height, transparent_index), (2, 2, 7));
    assert_eq!(
        rgba,
        vec![0, 0, 0, 0, 12, 34, 56, 255, 12, 34, 56, 255, 0, 0, 0, 0]
    );
}

// Source: TACTICAL.DLL type-303 indexed texture format; FUN_005d4d10
// turns decal transparency on for the planet, whose default transparent
// colour is black.
#[test]
fn the_planet_texture_decodes_its_palette_and_keys_only_black_out() {
    let mut bytes = Vec::from(TEXTURE_MAGIC.as_slice());
    for value in [1_u32, 2, 1, 5537, 2, 2, 0] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.extend_from_slice(&[3, 4]);
    let mut palette = [[0_u8; 4]; 256];
    palette[3] = [0, 0, 0, 255];
    palette[4] = [0, 0, 1, 255];
    let (width, height, mut rgba) =
        decode_indexed_rgba_with_rule(&bytes, &palette, 5537, 2).unwrap();
    assert_eq!((width, height), (2, 1));
    assert_eq!(rgba, vec![0, 0, 0, 255, 0, 0, 1, 255]);
    key_black_transparent(&mut rgba);
    assert_eq!(rgba, vec![0, 0, 0, 0, 0, 0, 1, 255]);
    assert!(decode_indexed_rgba_with_rule(&bytes, &palette, 5538, 2).is_err());
    assert!(decode_indexed_rgba_with_rule(&bytes, &palette, 5537, 1).is_err());
}

// Source: FUN_0059a850, FUN_00596ad0, FUN_005c2e60 planet resource
// and palette identity.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn tactical_planet_object_requires_its_exact_paired_palette() {
    let resource_id = 5507_u32;
    let palette_resource_id = resource_id + 30;
    let width = 256_u32;
    let height = 256_u32;
    let pixels = width * height;
    let mut bytes = Vec::from(TEXTURE_MAGIC.as_slice());
    for value in [1_u32, width, height, palette_resource_id, 2, pixels, 0] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.resize(bytes.len() + pixels as usize, 0);
    let record = NativeTextureRecord {
        identifier_kind: "id".to_string(),
        id: resource_id,
        name: None,
        language: 1033,
        kind: "indexed_rle".to_string(),
        width,
        height,
        palette_rule: Some("planet_pair".to_string()),
        source_sha256: "0".repeat(64),
        object_sha256: "0".repeat(64),
        object: "objects/unused.texture".to_string(),
    };
    validate_indexed_texture_object(&bytes, &record).unwrap();

    let palette_offset = TEXTURE_MAGIC.len() + 3 * std::mem::size_of::<u32>();
    bytes[palette_offset..palette_offset + 4]
        .copy_from_slice(&(palette_resource_id + 1).to_le_bytes());
    assert!(validate_indexed_texture_object(&bytes, &record).is_err());
}

// Source: FUN_005d9490, FUN_005d9620, FUN_005d9640, FUN_00595be0,
// FUN_005c1080 camera pose, clip planes, and field of view.
#[test]
fn original_camera_preserves_faction_pose_clip_and_field_contract() {
    let alliance = OriginalTacticalCamera::new(false, 100.0).pose();
    let empire = OriginalTacticalCamera::new(true, 100.0).pose();
    assert_eq!(alliance.pitch_degrees, 30);
    assert_eq!(alliance.yaw_degrees, -30);
    assert_eq!(empire.yaw_degrees, 150);
    // FUN_005d9640 yaws about +Y (0x005d967c pushes the axis (0, 1, 0)),
    // so the sides' views, 180 degrees apart, share their height.
    assert_near(alliance.position.x, -empire.position.x);
    assert_near(alliance.position.y, empire.position.y);
    assert_near(alliance.position.z, -empire.position.z);
    assert_near(alliance.position.y, 170.0 * 0.5);
    assert_near(alliance.near, 1.0);
    assert_near(alliance.far, 425.0);
    assert_near(alliance.field, 0.2);
    assert_near(alliance.fovy_radians, 2.0 * 0.2_f32.atan());
    assert_near(alliance.up.length(), 1.0);
}

fn project_for_test(camera: OriginalTacticalCamera, source: Vec3) -> Option<Vec2> {
    let pose = camera.pose();
    let matrix = Mat4::perspective_rh_gl(pose.fovy_radians, 1.0, pose.near, pose.far)
        * Mat4::look_at_rh(pose.position, pose.target, pose.up);
    project_world_position(
        matrix,
        vec3(source.x, source.y, -source.z),
        (16.0, 28.0, 440.0, 440.0),
    )
}

// Source: FUN_005c1d30 creates the viewport through FUN_005d9490 and
// FUN_005d9eb0 with x, y, width and height 0: the whole 440 x 438
// device. A0 TAC-01-C002 and TAC-01-C008 show its black rect at
// (17, 30)-(456, 467) at 640 x 480.
#[test]
fn the_scene_viewport_is_the_original_devices_440_by_438_rect() {
    assert_eq!(
        tactical_scene_viewport((16.0, 28.0, 444.0, 439.0)),
        (17.0, 30.0, 440.0, 438.0)
    );
    assert_eq!(
        tactical_scene_viewport((32.0, 56.0, 888.0, 878.0)),
        (34.0, 60.0, 880.0, 876.0)
    );
}

// Source: A0 TAC-01-C008, the Empire's view after Rotate Left 16 times.
// The holocube's near vertical edge, from corner (+x, -y, +z) up to
// (+x, +y, +z), lights these pixels: (x, row).
#[test]
fn the_holocube_edge_after_sixteen_turns_lands_on_the_original_pixels() {
    const A0_EDGE: [(f32, f32); 6] =
        [(92.0, 100.0), (94.0, 136.0), (97.0, 172.0), (99.0, 208.0), (102.0, 244.0), (105.0, 280.0)];
    let mut camera = OriginalTacticalCamera::new(true, 100.0);
    for _ in 0..16 {
        camera.turn_left();
    }
    let mut renderer = TacticalAssetRenderer::default();
    renderer.restore_camera_state(camera);
    let aperture = (16.0, 28.0, 444.0, 439.0);
    let points = (0..=400)
        .filter_map(|step| {
            let height = -50.0 + 100.0 * f32::from(step as u16) / 400.0;
            renderer.project_source_position(aperture, vec3(50.0, height, -50.0))
        })
        .collect::<Vec<_>>();
    for (pixel_x, row) in A0_EDGE {
        let centre = row + 0.5;
        let point = points
            .iter()
            .min_by(|a, b| (a.y - centre).abs().total_cmp(&(b.y - centre).abs()))
            .unwrap();
        // A one-pixel line centred at x lights the pixel holding x. The
        // original's line rasteriser is not GL's, so allow a quarter
        // pixel either side of A0's pixel; the old aperture-wide
        // projection put the line a whole pixel right.
        assert!(
            point.x > pixel_x - 0.25 && point.x < pixel_x + 1.25,
            "row {row}: A0 lights x {pixel_x}, the port's line is at {point:?}"
        );
    }
    // The bottom edge from the same corner towards (+x, -y, -z): A0
    // lights these (column, y) pixels.
    const A0_BOTTOM: [(f32, f32); 7] = [
        (150.0, 307.0),
        (200.0, 317.0),
        (250.0, 327.0),
        (300.0, 337.0),
        (350.0, 347.0),
        (400.0, 358.0),
        (430.0, 364.0),
    ];
    let points = (0..=400)
        .filter_map(|step| {
            let depth = -50.0 + 100.0 * f32::from(step as u16) / 400.0;
            renderer.project_source_position(aperture, vec3(50.0, -50.0, depth))
        })
        .collect::<Vec<_>>();
    for (column, pixel_y) in A0_BOTTOM {
        let centre = column + 0.5;
        let point = points
            .iter()
            .min_by(|a, b| (a.x - centre).abs().total_cmp(&(b.x - centre).abs()))
            .unwrap();
        assert!(
            point.y > pixel_y - 0.25 && point.y < pixel_y + 1.25,
            "column {column}: A0 lights y {pixel_y}, the port's line is at {point:?}"
        );
    }
}

// Source: FUN_005d39a0 / FUN_005d38d0 (SetDecalSize 1.5 x 5.0, origin
// (64, 64), no SetDecalScale) and FUN_005d2f50 (one unit toward the
// camera). A0 .artifacts/a0-vm/2026-11-18-edited-saves-8 run 1 (default
// Empire camera; the holocube fits yaw 150, pitch 30, field 0.2 on every
// edge pixel): the superlaser's target dies on the Alliance lane at
// (0, 0, -0.5e) (both runs' fire triangulates there), and its 3365
// (seq-l1-beam k1218) and 3370 (k1245) frames fit a 30.3-32 px decal
// centred at (368.6, 134.7) (tools/interface-parity/fireball-frame-a0.py
// --shape). The extent e is 127: the medium-LOD Imperial Dreadnaught
// (2641, x3.0) at (0, 0, 0.5e) lights A0 t-04's 84 hull pixels exactly
// (hyp: the fit; the save's unit count was not read).
#[test]
fn a_destroyed_fireball_is_a_decal_seven_and_a_half_units_wide_at_its_depth() {
    let camera = OriginalTacticalCamera::new(true, 127.0);
    let mut renderer = TacticalAssetRenderer::default();
    renderer.restore_camera_state(camera);
    let viewport = tactical_scene_viewport((16.0, 28.0, 444.0, 439.0));
    let hull = vec3(0.0, 0.0, 0.5 * 127.0);
    let (anchor, min, size) = renderer
        .decal_rect(viewport, hull, 1.0, 7.5, (64, 64), (128.0, 128.0))
        .unwrap();
    assert!(
        (anchor.x - 368.6).abs() < 1.0 && (anchor.y - 134.7).abs() < 1.0,
        "A0's fire is centred at (368.6, 134.7), the port's at {anchor:?}"
    );
    assert!(
        (30.3..=32.0).contains(&size.x) && size.x == size.y,
        "A0's decal is 30.3-32 px, the port's {size:?}"
    );
    assert!((min - (anchor - size * 0.5)).length() < 1e-3);

    // The size is 7.5 units at the depth of a frame one unit nearer
    // the camera than the hull: field 0.2 over the 438-row viewport.
    let pose = camera.pose();
    let forward = (pose.target - pose.position).normalize();
    let depth = (hull - pose.position).dot(forward) - 1.0;
    let expected = 7.5 * 219.0 / 0.2 / depth;
    assert!((size.x - expected).abs() < 0.02, "{} != {expected}", size.x);

    // Twice as far away, half as wide: not a fixed pixel size.
    let far = TacticalAssetRenderer {
        source_camera: Some(OriginalTacticalCamera::new(true, 254.0)),
        ..TacticalAssetRenderer::default()
    };
    let (_, _, far_size) = far
        .decal_rect(viewport, hull * 2.0, 1.0, 7.5, (64, 64), (128.0, 128.0))
        .unwrap();
    assert!((far_size.x * 2.0 - size.x).abs() < 0.2, "{far_size:?} against {size:?}");
}

// Source: FUN_005d39a0 gives the field frames 3520 and 3620 a 5.0-unit
// decal anchored on (64, 64) (0x5d39a0..0x5d39d8); FUN_005d3cc0 calls
// FUN_005d3060, which negates the unit vector to the camera (fchs,
// 0x5d3122): the cloud sits one unit behind the unit's centre.
#[test]
fn a_field_cloud_is_a_five_unit_decal_one_unit_behind_its_unit() {
    use rebellion_core::tactical::{
        ORIGINAL_FIELD_DECAL_CAMERA_OFFSET, ORIGINAL_FIELD_DECAL_ORIGIN,
        ORIGINAL_FIELD_DECAL_SIZE,
    };
    let camera = OriginalTacticalCamera::new(true, 106.0);
    let mut renderer = TacticalAssetRenderer::default();
    renderer.restore_camera_state(camera);
    let viewport = tactical_scene_viewport((16.0, 28.0, 444.0, 439.0));
    let unit = vec3(10.0, 0.0, 53.0);
    let (anchor, min, size) = renderer
        .decal_rect(
            viewport,
            unit,
            ORIGINAL_FIELD_DECAL_CAMERA_OFFSET,
            ORIGINAL_FIELD_DECAL_SIZE,
            ORIGINAL_FIELD_DECAL_ORIGIN,
            (128.0, 128.0),
        )
        .unwrap();
    let pose = camera.pose();
    let forward = (pose.target - pose.position).normalize();
    let depth = (unit - pose.position).dot(forward) + 1.0;
    let expected = 5.0 * 219.0 / 0.2 / depth;
    assert!((size.x - expected).abs() < 0.02, "{} != {expected}", size.x);
    assert_eq!(size.x, size.y);
    assert!((min - (anchor - size * 0.5)).length() < 1e-3);
}

// Source: FUN_005d39a0's hit decals: 2.5 units (5.0 x 0.5) anchored on
// texel (32, 32) of the 128 x 64 frames 3060 and 3180, where the spark
// is; each texel is 2.5 / 128 wide and 2.5 / 64 high.
#[test]
fn a_hit_decal_hangs_from_its_spark_texel_and_squeezes_its_wide_frame() {
    let mut renderer = TacticalAssetRenderer::default();
    renderer.restore_camera_state(OriginalTacticalCamera::new(true, 100.0));
    let viewport = tactical_scene_viewport((16.0, 28.0, 444.0, 439.0));
    let (anchor, min, size) = renderer
        .decal_rect(viewport, Vec3::ZERO, 1.0, 2.5, (32, 32), (128.0, 64.0))
        .unwrap();
    assert_eq!(size.x, size.y);
    assert!((anchor.x - min.x - size.x * 32.0 / 128.0).abs() < 1e-3);
    assert!((anchor.y - min.y - size.y * 32.0 / 64.0).abs() < 1e-3);
}

// Source: FUN_005d4d10 frames the planet at (2e, rand(0.2e) - 0.1e,
// rand(0.2e) - 0.1e) with an e-wide decal, from a stream seeded by the
// clock (FUN_005f5750). A0 TAC-01-C008 (Empire, 16 x Rotate Left)
// shows the disc's left edge at x 334, 339 and 352 on rows 30, 40 and
// 60, and its bottom edge at y 107 and 116 in columns 420 and 450. At
// extent 106 (the fixture's Umgul battle) the draws (6, 5) give those
// edges within two pixels (the probe finds edges by whole pixels); the
// centred frame is more than 9 px off.
// The disc's radius is 0.9905 of the decal's half width in the port's
// render (browser run 2026-11-12T04-31-05-179Z-3870797).
#[test]
fn the_umgul_planet_framing_is_one_of_the_originals_draws() {
    const A0_LEFT: [(f32, f32); 3] = [(30.0, 334.0), (40.0, 339.0), (60.0, 352.0)];
    const A0_BOTTOM: [(f32, f32); 2] = [(420.0, 107.0), (450.0, 116.0)];
    let extent = 106.0;
    let mut camera = OriginalTacticalCamera::new(true, extent);
    for _ in 0..16 {
        camera.turn_left();
    }
    let mut renderer = TacticalAssetRenderer::default();
    renderer.restore_camera_state(camera);
    let viewport = tactical_scene_viewport((16.0, 28.0, 444.0, 439.0));
    let pose = camera.pose();
    let right = (pose.target - pose.position).cross(pose.up).normalize_or_zero();
    let worst_miss = |offsets: crate::tactical_backdrop::BackdropOffsets| {
        let (y, z) = offsets.planet;
        let centre = vec3(2.0 * extent, y, -z);
        let middle = renderer.project_in_scene_viewport(viewport, centre).unwrap();
        let edge = renderer
            .project_in_scene_viewport(viewport, centre + right * extent * 0.5)
            .unwrap();
        let radius = 0.9905 * middle.distance(edge);
        let left = A0_LEFT.iter().map(|&(row, x)| {
            let dy = row + 0.5 - middle.y;
            (middle.x - (radius * radius - dy * dy).sqrt() - x).abs()
        });
        let bottom = A0_BOTTOM.iter().map(|&(column, y)| {
            let dx = column + 0.5 - middle.x;
            (middle.y + (radius * radius - dx * dx).sqrt() - y).abs()
        });
        left.chain(bottom).fold(0.0_f32, f32::max)
    };
    let drawn = crate::tactical_backdrop::BackdropOffsets::from_clock(
        crate::tactical_backdrop::UMGUL_A0_BACKDROP_CLOCK,
        extent,
        true,
    );
    let miss = worst_miss(drawn);
    assert!(miss < 2.0, "the drawn planet misses A0 by {miss} px");
    let centred = worst_miss(crate::tactical_backdrop::BackdropOffsets::CENTRED);
    assert!(centred > 9.0, "the centred planet misses A0 by {centred} px");
}

// Source: FUN_005d9640 (yaw about +Y, then pitch about +X, then
// (0, 0, -distance)). A0 TAC-01-C002: the first Imperial frame shows the
// holocube corner (+x, -y, -z) at (157, 290); TAC-01-C008 (16 times left)
// shows that corner at (105, 298) with the planet (+2e x) at the top
// right; TAC-03-C020 (120 times left, 8 zoom-outs) shows the Death Star
// (-2e x) near (267, 90), its random offsets aside.
#[test]
fn the_camera_orbits_the_vertical_axis_as_the_original_captures_show() {
    let near = |actual: Option<Vec2>, x: f32, y: f32, tolerance: f32| {
        let actual = actual.expect("in front of the camera");
        assert!(
            (actual.x - x).abs() <= tolerance && (actual.y - y).abs() <= tolerance,
            "expected near ({x}, {y}), got {actual:?}"
        );
    };
    let extent = 100.0;
    let corner = vec3(50.0, -50.0, -50.0);
    let mut camera = OriginalTacticalCamera::new(true, extent);
    near(project_for_test(camera, corner), 157.0, 290.0, 3.0);
    for _ in 0..16 {
        camera.turn_left();
    }
    near(project_for_test(camera, vec3(50.0, -50.0, 50.0)), 105.0, 298.0, 3.0);
    let planet = project_for_test(camera, vec3(2.0 * extent, 0.0, 0.0)).unwrap();
    assert!(planet.x > 400.0 && planet.y < 28.0, "planet at {planet:?}");
    for _ in 16..120 {
        camera.turn_left();
    }
    for _ in 0..8 {
        camera.zoom_out();
    }
    near(
        project_for_test(camera, vec3(-2.0 * extent, 11.0, -9.0)),
        267.0,
        90.0,
        50.0,
    );
}

// A0 TAC-01-C007 and TAC-01-C009 (Battle at Yavin, Imperial player):
// the Imperial Star Destroyer and Victory Destroyer 2 of Task Force 1
// against the Corellian Corvette Swift, squadrons in their hangars, so
// the extent is 106 (FUN_005ab650). The Star Destroyer leads at x 0 and
// the Victory stands one Stand Off slot from it (FUN_005ec400, the
// anchor less the slot at 0x005eb040, spacing FUN_005ea030's 12 over
// the 1.2-scaled footprints of TACTICAL.DLL meshes 2560 and 2550).
// Their boxes are the meshes' bounds (x3, 0x005d5b10) about the hulls,
// facing the Empire's -z (0x005ab921). A0 shows:
// - TAC-01-C009's sequence, frame 1 (three zoom-outs, before either
//   ship moved): the Star Destroyer's box at x 62..99, y 365..398 and
//   the Victory's at x 17 (the view's edge)..45, y 346..375;
// - TAC-03-C009 and C015 (first view): the Star Destroyer's box at
//   x 17 (edge)..54, y 404..447, and no box or hull of the Victory;
// - TAC-01-C007 (first view): Swift's hull pixels centred at
//   (369.4, 133.4).
#[test]
fn the_a0_yavin_battle_frames_its_hulls_where_the_original_does() {
    use rebellion_core::tactical::{
        original_formation_lateral_offset, original_formation_spacing,
        OriginalTacticalLayout,
    };
    const STAR_DESTROYER: [f32; 6] = [-1.527, -0.437, -3.002, 1.559, 1.137, 3.007];
    const VICTORY: [f32; 6] = [-1.57, -0.426, -2.633, 1.57, 0.943, 2.877];
    let layout = OriginalTacticalLayout::from_active_counts(1, 2);
    assert_eq!(layout.battle_extent, 106.0);
    let lane = layout.outer_positive_z;
    // The Empire's forward (0, 0, -1) gives the slot axis (-1, 0, 0).
    let spacing = original_formation_spacing(&[Some(6.755), Some(6.342)], 1);
    let victory_x = -original_formation_lateral_offset(1, spacing);
    assert_eq!(victory_x, 12.0);
    let viewport = tactical_scene_viewport((16.0, 28.0, 444.0, 439.0));
    let axes = original_hull_axes(vec3(0.0, 0.0, -1.0));
    let frame = |zoom_outs: usize, bounds: [f32; 6], source: Vec3| {
        let mut camera = OriginalTacticalCamera::new(true, layout.battle_extent);
        for _ in 0..zoom_outs {
            camera.zoom_out();
        }
        let mut renderer = TacticalAssetRenderer::default();
        renderer.restore_camera_state(camera);
        let centre = vec3(source.x, source.y, -source.z);
        let mut low = Vec2::splat(f32::MAX);
        let mut high = Vec2::splat(f32::MIN);
        for corner in 0..8 {
            let pick = |bit: usize, axis: usize| {
                if corner & bit == 0 {
                    bounds[axis]
                } else {
                    bounds[axis + 3]
                }
            };
            let local = vec3(pick(1, 0), pick(2, 1), -pick(4, 2));
            let point = centre + axes[0] * local.x + axes[1] * local.y + axes[2] * local.z;
            let pixel = renderer.project_in_scene_viewport(viewport, point).unwrap();
            low = low.min(pixel);
            high = high.max(pixel);
        }
        let middle = renderer.project_in_scene_viewport(viewport, centre).unwrap();
        (low, high, middle)
    };
    let near = |actual: f32, original: f32| {
        assert!((actual - original).abs() <= 1.5, "{actual} against A0 {original}");
    };
    let (low, high, _) = frame(3, STAR_DESTROYER, vec3(0.0, 0.0, lane));
    near(low.x, 62.0);
    near(high.x, 99.0);
    near(low.y, 365.0);
    near(high.y, 398.0);
    let (low, high, _) = frame(3, VICTORY, vec3(victory_x, 0.0, lane));
    assert!(low.x < 17.0, "{low}");
    near(high.x, 45.0);
    near(low.y, 346.0);
    near(high.y, 375.0);
    let (low, high, _) = frame(0, STAR_DESTROYER, vec3(0.0, 0.0, lane));
    assert!(low.x < 17.0, "{low}");
    near(high.x, 54.0);
    near(low.y, 404.0);
    near(high.y, 447.0);
    let (_, high, _) = frame(0, VICTORY, vec3(victory_x, 0.0, lane));
    assert!(high.x < 17.0, "the Victory is in A0's empty first view: {high}");
    let (_, _, swift) = frame(0, STAR_DESTROYER, vec3(0.0, 0.0, -lane));
    near(swift.x, 369.4);
    near(swift.y, 133.4);
}

#[test]
fn a_decal_is_drawn_only_when_its_square_reaches_the_view() {
    let view = (16.0, 28.0, 440.0, 440.0);
    assert!(decal_overlaps_aperture(vec2(0.0, 0.0), vec2(20.0, 30.0), view));
    assert!(!decal_overlaps_aperture(vec2(0.0, 0.0), vec2(16.0, 30.0), view));
    assert!(!decal_overlaps_aperture(vec2(0.0, 0.0), vec2(20.0, 28.0), view));
    assert!(decal_overlaps_aperture(vec2(455.0, 467.0), vec2(5.0, 5.0), view));
    assert!(!decal_overlaps_aperture(vec2(456.0, 100.0), vec2(5.0, 5.0), view));
    assert!(!decal_overlaps_aperture(vec2(100.0, 468.0), vec2(5.0, 5.0), view));
}

// Source: 0x005d3500 (vtable 0x0066d198 + 0x1c, called from
// FUN_005c1f10 with the hull's position and its +0x10 forward vector):
// rows (-Oz, 0, Ox)/|..|, (0, 1, 0), -O, then the position, passed to
// AddTransform(REPLACE).
#[test]
fn a_hull_turns_about_the_vertical_to_face_its_forward_direction() {
    let source_transform = |forward: Vec3, local: Vec3| {
        let side = vec3(-forward.z, 0.0, forward.x).normalize();
        side * local.x + Vec3::Y * local.y - forward * local.z
    };
    for forward in [
        vec3(0.0, 0.0, 1.0),
        vec3(0.0, 0.0, -1.0),
        vec3(1.0, 0.0, 0.0),
        vec3(0.6, 0.0, -0.8),
        // Not unit length: the side axis is normalised, -O is not.
        vec3(0.0, 0.5, 2.0),
    ] {
        let axes = original_hull_axes(forward);
        for local in [vec3(1.0, 0.0, 0.0), vec3(0.0, 2.0, 0.0), vec3(0.5, -1.0, 3.0)] {
            // Render space reflects source Z on both sides.
            let render_local = vec3(local.x, local.y, -local.z);
            let placed = axes[0] * render_local.x
                + axes[1] * render_local.y
                + axes[2] * render_local.z;
            let expected = source_transform(forward, local);
            assert!(
                (placed - vec3(expected.x, expected.y, -expected.z)).length() < 1e-5,
                "forward {forward:?} local {local:?}: {placed:?}"
            );
        }
    }
    // The source nose, local -z, points along the forward direction.
    let axes = original_hull_axes(vec3(0.0, 0.0, 1.0));
    assert_eq!(axes[2], vec3(0.0, 0.0, -1.0));
    assert_eq!(IDENTITY_HULL_AXES, [Vec3::X, Vec3::Y, Vec3::Z]);
    // A vertical forward leaves the side axis unnormalised (zero).
    assert_eq!(original_hull_axes(vec3(0.0, 1.0, 0.0))[0], Vec3::ZERO);
}

// Source: 0x005d5b10 (Load of type 0x12d, then Scale(DAT_0066d2b0 = 3.0)
// on all three axes) for every hull in the 0x0066c990 cache.
#[test]
fn a_hull_mesh_loads_three_times_its_authored_size() {
    let mut bytes = Vec::from(MESH_MAGIC.as_slice());
    for value in [1_u32, 1, 1, 3, 1, 0] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for value in [-1.0_f32, -0.5, -2.0, 1.0, 0.5, 2.0] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for value in [1.0_f32, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.extend_from_slice(&0_u32.to_le_bytes());
    for value in [0_u32, 3, 3] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for position in [[1.0_f32, 0.5, 2.0], [-1.0, 0.0, 0.0], [0.0, -0.5, -2.0]] {
        for value in position.into_iter().chain([0.0, 1.0, 0.0, 0.0, 0.0]) {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
    for index in [0_u16, 1, 2] {
        bytes.extend_from_slice(&index.to_le_bytes());
    }
    let decoded = decode_mesh_object(&bytes).unwrap();
    assert_eq!(decoded.source_bounds, [-3.0, -1.5, -6.0, 3.0, 1.5, 6.0]);
    assert_eq!(
        decoded.chunks[0].mesh.vertices[0].position,
        authored_position(vec3(3.0, 1.5, 6.0))
    );
    let envelope = tactical_collision_envelope(decoded.source_bounds);
    assert_eq!(envelope.vertical_radius, 1.5);
}

// Source: d3drm.dll device constructor 0x6d81a08b (quality +0x68 =
// 0x88, D3DRMRENDER_FLAT) and FUN_6d827ec2 (the lower of device and
// mesh quality): one normal per face.
#[test]
fn hull_faces_are_lit_flat_with_their_plane_normal() {
    let vertex = |position: Vec3, normal: Vec3| Vertex {
        position,
        uv: vec2(0.0, 0.0),
        color: [255, 255, 255, 255],
        normal: normal.extend(0.0),
    };
    // Two faces sharing an edge, their vertex normals smoothed apart.
    let vertices = [
        vertex(vec3(0.0, 0.0, 0.0), vec3(0.0, 1.0, 0.0)),
        vertex(vec3(1.0, 0.0, 0.0), vec3(0.6, 0.8, 0.0)),
        vertex(vec3(0.0, 0.0, 1.0), vec3(0.0, 0.8, 0.6)),
        vertex(vec3(1.0, -1.0, 1.0), vec3(0.6, 0.0, 0.8)),
    ];
    let meshes = flat_shaded_meshes(&vertices, &[0, 2, 1, 1, 2, 3]);
    assert_eq!(meshes.len(), 1);
    let mesh = &meshes[0];
    assert_eq!(mesh.vertices.len(), 6);
    assert_eq!(mesh.indices, vec![0, 1, 2, 3, 4, 5]);
    assert_eq!(mesh.vertices[1].position, vertices[2].position);
    // The first face lies in y = 0; its normal faces its vertex
    // normals' side, +y, whatever its winding.
    for corner in &mesh.vertices[..3] {
        assert!((corner.normal.truncate() - Vec3::Y).length() < 1e-6);
    }
    let second = (vertices[2].position - vertices[1].position)
        .cross(vertices[3].position - vertices[1].position)
        .normalize();
    let second = if second.y < 0.0 { -second } else { second };
    for corner in &mesh.vertices[3..] {
        assert!((corner.normal.truncate() - second).length() < 1e-6);
    }
    // Wound the other way, the same face still faces +y.
    let turned = flat_shaded_meshes(&vertices, &[0, 1, 2]);
    for corner in &turned[0].vertices {
        assert!((corner.normal.truncate() - Vec3::Y).length() < 1e-6);
    }
    // A degenerate face keeps its smoothed normal.
    let line = flat_shaded_meshes(&vertices[..2], &[0, 1, 1]);
    let expected = (vec3(0.0, 1.0, 0.0) + vec3(1.2, 1.6, 0.0)).normalize();
    assert!((line[0].vertices[0].normal.truncate() - expected).length() < 1e-6);
    // A chunk past the u16 range splits.
    let many = (0..30_000_u16).flat_map(|_| [0, 2, 1]).collect::<Vec<_>>();
    let split = flat_shaded_meshes(&vertices, &many);
    assert_eq!(split.len(), 2);
    assert_eq!(split[0].vertices.len(), 65_535);
    assert_eq!(split.iter().map(|mesh| mesh.vertices.len()).sum::<usize>(), 90_000);
    assert!(split.iter().all(|mesh| mesh.vertices.len() <= u16::MAX as usize));
}

// Source: FUN_005da150 (nine vertices, eight faces round vertex 8,
// SetColorRGB(0, 1, 0), Scale(0.06e, 0.06e, 0.15e), Translate(0, 0, -10)).
#[test]
fn the_superlaser_cone_sits_ten_units_behind_the_decal_pointing_at_the_target() {
    let origin = vec3(1.0, 2.0, 3.0);
    let axis = vec3(0.0, 0.0, 1.0);
    let mesh = death_star_beam_cone_mesh(origin, axis, 100.0);
    assert_eq!(mesh.vertices.len(), 9);
    assert_eq!(mesh.indices.len(), 24);
    assert_eq!(&mesh.indices[..6], &[8, 0, 1, 8, 1, 2]);
    assert_eq!(&mesh.indices[21..], &[8, 7, 0]);
    let base = origin - axis * 10.0;
    for vertex in &mesh.vertices[..8] {
        let offset = vertex.position - base;
        assert!((offset.length() - 6.0).abs() < 1e-4);
        assert!(offset.dot(axis).abs() < 1e-4);
    }
    assert!((mesh.vertices[8].position - (base + axis * 15.0)).length() < 1e-4);
    assert!(mesh.vertices.iter().all(|vertex| vertex.color == [0, 255, 0, 255]));
    // The ring steps 45 degrees: neighbours are 2r sin 22.5 apart.
    let chord = 2.0 * 6.0 * std::f32::consts::FRAC_PI_8.sin();
    for step in 0..8 {
        let next = (step + 1) % 8;
        let gap = mesh.vertices[step].position - mesh.vertices[next].position;
        assert!((gap.length() - chord).abs() < 1e-3, "step {step}: {gap:?}");
    }
    // A vertical axis still gives a ring round it.
    let up = death_star_beam_cone_mesh(Vec3::ZERO, Vec3::Y, 100.0);
    for vertex in &up.vertices[..8] {
        let offset = vertex.position - Vec3::Y * -10.0;
        assert!((offset.length() - 6.0).abs() < 1e-4);
        assert!(offset.y.abs() < 1e-4);
    }
}

// Source: REBEXE 0x005da6d8 / 0x005da8ca LookAt(target, decal frame, 0);
// d3drm.dll FUN_6d81fa3a -> FUN_6d81f86b (constraint Z) takes the
// reference +Y made perpendicular to the axis as up, falling back to
// (-y, s, 0) only when s = sqrt(1 - y^2) < 1e-4.
#[test]
fn the_superlaser_cone_rolls_as_look_at_with_the_z_constraint() {
    let axis = vec3(0.1, 0.995, 0.0).normalize();
    let s = (1.0 - axis.y * axis.y).sqrt();
    let expected_up = vec3(-axis.y * axis.x / s, s, 0.0);
    let mesh = death_star_beam_cone_mesh(Vec3::ZERO, axis, 100.0);
    let base = -axis * 10.0;
    // Ring vertex 2 (90 degrees) lies along up.
    let offset = (mesh.vertices[2].position - base) / 6.0;
    assert!((offset - expected_up).length() < 1e-4, "{offset:?}");
    // An axis off both level axes keeps the full Gram-Schmidt up.
    let axis = vec3(0.05, 0.99, 0.08).normalize();
    let s = (1.0 - axis.y * axis.y).sqrt();
    let expected_up = vec3(-axis.y * axis.x / s, s, -axis.y * axis.z / s);
    let mesh = death_star_beam_cone_mesh(Vec3::ZERO, axis, 100.0);
    let offset = (mesh.vertices[2].position + axis * 10.0) / 6.0;
    assert!((offset - expected_up).length() < 1e-4, "{offset:?}");
    // Exactly vertical: up is (-1, 0, 0).
    let vertical = death_star_beam_cone_mesh(Vec3::ZERO, Vec3::Y, 100.0);
    let offset = (vertical.vertices[2].position + Vec3::Y * 10.0) / 6.0;
    assert!((offset - vec3(-1.0, 0.0, 0.0)).length() < 1e-4, "{offset:?}");
    // A level axis keeps world +Y as up.
    let level = death_star_beam_cone_mesh(Vec3::ZERO, Vec3::Z, 100.0);
    let offset = (level.vertices[2].position + Vec3::Z * 10.0) / 6.0;
    assert!((offset - Vec3::Y).length() < 1e-4, "{offset:?}");
}

// Source: FUN_005da150 SetQuality(0x40) (meshbuilder slot +0x78,
// D3DRMRENDER_WIREFRAME) on the cone; the ring 10 behind the decal is
// hidden by its depth, as Sistros's r0Pc2XALoqA capture shows (8 spokes).
#[test]
fn the_superlaser_cone_draws_its_spokes_as_lines_and_hides_its_ring_behind_the_decal() {
    let origin = vec3(1.0, 2.0, 3.0);
    // The beam points at the camera: the view looks down -Z.
    let axis = vec3(0.0, 0.0, 1.0);
    let view_forward = vec3(0.0, 0.0, -1.0);
    let segments = death_star_beam_cone_segments(origin, axis, 100.0, view_forward, origin);
    // Eight spokes; the eight ring edges lie wholly behind the decal.
    assert_eq!(segments.len(), 8);
    let apex = origin + axis * 5.0;
    for (from, to) in &segments {
        let (near, far) = if from.z > to.z { (from, to) } else { (to, from) };
        assert!((*near - apex).length() < 1e-4, "{near:?}");
        // Each spoke is cut at the decal's depth (z 3), a third of the
        // way from the apex (z 8) to the ring (z -7): radius 6 * 5 / 15.
        assert!((far.z - origin.z).abs() < 1e-4, "{far:?}");
        let radial = vec2(far.x - origin.x, far.y - origin.y).length();
        assert!((radial - 2.0).abs() < 1e-3, "{far:?}");
    }
    // Seen from behind the Death Star, nothing of the cone is in front
    // except the ring and the spokes' far ends: all 16 edges show, cut.
    let behind = death_star_beam_cone_segments(origin, axis, 100.0, -view_forward, origin);
    assert_eq!(behind.len(), 16);
    let ring_edges = behind
        .iter()
        .filter(|(from, to)| (from.z - (origin.z - 10.0)).abs() < 1e-4
            && (to.z - (origin.z - 10.0)).abs() < 1e-4)
        .count();
    assert_eq!(ring_edges, 8);
}

// A0 2026-11-18-edited-saves-8 l-07 (377f230464ae): the beam's line
// leaves the dish behind the Death Star decal, which hides it up to the
// sphere's rim; a beam towards the camera stays in front.
// A0 2026-11-18-edited-saves-8 l-07 (377f230464ae, run 2 of
// seq-l2-beam): the superlaser's star at the dish, 13 x 11 pixels at
// (417..429, 220..230) on a 42-pixel sphere, the ring in front of the
// decal, and the line hidden by the sphere up to x 401 then visible.
// The camera is fitted to that frame's holocube (pitch 30, yaw 180,
// 21 zoom-outs: mean edge distance 2.2 px) and the Death Star decal to
// its sphere (frame offset (0.13e, 0)); the target is fitted on the ray
// through l-08's tip (237, 230) and the extent to the star (hyp: 120,
// the -10 translate is in world units). The beam is drawn by the port's
// geometry (death_star_beam_cone_segments, split_at_decal_depth); the
// sphere's opaque pixels are A0's 21-pixel disc.
#[test]
fn the_superlaser_star_matches_a0_l07_at_its_fitted_camera() {
    let e = 120.0_f32;
    let mut camera = OriginalTacticalCamera::new(true, e);
    camera.pitch_degrees = 30;
    camera.yaw_degrees = 180;
    for _ in 0..21 {
        camera.zoom_out();
    }
    let pose = camera.pose();
    let aperture = tactical_scene_viewport((16.0, 28.0, 444.0, 439.0));
    // Camera3D::matrix for this pose (its Default needs a window).
    let matrix = Mat4::perspective_rh_gl(pose.fovy_radians, aperture.2 / aperture.3, pose.near, pose.far)
        * Mat4::look_at_rh(pose.position, pose.target, pose.up);
    let project = |point: Vec3| project_world_position(matrix, point, aperture);
    let decal = vec3(-2.0 * e, 0.13 * e, 0.0);
    let dish = vec3(-2.0 * e, 0.24 * e, 0.09 * e);
    let target = vec3(0.0, 0.229 * e, 0.013 * e);
    let sphere = project(decal).unwrap();
    // The fitted decal lands on A0's sphere centre (418, 238.8).
    assert!((sphere - vec2(418.0, 238.8)).length() < 1.0, "{sphere:?}");
    let view_forward = (pose.target - pose.position).normalize();
    let axis = (target - dish).normalize();
    let tip = dish + (target - dish) * 0.5;
    let (line_front, line_behind) = split_at_decal_depth(dish, tip, decal, view_forward);
    let mut pixels = std::collections::BTreeSet::new();
    let mut plot = |from: Vec3, to: Vec3, under_decal: bool| {
        for step in 0..=400 {
            let point = from + (to - from) * (step as f32 / 400.0);
            let Some(screen) = project(point) else { continue };
            if under_decal && screen.distance(sphere) < 21.0 {
                continue;
            }
            pixels.insert((screen.x.floor() as i32, screen.y.floor() as i32));
        }
    };
    for (from, to) in death_star_beam_cone_segments(dish, axis, e, view_forward, decal)
        .into_iter()
        .chain(line_front)
    {
        plot(from, to, false);
    }
    if let Some((from, to)) = line_behind {
        plot(from, to, true);
    }
    // The star: the pixels near the dish.
    let star: Vec<_> = pixels
        .iter()
        .filter(|(x, y)| (405..440).contains(x) && (212..236).contains(y))
        .collect();
    let bbox = star.iter().fold((i32::MAX, i32::MAX, i32::MIN, i32::MIN), |b, &&(x, y)| {
        (b.0.min(x), b.1.min(y), b.2.max(x), b.3.max(y))
    });
    // A0 l-07's star pixels, (0, 255, 0), by row.
    let a0_rows: [(i32, &[i32]); 11] = [
        (220, &[425, 426, 427]),
        (221, &[422, 423, 424, 425, 427]),
        (222, &[420, 421, 428]),
        (223, &[426, 427, 428, 429]),
        (224, &[418, 419, 420, 421, 422, 423, 424, 425, 429]),
        (225, &[417, 429]),
        (226, &[417, 418, 429]),
        (227, &[419, 420, 421, 422, 423, 424, 425, 426, 427, 428, 429]),
        (228, &[428]),
        (229, &[422, 427]),
        (230, &[423, 424, 425, 426]),
    ];
    let a0_star: Vec<(i32, i32)> = a0_rows
        .iter()
        .flat_map(|&(y, xs)| xs.iter().map(move |&x| (x, y)))
        .collect();
    let near = |p: (i32, i32), set: &[(i32, i32)]| {
        set.iter().any(|q| (q.0 - p.0).abs() <= 1 && (q.1 - p.1).abs() <= 1)
    };
    let star_points: Vec<(i32, i32)> = star.iter().map(|&&p| p).collect();
    // Every port pixel lies within a pixel of A0's, and nine in ten of
    // A0's within a pixel of the port's (measured: 44 of 44, 43 of 47).
    assert!(star_points.iter().all(|&p| near(p, &a0_star)), "{star_points:?}");
    let covered = a0_star.iter().filter(|&&p| near(p, &star_points)).count();
    assert!(covered * 10 >= a0_star.len() * 9, "{covered} of {}", a0_star.len());
    let a0 = (417, 220, 429, 230);
    assert!(
        (bbox.0 - a0.0).abs() <= 1
            && (bbox.1 - a0.1).abs() <= 1
            && (bbox.2 - a0.2).abs() <= 1
            && (bbox.3 - a0.3).abs() <= 1,
        "star {bbox:?} against A0 {a0:?}"
    );
    // The line: hidden by the sphere from the star to x 401, then
    // drawn on row 226 (A0: x 395..400 visible, 401..416 hidden).
    assert!((395..=400).all(|x| pixels.iter().any(|&(px, py)| px == x && (225..=227).contains(&py))));
    assert!(!pixels.iter().any(|&(x, y)| (403..=414).contains(&x) && (224..=228).contains(&y)));
}

#[test]
fn the_superlaser_line_behind_the_decal_is_kept_for_the_pass_under_it() {
    let origin = vec3(0.0, 0.0, 0.0);
    let view_forward = vec3(0.0, 0.0, -1.0);
    // Away from the camera and to the left: wholly behind.
    let far = vec3(-50.0, 0.0, -30.0);
    assert_eq!(
        split_at_decal_depth(origin, far, origin, view_forward),
        (None, Some((origin, far)))
    );
    // Towards the camera: wholly in front.
    let near = vec3(-50.0, 0.0, 30.0);
    assert_eq!(
        split_at_decal_depth(origin, near, origin, view_forward),
        (Some((origin, near)), None)
    );
    // Through the plane: cut where it crosses the decal's depth.
    let (front, behind) = split_at_decal_depth(
        vec3(0.0, 0.0, 10.0),
        vec3(0.0, 0.0, -10.0),
        origin,
        view_forward,
    );
    assert_eq!(front, Some((vec3(0.0, 0.0, 10.0), Vec3::ZERO)));
    assert_eq!(behind, Some((Vec3::ZERO, vec3(0.0, 0.0, -10.0))));
}

// Source: FUN_005ab650 tactical layout and lane derivation.
#[test]
fn original_layout_expands_extent_and_derives_four_source_lanes() {
    let balanced = OriginalTacticalLayout::from_active_counts(2, 2);
    assert_eq!(balanced.first_active_objects, 2);
    assert_eq!(balanced.second_active_objects, 2);
    assert_near(balanced.battle_extent, 106.0);
    assert_near(balanced.outer_positive_z, 53.0);
    assert_near(balanced.outer_negative_z, -53.0);
    assert_near(balanced.inner_negative_z, -33.0);
    assert_near(balanced.inner_positive_z, 33.0);

    let uneven = OriginalTacticalLayout::from_active_counts(7, 3);
    assert_near(uneven.battle_extent, 121.0);
    assert_near(uneven.outer_positive_z, 60.5);
    assert_near(uneven.inner_positive_z, 40.5);
}

// Class_0066d478_vtc (0x005d9b10), no second object: e = max(X span,
// Y span) of the object's box (vt +0x2c); SetPosition (+0xf4) puts the
// camera at (-e, e, 2e) in the hull's frame and SetOrientation (+0xf0)
// faces it along the hull's (0, 0, -1) with up (0, 1, 0). The hull's
// frame is 0x005d3500's: x = (-Oz, 0, Ox), y = +Y, z = -O.
#[test]
fn the_chase_camera_sits_behind_and_above_its_hull_and_looks_along_its_nose() {
    // A hull at (10, 2, 30) heading source +X; box spans 4 (x), 6 (y)
    // and 40 (z): e is 6, the Z span does not count.
    let view = original_chase_view(
        10_001,
        vec3(10.0, 2.0, 30.0),
        vec3(1.0, 0.0, 0.0),
        [-2.0, -3.0, -20.0, 2.0, 3.0, 20.0],
    );
    // Source: side = (0, 0, 1), z = (-1, 0, 0). Eye = P - 6 side + 6 Y
    // + 12 z = (-2, 8, 24); render space reflects Z.
    assert_eq!(view.eye, vec3(-2.0, 8.0, -24.0));
    assert_eq!(view.direction, vec3(1.0, 0.0, 0.0));
    assert_eq!(view.up, Vec3::Y);
    assert_eq!(view.object_id, 10_001);
    // Heading source +Z (the Alliance's lane faces the Empire's).
    let view = original_chase_view(
        1,
        Vec3::ZERO,
        vec3(0.0, 0.0, 1.0),
        [0.0, 0.0, 0.0, 4.0, 2.0, 1.0],
    );
    assert_eq!(view.eye, vec3(4.0, 4.0, 8.0));
    assert_eq!(view.direction, vec3(0.0, 0.0, -1.0));
    // A centred box whose X span (6) is the larger: e is 6.
    let view = original_chase_view(
        1,
        Vec3::ZERO,
        vec3(0.0, 0.0, 1.0),
        [-3.0, -1.0, -9.0, 3.0, 1.0, 9.0],
    );
    assert_eq!(view.eye, vec3(6.0, 6.0, 12.0));
    // A climbing hull: 0x005d3500's x axis (-Oz, 0, Ox) is normalised.
    let view = original_chase_view(
        1,
        Vec3::ZERO,
        vec3(0.0, 0.6, 0.8),
        [-1.0, -1.0, -1.0, 1.0, 1.0, 1.0],
    );
    for (axis, expected) in [(view.eye.x, 2.0), (view.eye.y, -0.4), (view.eye.z, 3.2)] {
        assert_near(axis, expected);
    }
    // A vertical heading leaves that axis zero, as 0x005d3500 does.
    let view = original_chase_view(1, Vec3::ZERO, Vec3::Y, [-1.0, -1.0, -1.0, 1.0, 1.0, 1.0]);
    assert_eq!(view.eye, vec3(0.0, -2.0, 0.0));
}

// 0x005d9b10 sets the viewport's front to 0.1 and its field to 0.02
// (FUN_005d9a70's +0x38); the free camera (0x005d9640) sets front 1.0
// and field 0.2. FUN_005c2990 command 8 memorizes the free camera.
#[test]
fn a_chase_replaces_the_orbit_view_until_it_is_cleared() {
    let mut camera = OriginalTacticalCamera::new(false, 100.0);
    let free = camera.pose();
    let view = original_chase_view(
        10_001,
        vec3(0.0, 0.0, -53.0),
        vec3(0.0, 0.0, 1.0),
        [-1.0, -1.0, -1.0, 1.0, 1.0, 1.0],
    );
    camera.set_chase(view);
    let chase = camera.pose();
    assert_eq!(chase.position, view.eye);
    assert_eq!(chase.target, view.eye + view.direction);
    assert_eq!(chase.up, Vec3::Y);
    assert_near(chase.near, 0.1);
    // The chase sets no back plane: the free camera's stays.
    assert_near(chase.far, free.far);
    assert_near(chase.field, 0.02);
    assert_near(chase.fovy_radians, 2.0 * 0.2_f32.atan());
    assert_eq!(chase.target_object_id, Some(10_001));
    assert_eq!(camera.without_chase().pose(), free);
    let mut renderer = TacticalAssetRenderer::default();
    renderer.restore_camera_state(OriginalTacticalCamera::new(false, 100.0));
    renderer.set_chase(view);
    assert_eq!(
        renderer.camera_state().map(|camera| camera.pose()),
        Some(free)
    );
    camera.clear_chase();
    assert_eq!(camera.pose(), free);
}

// 0x005d97c0 command 9 with no selection: FUN_00595be0 gives 0, so the
// camera drops its target (+0x44, +0x40) and looks at its origin.
#[test]
fn camera_command_nine_without_a_selection_drops_the_target() {
    let mut camera = OriginalTacticalCamera::new(false, 100.0);
    assert_eq!(camera.target_object_id(), None);
    camera.focus_target(17, vec3(4.0, 5.0, 6.0));
    assert_eq!(camera.target_object_id(), Some(17));
    camera.clear_target();
    assert_eq!(camera.target_object_id(), None);
    assert_eq!(camera.pose().target, Vec3::ZERO);
    let mut renderer = TacticalAssetRenderer::default();
    assert_eq!(renderer.target_object_id(), None);
    renderer.restore_camera_state(OriginalTacticalCamera::new(false, 100.0));
    renderer.focus_target(23, vec3(1.0, 2.0, 3.0));
    assert_eq!(renderer.target_object_id(), Some(23));
    renderer.clear_target();
    assert_eq!(renderer.target_object_id(), None);
    assert_eq!(renderer.camera_state().map(|camera| camera.target_point()), Some(Vec3::ZERO));
    assert_eq!(camera.target_point(), Vec3::ZERO);
}

// Source: command switch at 0x005d97c0 for zoom and orbit steps.
#[test]
fn original_camera_commands_use_field_zoom_and_adaptive_orbit_steps() {
    let mut camera = OriginalTacticalCamera::new(false, 100.0);
    camera.zoom_in();
    assert_near(camera.field, 0.18);
    assert_eq!(camera.zoom_step, 4);
    assert_eq!(camera.orbit_step, 4);
    camera.turn_left();
    assert_eq!(camera.yaw_degrees, -34);
    camera.turn_right();
    assert_eq!(camera.yaw_degrees, -30);
    camera.zoom_out();
    assert_near(camera.field, 0.198);
    assert_eq!(camera.zoom_step, 5);
    assert_eq!(camera.orbit_step, 5);
    camera.pitch_up();
    assert_eq!(camera.pitch_degrees, 35);
    camera.pitch_down();
    assert_eq!(camera.pitch_degrees, 30);
    camera.focus_target(17, vec3(4.0, 5.0, 6.0));
    let targeted = camera.pose();
    assert_eq!(targeted.target_object_id, Some(17));
    assert_eq!(targeted.target, vec3(4.0, 5.0, 6.0));

    for _ in 0..100 {
        camera.zoom_in();
    }
    assert_near(camera.field, ORIGINAL_CAMERA_FIELD_MIN);
    assert_eq!(camera.orbit_step, 1);
    for _ in 0..200 {
        camera.zoom_out();
    }
    assert_near(camera.field, ORIGINAL_CAMERA_FIELD_MAX);
    assert_eq!(camera.orbit_step, 5);
}

// Source: FUN_005d3770 LOD state selection and threshold edges.
#[test]
fn original_lod_rule_preserves_threshold_edges_and_reduced_detail_mode() {
    let close = TacticalLodView::CLOSE_FIXTURE;
    let medium = TacticalLodView::MEDIUM_FIXTURE;
    let far = TacticalLodView::FAR_FIXTURE;
    assert_eq!(
        select_original_tactical_lod(OriginalTacticalLod::Medium, close),
        OriginalTacticalLod::Close
    );
    assert_eq!(
        select_original_tactical_lod(OriginalTacticalLod::Close, medium),
        OriginalTacticalLod::Medium
    );
    assert_eq!(
        select_original_tactical_lod(OriginalTacticalLod::Medium, far),
        OriginalTacticalLod::Far
    );
    for threshold in [ORIGINAL_CLOSE_THRESHOLD, ORIGINAL_MEDIUM_THRESHOLD] {
        let view = TacticalLodView {
            view_depth: threshold,
            projection_scale: 1.0,
            high_detail: true,
        };
        for current in [
            OriginalTacticalLod::Close,
            OriginalTacticalLod::Medium,
            OriginalTacticalLod::Far,
        ] {
            assert_eq!(select_original_tactical_lod(current, view), current);
        }
    }
    fn immediately_below(value: f32) -> f32 {
        f32::from_bits(value.to_bits() - 1)
    }
    fn immediately_above(value: f32) -> f32 {
        f32::from_bits(value.to_bits() + 1)
    }

    // Non-unit scales guard the executable's divide-then-round sequence.
    // A multiply-first implementation fails at least the 0.3 close edge.
    for projection_scale in [0.3_f32, 2.3_f32] {
        let close_edge = ORIGINAL_CLOSE_THRESHOLD / projection_scale;
        let medium_edge = ORIGINAL_MEDIUM_THRESHOLD / projection_scale;
        for (edge, below, above) in [
            (
                close_edge,
                OriginalTacticalLod::Close,
                OriginalTacticalLod::Medium,
            ),
            (
                medium_edge,
                OriginalTacticalLod::Medium,
                OriginalTacticalLod::Far,
            ),
        ] {
            let at_edge = TacticalLodView {
                view_depth: edge,
                projection_scale,
                high_detail: true,
            };
            assert_eq!(
                select_original_tactical_lod(OriginalTacticalLod::Far, at_edge),
                OriginalTacticalLod::Far
            );
            assert_eq!(
                select_original_tactical_lod(
                    OriginalTacticalLod::Far,
                    TacticalLodView {
                        view_depth: immediately_below(edge),
                        ..at_edge
                    },
                ),
                below
            );
            assert_eq!(
                select_original_tactical_lod(
                    OriginalTacticalLod::Close,
                    TacticalLodView {
                        view_depth: immediately_above(edge),
                        ..at_edge
                    },
                ),
                above
            );
        }

        let reduced_edge = ORIGINAL_REDUCED_DETAIL_THRESHOLD / projection_scale;
        for (view_depth, expected) in [
            (immediately_below(reduced_edge), OriginalTacticalLod::Medium),
            (reduced_edge, OriginalTacticalLod::Medium),
            (immediately_above(reduced_edge), OriginalTacticalLod::Far),
        ] {
            assert_eq!(
                select_original_tactical_lod(
                    OriginalTacticalLod::Close,
                    TacticalLodView {
                        view_depth,
                        projection_scale,
                        high_detail: false,
                    },
                ),
                expected
            );
        }
    }

    let mut current = OriginalTacticalLod::Close;
    for (zoom, expected) in [
        (2.0, OriginalTacticalLod::Close),
        (1.6, OriginalTacticalLod::Medium),
        (0.524_288, OriginalTacticalLod::Far),
        (0.655_36, OriginalTacticalLod::Medium),
        (2.0, OriginalTacticalLod::Close),
    ] {
        current =
            select_original_tactical_lod(current, TacticalLodView::from_fixture_zoom(zoom));
        assert_eq!(current, expected);
    }
}

// Source: 0x005d4af0 fighter sprite selection thresholds.
#[test]
fn original_fighter_detail_uses_exact_source_thresholds() {
    for (view_span, expected) in [
        (0.0, OriginalFighterDetail::Close),
        (4.999, OriginalFighterDetail::Close),
        (5.0, OriginalFighterDetail::Far),
        (9.999, OriginalFighterDetail::Far),
        (10.0, OriginalFighterDetail::Indicator),
        (100.0, OriginalFighterDetail::Indicator),
    ] {
        assert_eq!(
            select_original_fighter_detail(OriginalFighterDetail::Far, view_span, true),
            expected
        );
    }
    assert_eq!(
        select_original_fighter_detail(OriginalFighterDetail::Close, 1.0, false),
        OriginalFighterDetail::Far
    );
    assert_eq!(
        select_original_fighter_detail(OriginalFighterDetail::Indicator, f32::NAN, true),
        OriginalFighterDetail::Indicator
    );
    assert_eq!(
        select_original_fighter_detail(OriginalFighterDetail::Close, -1.0, true),
        OriginalFighterDetail::Close
    );
}

// Source: TACTICAL.DLL resource identity sets (mesh 2010-2642,
// textures 3020-5557, named BMP textures).
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn complete_tactical_corpus_identity_sets_are_exact() {
    let meshes = expected_mesh_ids();
    let numeric = expected_numeric_texture_ids();
    let named: HashSet<_> = EXPECTED_NAMED_TEXTURES.iter().copied().collect();
    assert_eq!(meshes.len(), 87);
    assert_eq!(numeric.len(), 338);
    assert_eq!(named.len(), 59);
    assert!(meshes.contains(&2010));
    assert!(meshes.contains(&2642));
    assert!(!meshes.contains(&2009));
    assert!(!numeric.contains(&4040));
    assert!(numeric.contains(&4044));
    assert!(numeric.contains(&5557));
    assert!(named.contains("MONCAL52.BMP"));
}

// Source: FUN_005ab0e0 and FUN_005ab650 select the Death Star's 5010 or
// 5020 sprite outside the capital mesh table.
#[test]
fn death_star_manager_sprites_are_valid_original_effect_resources() {
    assert!(is_original_effect_resource(5010));
    assert!(is_original_effect_resource(5020));
    assert!(!is_original_effect_resource(5030));
}

// Source: TACTICAL.DLL native manifest — 87 meshes, 397 textures.
#[cfg(not(target_arch = "wasm32"))]
#[test]
#[ignore = "requires the ignored runtime store generated from an owned TACTICAL.DLL"]
fn owned_native_complete_corpus_installs_with_exact_counts() {
    let runtime = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/base/ui/tactical-dll/TACTICAL3D/runtime");
    let manifest = read_native_manifest(&runtime).expect("validate original corpus manifest");

    let mut replaced_mesh = manifest.clone();
    replaced_mesh.meshes[0].id = 9999;
    assert!(validate_native_manifest(&replaced_mesh).is_err());

    let mut wrong_language = manifest.clone();
    wrong_language.meshes[0].language = 0;
    assert!(validate_native_manifest(&wrong_language).is_err());

    let mut wrong_kind = manifest.clone();
    wrong_kind.textures[0].kind = "palette_rgb24".to_string();
    assert!(validate_native_manifest(&wrong_kind).is_err());

    let mut broken_binding = manifest;
    let binding = broken_binding
        .meshes
        .iter_mut()
        .find_map(|record| record.texture_bindings.as_mut()?.first_mut())
        .expect("original corpus contains a named texture binding");
    binding.resource_name = "NOT-A-TACTICAL-TEXTURE.BMP".to_string();
    assert!(validate_native_manifest(&broken_binding).is_err());

    install_native_tactical_assets(&runtime).expect("install complete native tactical corpus");
    let cache = TACTICAL_OBJECT_CACHE.lock().unwrap();
    assert_eq!(cache.meshes.len(), 87);
    assert_eq!(cache.textures.len(), 397);
    for key in ["2010/1033", "2510/1033", "2642/1033"] {
        assert!(cache.meshes.contains_key(key), "missing mesh {key}");
    }
    for key in ["MONCAL52.BMP/1033", "STRIKE52.BMP/1033", "5531/1033"] {
        assert!(cache.textures.contains_key(key), "missing texture {key}");
    }
}

#[test]
fn an_ortpal_record_gives_its_256_colours_after_its_head() {
    let mut bytes = b"ORTPAL00\x01\x00\x00\x00\x9b\x15\x00\x00".to_vec();
    bytes.extend((0..768).map(|i| (i % 251) as u8));
    let rgb = ortpal_rgb(&bytes).unwrap();
    assert_eq!(rgb.len(), 256);
    assert_eq!(rgb[0], [0, 1, 2]);
    assert_eq!(rgb[112], [85, 86, 87]);
    assert_eq!(ortpal_rgb(&bytes[..700]), None);
}

// FUN_005d8c30 (scale 1.5, FUN_005c1c10): a regular tetrahedron of
// edge 1.5, apex up, centred on its point.
#[test]
fn a_navigation_marker_is_a_regular_tetrahedron_of_edge_one_and_a_half() {
    let center = vec3(10.0, 20.0, 30.0);
    let faces = navigation_marker_faces(center, vec3(1.0, 0.0, 0.0));
    let mut corners = faces
        .iter()
        .flat_map(|(corners, _)| *corners)
        .collect::<Vec<_>>();
    corners.sort_by(|a, b| a.to_array().partial_cmp(&b.to_array()).unwrap());
    corners.dedup_by(|a, b| a.distance(*b) < 1e-4);
    assert_eq!(corners.len(), 4);
    for (index, a) in corners.iter().enumerate() {
        for b in &corners[index + 1..] {
            assert!((a.distance(*b) - 1.5).abs() < 2e-3, "{a} {b}");
        }
    }
    let top = corners
        .iter()
        .map(|corner| corner.y)
        .fold(f32::MIN, f32::max);
    assert!((top - (center.y + 0.6124 * 1.5)).abs() < 1e-4);
    let middle = corners.iter().copied().sum::<Vec3>() / 4.0;
    assert!(middle.distance(center) < 1e-3);
}

// FUN_005d8c30's face normals, z reflected into render space: each face
// takes its own outward normal's light.
#[test]
fn each_navigation_marker_face_is_lit_by_its_outward_normal() {
    let center = vec3(4.0, -2.0, 7.0);
    let tint = vec3(1.0, 1.0, 1.0);
    let corner = |index: usize| {
        let [x, y, z] = NAVIGATION_MARKER_VERTICES[index];
        center + authored_position(vec3(x, y, z)) * NAVIGATION_MARKER_SCALE
    };
    // The face opposite each vertex, and its source normal.
    let normals = [
        (0, vec3(0.0, 0.3333, -0.9428)),
        (1, vec3(-0.8165, 0.3333, 0.4714)),
        (2, vec3(0.8165, 0.3333, 0.4714)),
        (3, vec3(0.0, -1.0, 0.0)),
    ];
    let faces = navigation_marker_faces(center, tint);
    for (opposite, normal) in normals {
        let (_, color) = faces
            .iter()
            .find(|(corners, _)| corners.iter().all(|c| c.distance(corner(opposite)) > 1e-3))
            .unwrap();
        assert_eq!(
            *color,
            original_flat_face_color(tint, authored_position(normal)),
            "the face opposite vertex {opposite}"
        );
    }
    let bytes = faces.map(|(_, color): ([Vec3; 3], Color)| {
        let rgba: [u8; 4] = color.into();
        rgba[0]
    });
    assert_eq!(bytes.iter().filter(|&&byte| byte == 128).count(), 2, "{bytes:?}");
    assert!(bytes.contains(&255) && bytes.contains(&203), "{bytes:?}");
}

// d3dim's ambient 128 on the magenta mesh: A0 VM 2026-10-27
// (nav-point-left-click.png) draws the selected point (128, 0, 128); the
// faces the light reaches are brighter.
#[test]
fn a_navigation_markers_unlit_face_takes_the_ambient_light_alone() {
    let faces = navigation_marker_faces(Vec3::ZERO, vec3(1.0, 0.0, 1.0));
    let colors = faces.map(|(_, color)| -> [u8; 4] { color.into() });
    assert!(colors.contains(&[128, 0, 128, 255]), "{colors:?}");
    assert!(colors.iter().any(|color| color[0] > 128), "{colors:?}");
    assert!(colors
        .iter()
        .all(|color| color[1] == 0 && color[0] == color[2]));
}

// Source: TACTICAL.DLL native LOD family proof keys.
#[cfg(all(feature = "interface-test-fixtures", not(target_arch = "wasm32")))]
#[test]
#[ignore = "requires the ignored runtime store generated from an owned TACTICAL.DLL"]
fn owned_native_lod_family_installs_with_exact_typed_keys() {
    let runtime = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/base/ui/tactical-dll/TACTICAL3D/runtime");
    install_native_tactical_lod_family(&runtime)
        .expect("install owned P57 tactical LOD family");
    let cache = TACTICAL_OBJECT_CACHE.lock().unwrap();
    assert_eq!(cache.meshes.len(), PROOF_MESH_KEYS.len());
    assert_eq!(
        cache.textures.len(),
        PROOF_TEXTURE_KEYS.len()
            + usize::try_from(TACTICAL_PALETTE_LAST - TACTICAL_PALETTE_FIRST + 1).unwrap()
    );
    for key in PROOF_MESH_KEYS {
        assert!(cache.meshes.contains_key(key));
    }
    for key in PROOF_TEXTURE_KEYS {
        assert!(cache.textures.contains_key(key));
    }
    for palette_id in TACTICAL_PALETTE_FIRST..=TACTICAL_PALETTE_LAST {
        assert!(cache.textures.contains_key(&format!("{palette_id}/1033")));
    }
}
