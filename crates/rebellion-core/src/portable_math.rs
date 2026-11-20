//! Platform-independent `f32` transcendental functions for the simulation.
//!
//! `f32::sin_cos`, `acos` and `hypot` call the platform's C library natively
//! (glibc, the macOS libm) but Rust's own `libm` port in WebAssembly, and the
//! two may round the last bit differently. One such bit in a fighter's
//! `hypot` made the same tactical battle end in a different state natively
//! and in the browser (`rebellion_data::replay_sweep`). These versions use
//! only IEEE-754 basic operations and square roots, which every target rounds
//! identically, so state that goes through them is the same everywhere.
//!
//! port: the kernels follow FreeBSD's / musl's `k_sinf`, `k_cosf`,
//! `__rem_pio2f` and `acosf` (MIT-licensed), each well under one ulp in
//! `f32`; `hypot` sums in `f64`.

/// `x.hypot(y)`, from the exact `f64` squares (a product of two `f32` values
/// fits a `f64` exactly) and one correctly rounded `f64` square root.
#[must_use]
pub fn hypot(x: f32, y: f32) -> f32 {
    if x.is_infinite() || y.is_infinite() {
        return f32::INFINITY;
    }
    let (x, y) = (f64::from(x), f64::from(y));
    #[expect(
        clippy::cast_possible_truncation,
        reason = "Rounding the f64 root to f32 is the function's result."
    )]
    let root = (x * x + y * y).sqrt() as f32;
    root
}

// The constants are given by their bits, as FreeBSD gives them in hex.
// FreeBSD k_sinf.c: |sin(x)/x - s(x)| < 2**-37.5 on [-pi/4, pi/4].
const S1: f64 = f64::from_bits(0xbfc5_5555_54cb_ac77); // port: k_sinf.c -0x15555554cbac77p-55
const S2: f64 = f64::from_bits(0x3f81_1110_896e_fbb2); // port: k_sinf.c 0x111110896efbb2p-59
const S3: f64 = f64::from_bits(0xbf2a_00f9_e2ca_e774); // port: k_sinf.c -0x1a00f9e2cae774p-65
const S4: f64 = f64::from_bits(0x3ec6_cd87_8c3b_46a7); // port: k_sinf.c 0x16cd878c3b46a7p-71

// FreeBSD k_cosf.c: |cos(x) - c(x)| < 2**-34.1 on [-pi/4, pi/4].
const C0: f64 = f64::from_bits(0xbfdf_ffff_fd0c_5e81); // port: k_cosf.c -0x1ffffffd0c5e81p-54
const C1: f64 = f64::from_bits(0x3fa5_5553_e105_3a42); // port: k_cosf.c 0x155553e1053a42p-57
const C2: f64 = f64::from_bits(0xbf56_c087_e80f_1e27); // port: k_cosf.c -0x16c087e80f1e27p-62
const C3: f64 = f64::from_bits(0x3ef9_9342_e0ee_5069); // port: k_cosf.c 0x199342e0ee5069p-68

// FreeBSD e_rem_pio2f.c: 2/pi, and pi/2 split into 33 + 53 bits.
const INV_PIO2: f64 = std::f64::consts::FRAC_2_PI; // port: e_rem_pio2f.c
const PIO2_1: f64 = f64::from_bits(0x3ff9_21fb_5000_0000); // port: e_rem_pio2f.c
const PIO2_1T: f64 = f64::from_bits(0x3e51_10b4_611a_6263); // port: e_rem_pio2f.c
/// port: 1.5 / `f64::EPSILON` (e_rem_pio2f.c `toint`): adding it rounds a
/// `f64` to an integer.
const TO_INT: f64 = 1.5 / f64::EPSILON;

fn kernel_sin(x: f64) -> f64 {
    let z = x * x;
    let w = z * z;
    let r = S3 + z * S4;
    let s = z * x;
    (x + s * (S1 + z * S2)) + s * w * r
}

fn kernel_cos(x: f64) -> f64 {
    let z = x * x;
    let w = z * z;
    let r = C2 + z * C3;
    ((1.0 + z * C0) + w * C1) + (w * z) * r
}

/// port: `x.sin_cos()` for finite `x`: the argument reduced by the nearest
/// multiple of pi/2 in `f64`, then the polynomial kernels. A non-finite `x`
/// gives NaNs, as the standard function does.
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    reason = "The kernels' f64 results round to f32 once; the quadrant is a small integer."
)]
pub fn sin_cos(x: f32) -> (f32, f32) {
    if !x.is_finite() {
        return (f32::NAN, f32::NAN);
    }
    let x = f64::from(x);
    // |x| ~<= pi/4 needs no reduction (0x3f490fda, FreeBSD s_sinf.c).
    if x.abs() <= f64::from(f32::from_bits(0x3f49_0fda)) {
        return (kernel_sin(x) as f32, kernel_cos(x) as f32);
    }
    let n = (x * INV_PIO2 + TO_INT) - TO_INT;
    let y = x - n * PIO2_1 - n * PIO2_1T;
    let (sin, cos) = (kernel_sin(y), kernel_cos(y));
    // x = n pi/2 + y.
    let (sin, cos) = match (n as i64) & 3 {
        0 => (sin, cos),
        1 => (cos, -sin),
        2 => (-sin, -cos),
        _ => (-cos, sin),
    };
    (sin as f32, cos as f32)
}

