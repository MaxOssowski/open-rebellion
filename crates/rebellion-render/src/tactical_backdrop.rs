//! The random framing of the tactical backdrop decals.
//!
//! `FUN_005d4d10` builds the planet and Death Star frames with a local
//! random stream (the object at `[esp+0x20]`, constructed by
//! `FUN_005f5650` with state -1 at 0x005d52ab). Its first draw seeds it from
//! the wall clock: `FUN_005f5700` -> `FUN_005f56b0(-1)` -> `FUN_005f5750`,
//! which calls the CRT's `srand(time(NULL))` (`FUN_0061b120`, `FUN_0061a300`)
//! and combines four `rand()` (`FUN_0061a310`). The framing therefore depends
//! on the second the battle scene was built, not on the game state.

/// `FUN_005f57a0`: one step of the stream, `s * 41358 mod (2^31 - 1)` in
/// the original's wrapping 32-bit arithmetic.
fn step(state: i32) -> i32 {
    let next = state
        .wrapping_mul(0xa18e)
        .wrapping_add((state / 0xcad4).wrapping_mul(-0x7fff_ffff));
    if next < 0 {
        next.wrapping_add(0x7fff_ffff)
    } else {
        next
    }
}

/// `FUN_005f5750`: `srand(clock)`, then `rand() * rand() + rand() * rand()`
/// until the sum is not `0x7fffffff`. `FUN_0061a310` is the MSVC CRT
/// `rand`: `holdrand * 214013 + 2531011`, bits 16..30.
fn clock_seed(clock: u32) -> i32 {
    let mut holdrand = clock;
    let mut rand = || {
        holdrand = holdrand.wrapping_mul(214_013).wrapping_add(2_531_011);
        ((holdrand >> 16) & 0x7fff) as i32
    };
    loop {
        let seed = (rand() * rand()).wrapping_add(rand() * rand());
        if seed != 0x7fff_ffff {
            return seed;
        }
    }
}

/// The local stream of `FUN_005d4d10`, already seeded: `FUN_005f56b0` sets
/// the seed and steps it 16 times.
struct BackdropStream(i32);

impl BackdropStream {
    fn from_clock(clock: u32) -> Self {
        let mut state = clock_seed(clock);
        for _ in 0..16 {
            state = step(state);
        }
        Self(state)
    }

    /// `FUN_005f5700(n)`: 0 when `n <= 1`, otherwise the next state divided
    /// by `0x7ffffffe / n`: 0 to `n`, `n` itself rarely.
    fn draw(&mut self, bound: i32) -> i32 {
        if bound <= 1 {
            return 0;
        }
        self.0 = step(self.0);
        self.0 / (0x7fff_fffe / bound)
    }

    /// One offset of a frame: `rand(ftol(2 * k * e)) - k * e`, with `k * e`
    /// held as a float (`fst DWORD`, 0x005d53a8 and 0x005d552f).
    fn offset(&mut self, extent: f32, factor: f32) -> f32 {
        let part = (f64::from(extent) * f64::from(factor)) as f32;
        let bound = (2.0 * f64::from(part)) as i32;
        (f64::from(self.draw(bound)) - f64::from(part)) as f32
    }
}

/// The y and z offsets `FUN_005d4d10` gives the planet's and the Death
/// Star's frames.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BackdropOffsets {
    pub planet: (f32, f32),
    pub death_star: (f32, f32),
}

impl BackdropOffsets {
    /// Both frames at the middle of their ranges.
    pub(crate) const CENTRED: Self = Self {
        planet: (0.0, 0.0),
        death_star: (0.0, 0.0),
    };

    /// The offsets of a scene built at `clock` (seconds since 1970) for a
    /// battle extent `extent`. The planet's frame is built only while Show
    /// Planet is on (`+0x680`, 0x005d5372) and draws first: y at 0x005d53bc,
    /// z at 0x005d53e6, each `rand(0.2e) - 0.1e` (`DAT_0066d23c` 0.1). The
    /// Death Star's frame draws next: y at 0x005d5543, z at 0x005d556d, each
    /// `rand(0.4e) - 0.2e` (`DAT_0066d244` 0.2).
    pub(crate) fn from_clock(clock: u32, extent: f32, show_planet: bool) -> Self {
        let mut stream = BackdropStream::from_clock(clock);
        let planet = if show_planet {
            let y = stream.offset(extent, 0.1);
            let z = stream.offset(extent, 0.1);
            (y, z)
        } else {
            (0.0, 0.0)
        };
        let y = stream.offset(extent, 0.2);
        let z = stream.offset(extent, 0.2);
        Self {
            planet,
            death_star: (y, z),
        }
    }
}

/// A second whose stream frames the planet at extent 106 as A0
/// TAC-01-C008 shows it: planet draws 6 and 5 of `rand(21)`, offsets
/// (-4.6, -5.6). The capture's real second is unknown; any second the
/// stream maps to these draws gives the same planet. The port's test
/// fixture pins it for `production-umgul-backdrop`.
pub const UMGUL_A0_BACKDROP_CLOCK: u32 = 1_800_000_005;

