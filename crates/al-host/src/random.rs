//! Randomness for `Random` and `Randomize`: .NET's seeded `System.Random`, as the legacy
//! subtractive generator (Knuth). BC keeps one per session (`ALSystemNumeric.ALRandomize(int)` is
//! `new Random(seed)`), and .NET 6 and later keep the legacy algorithm for an explicit seed, so
//! `Randomize(seed)` followed by `Random(n)` gives the same numbers here as in BC.
//!
//! The generator is a pure function of its seed: no clock, no OS entropy (ADR-006). What the
//! session seeds it with before AL calls `Randomize(seed)` is the host's choice
//! ([`crate::Host::random_seed`]); BC seeds it from the clock, which AL cannot observe beyond the
//! range of the numbers (docs/owner-review.md).

const MBIG: i32 = i32::MAX;
const MSEED: i32 = 161_803_398;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NetRandom {
    seed_array: [i32; 56],
    inext: usize,
    inextp: usize,
}

impl NetRandom {
    /// `new Random(seed)`.
    pub fn new(seed: i32) -> NetRandom {
        let mut a = [0i32; 56];
        let subtraction = if seed == i32::MIN { i32::MAX } else { seed.abs() };
        let mut mj = MSEED.wrapping_sub(subtraction);
        a[55] = mj;
        let mut mk = 1i32;
        for i in 1..55usize {
            let ii = (21 * i) % 55;
            a[ii] = mk;
            mk = mj.wrapping_sub(mk);
            if mk < 0 {
                mk = mk.wrapping_add(MBIG);
            }
            mj = a[ii];
        }
        for _ in 1..5 {
            for i in 1..56usize {
                let n = i + 30;
                a[i] = a[i].wrapping_sub(a[1 + n % 55]);
                if a[i] < 0 {
                    a[i] = a[i].wrapping_add(MBIG);
                }
            }
        }
        NetRandom {
            seed_array: a,
            inext: 0,
            inextp: 21,
        }
    }

    fn internal_sample(&mut self) -> i32 {
        let mut next = self.inext + 1;
        let mut nextp = self.inextp + 1;
        if next >= 56 {
            next = 1;
        }
        if nextp >= 56 {
            nextp = 1;
        }
        let mut ret = self.seed_array[next].wrapping_sub(self.seed_array[nextp]);
        if ret == MBIG {
            ret -= 1;
        }
        if ret < 0 {
            ret = ret.wrapping_add(MBIG);
        }
        self.seed_array[next] = ret;
        self.inext = next;
        self.inextp = nextp;
        ret
    }

    /// `Next()`: 0 to `i32::MAX - 1`.
    pub fn next_int(&mut self) -> i32 {
        self.internal_sample()
    }

    /// `Next(max)` for `max > 0`: 0 up to but not including `max`.
    pub fn next_below(&mut self, max: i32) -> i32 {
        debug_assert!(max > 0);
        (f64::from(self.internal_sample()) * (1.0 / f64::from(MBIG)) * f64::from(max)) as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Reference values from `new System.Random(seed)` on .NET 8.
    #[test]
    fn matches_dotnets_seeded_random() {
        assert_eq!(NetRandom::new(1).next_int(), 534_011_718);
        assert_eq!(NetRandom::new(42).next_int(), 1_434_747_710);
        assert_eq!(NetRandom::new(0).next_int(), 1_559_595_546);
    }

    #[test]
    fn next_below_scales_the_sample() {
        let mut r = NetRandom::new(42);
        assert_eq!([r.next_below(100), r.next_below(1000), r.next_below(7)], [66, 140, 0]);
        let mut r = NetRandom::new(1);
        assert_eq!([r.next_below(100), r.next_below(1000), r.next_below(7)], [24, 110, 3]);
        assert_eq!(r.next_below(1), 0);
        assert_eq!(r.next_below(2_147_483_647), 1_412_011_072);
    }

    #[test]
    fn the_sign_of_the_seed_does_not_matter_and_min_value_acts_as_max_value() {
        assert_eq!(NetRandom::new(-1).next_int(), NetRandom::new(1).next_int());
        assert_eq!(NetRandom::new(i32::MIN).next_int(), 1_559_595_546);
        assert_eq!(NetRandom::new(i32::MAX).next_int(), 1_559_595_546);
    }
}