// musl acosf.c.
const PIO2_HI: f32 = f32::from_bits(0x3fc9_0fda); // port: acosf.c
const PIO2_LO: f32 = f32::from_bits(0x33a2_2168); // port: acosf.c
const P_S0: f32 = f32::from_bits(0x3e2a_aa75); // port: acosf.c
const P_S1: f32 = f32::from_bits(0xbd2f_13ba); // port: acosf.c
const P_S2: f32 = f32::from_bits(0xbc0d_d36b); // port: acosf.c
const Q_S1: f32 = f32::from_bits(0xbf34_e5ae); // port: acosf.c

fn acos_rational(z: f32) -> f32 {
    let p = z * (P_S0 + z * (P_S1 + z * P_S2));
    let q = 1.0 + z * Q_S1;
    p / q
}

/// port: `x.acos()`, musl's `acosf` in `f32` arithmetic. Outside [-1, 1] it is
/// NaN, as the standard function is. musl adds `0x1p-120` to the results
/// for -1 and tiny `x` only to raise the inexact flag; the sums round to the
/// same values, so they are left out.
#[must_use]
pub fn acos(x: f32) -> f32 {
    let magnitude = x.abs();
    if magnitude >= 1.0 {
        return if x == 1.0 {
            0.0
        } else if x == -1.0 {
            2.0 * PIO2_HI
        } else {
            f32::NAN
        };
    }
    if magnitude < 0.5 {
        if magnitude.to_bits() <= 0x3280_0000 {
            return PIO2_HI;
        }
        return PIO2_HI - (x - (PIO2_LO - x * acos_rational(x * x)));
    }
    if x < 0.0 {
        let z = (1.0 + x) * 0.5;
        let s = z.sqrt();
        let w = acos_rational(z) * s - PIO2_LO;
        return 2.0 * (PIO2_HI - (s + w));
    }
    let z = (1.0 - x) * 0.5;
    let s = z.sqrt();
    let df = f32::from_bits(s.to_bits() & 0xffff_f000);
    let c = (z - df * df) / (s + df);
    let w = acos_rational(z) * s + c;
    2.0 * (df + w)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Distance in units in the last place between two finite `f32`s.
    fn ulps(a: f32, b: f32) -> u32 {
        let key = |v: f32| {
            let bits = v.to_bits().cast_signed();
            if bits < 0 {
                i32::MIN - bits
            } else {
                bits
            }
        };
        key(a).abs_diff(key(b))
    }

    fn samples() -> impl Iterator<Item = f32> {
        // Every 997th float in [-8, 8] plus the edges of each branch.
        let step = 997;
        let span = 8.0_f32.to_bits();
        (0..span)
            .step_by(step)
            .flat_map(|bits| {
                let v = f32::from_bits(bits);
                [v, -v]
            })
            .chain([0.0, -0.0, 0.5, -0.5, 1.0, -1.0, std::f32::consts::FRAC_PI_4])
    }

    #[test]
    fn sin_cos_stays_within_one_ulp_of_the_standard_library() {
        for x in samples() {
            let (sin, cos) = sin_cos(x);
            let (std_sin, std_cos) = x.sin_cos();
            // Near a zero the standard result's own error dominates.
            assert!(
                ulps(sin, std_sin) <= 1 || (sin - std_sin).abs() < 1e-7,
                "sin {x}"
            );
            assert!(
                ulps(cos, std_cos) <= 1 || (cos - std_cos).abs() < 1e-7,
                "cos {x}"
            );
        }
    }

    #[test]
    fn acos_stays_within_one_ulp_of_the_standard_library() {
        for x in samples().filter(|x| x.abs() <= 1.0) {
            assert!(
                ulps(acos(x), x.acos()) <= 1,
                "acos {x}: {} {}",
                acos(x),
                x.acos()
            );
        }
        assert!(acos(1.5).is_nan());
        assert_eq!(acos(1.0), 0.0);
    }

    #[test]
    fn hypot_stays_within_one_ulp_of_the_standard_library() {
        for x in samples() {
            for y in [0.0, 1.0, -3.5, 1e-3, 7.25, x * 0.5] {
                assert!(ulps(hypot(x, y), x.hypot(y)) <= 1, "hypot {x} {y}");
            }
        }
        assert_eq!(hypot(3.0, 4.0), 5.0);
        assert_eq!(hypot(f32::INFINITY, f32::NAN), f32::INFINITY);
    }

    #[test]
    fn the_quadrants_follow_the_angle_sum_identities() {
        let quarter = std::f32::consts::FRAC_PI_2;
        let (sin, cos) = sin_cos(quarter);
        assert!((sin - 1.0).abs() < 1e-7 && cos.abs() < 1e-7);
        let (sin, cos) = sin_cos(-std::f32::consts::PI);
        assert!(sin.abs() < 1e-6 && (cos + 1.0).abs() < 1e-7);
        assert!(sin_cos(f32::NAN).0.is_nan());
    }
}