/// The wall clock in whole seconds, the `time(NULL)` the original seeds
/// the backdrop stream with.
pub(crate) fn wall_clock_seconds() -> u32 {
    macroquad::miniquad::date::now() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    // Source: FUN_005f57a0 (multiplier 0xa18e, quotient 0xcad4, modulus
    // 0x7fffffff): the Park-Miller step with multiplier 41358.
    #[test]
    fn the_backdrop_stream_steps_as_the_minimal_standard_generator() {
        for state in [1_i32, 2, 12_345, 0x7fff_fffe, 1_000_000_007] {
            let expected = (i64::from(state) * 41_358) % 0x7fff_ffff;
            assert_eq!(i64::from(step(state)), expected, "state {state}");
        }
        // A zero state stays zero.
        assert_eq!(step(0), 0);
    }

    // Source: FUN_005f5750 with the MSVC CRT rand (FUN_0061a310). After
    // srand(1) the CRT's first four rand() values are 41, 18467, 6334 and
    // 26500.
    #[test]
    fn the_backdrop_seed_combines_four_crt_rand_values_after_srand() {
        assert_eq!(clock_seed(1), 41 * 18_467 + 6_334 * 26_500);
    }

    // Source: FUN_005f5700: the next state over 0x7ffffffe / n.
    #[test]
    fn a_backdrop_draw_spans_zero_to_its_bound() {
        let mut stream = BackdropStream(0x7fff_fffe / 21 * 6);
        let expected = step(stream.0) / (0x7fff_fffe / 21);
        assert_eq!(stream.draw(21), expected);
        assert_eq!(BackdropStream(5).draw(1), 0);
        let mut stream = BackdropStream::from_clock(1_800_000_000);
        let draws = (0..2000).map(|_| stream.draw(21)).collect::<Vec<_>>();
        assert!(draws.iter().all(|draw| (0..=21).contains(draw)));
        assert!((0..21).all(|value| draws.contains(&value)));
    }

    // Source: FUN_005d4d10: for extent 106 the planet's offsets are
    // rand(21) - 10.6 and the Death Star's rand(42) - 21.2; without Show
    // Planet the Death Star takes the stream's first two draws.
    #[test]
    fn the_planet_draws_first_and_the_death_star_follows() {
        let clock = 1_800_000_000;
        let mut stream = BackdropStream::from_clock(clock);
        let draws = [21, 21, 42, 42].map(|bound| stream.draw(bound) as f32);
        let shown = BackdropOffsets::from_clock(clock, 106.0, true);
        assert!((shown.planet.0 - (draws[0] - 10.6)).abs() < 1e-4);
        assert!((shown.planet.1 - (draws[1] - 10.6)).abs() < 1e-4);
        assert!((shown.death_star.0 - (draws[2] - 21.2)).abs() < 1e-4);
        assert!((shown.death_star.1 - (draws[3] - 21.2)).abs() < 1e-4);
        let mut stream = BackdropStream::from_clock(clock);
        let first = [42, 42].map(|bound| stream.draw(bound) as f32);
        let hidden = BackdropOffsets::from_clock(clock, 106.0, false);
        assert_eq!(hidden.planet, (0.0, 0.0));
        assert!((hidden.death_star.0 - (first[0] - 21.2)).abs() < 1e-4);
        assert!((hidden.death_star.1 - (first[1] - 21.2)).abs() < 1e-4);
    }

    // Source: A0 TAC-01-C008; the fit is in tactical_assets.rs
    // (the_umgul_planet_framing_is_one_of_the_originals_draws).
    #[test]
    fn the_umgul_a0_clock_draws_the_planets_sixth_and_fifth_offsets() {
        let offsets = BackdropOffsets::from_clock(UMGUL_A0_BACKDROP_CLOCK, 106.0, true);
        assert!(
            (offsets.planet.0 - (6.0 - 10.6)).abs() < 1e-4,
            "{offsets:?}"
        );
        assert!(
            (offsets.planet.1 - (5.0 - 10.6)).abs() < 1e-4,
            "{offsets:?}"
        );
    }

    // Source: FUN_0061b120 is time(NULL), seconds since 1970.
    #[test]
    fn the_backdrop_clock_reads_seconds_since_1970() {
        assert!(wall_clock_seconds() > 1_700_000_000);
    }

    // Source: FUN_005f5750 seeds from time(NULL): two scenes built a second
    // apart are framed differently.
    #[test]
    fn the_backdrop_framing_follows_the_clock_second() {
        let framings = (0..20)
            .map(|second| BackdropOffsets::from_clock(1_800_000_000 + second, 106.0, true).planet)
            .collect::<Vec<_>>();
        assert!(framings.windows(2).any(|pair| pair[0] != pair[1]));
        assert_eq!(
            BackdropOffsets::from_clock(1_800_000_007, 106.0, true),
            BackdropOffsets::from_clock(1_800_000_007, 106.0, true)
        );
    }
}
