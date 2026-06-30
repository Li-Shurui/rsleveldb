pub struct Random {
    seed: u32,
}

impl Random {
    pub fn new(s: u32) -> Self {
        let s = s & 0x7fffffff;
        let s = 
            if s == 0 || s == 2147483647 {
                1
            } else {
                s
            };
        Self { seed: s}
    }

    pub fn next(&mut self) -> u32 {
        const M: u64 = 2147483647;  // 2^31-1
        const A: u64 = 16807;       // bits 14, 8, 7, 5, 2, 1, 0
        // We are computing
        //       seed_ = (seed_ * A) % M,    where M = 2^31-1
        //
        // seed_ must not be zero or M, or else all subsequent computed values
        // will be zero or M respectively.  For all other values, seed_ will end
        // up cycling through every number in [1,M-1]
        let product = self.seed as u64 * A;

        // Compute (product % M) using the fact that ((x << 31) % M) == x.
        self.seed = ((product >> 31) + (product & M)) as u32;
        // The first reduction may overflow by 1 bit, so we may need to
        // repeat.  mod == M is not possible; using > allows the faster
        // sign-bit-based test.
        if self.seed > M as u32 {
            self.seed -= M as u32;
        }

        self.seed
    }

    // Returns a uniformly distributed value in the range [0..n-1]
    // REQUIRES: n > 0
    pub fn uniform(&mut self, n: u32) -> u32 { self.next() % n }

    // Randomly returns true ~"1/n" of the time, and false otherwise.
    // REQUIRES: n > 0
    pub fn one_in(&mut self, n: u32) -> bool { self.next() % n == 0 }

    // Skewed: pick "base" uniformly from range [0,max_log] and then
    // return "base" random bits.  The effect is to pick a number in the
    // range [0,2^max_log-1] with exponential bias towards smaller numbers.
    pub fn skewed(&mut self, max_log: u32) -> u32 { 
        let x = 1 << self.uniform(max_log + 1);
        self.uniform(x)
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_normalizes_zero_seed() {
        let mut r1 = Random::new(0);
        let mut r2 = Random::new(1);
        assert_eq!(r1.next(), r2.next());
    }

    #[test]
    fn test_new_normalizes_max_seed() {
        let mut r1 = Random::new(2147483647);
        let mut r2 = Random::new(1);
        assert_eq!(r1.next(), r2.next());
    }

    #[test]
    fn test_new_normalizes_high_seed() {
        let mut r1 = Random::new(0xFFFFFFFF);
        let mut r2 = Random::new(1);
        assert_eq!(r1.next(), r2.next());
    }

    #[test]
    fn test_next_returns_known_sequence() {
        let mut r = Random::new(1);
        assert_eq!(r.next(), 16807);
        assert_eq!(r.next(), 282475249);
        assert_eq!(r.next(), 1622650073);
        assert_eq!(r.next(), 984943658);
        assert_eq!(r.next(), 1144108930);
    }

    #[test]
    fn test_next_in_valid_range() {
        let mut r = Random::new(42);
        for _ in 0..10000 {
            let n = r.next();
            assert!(n >= 1 && n <= 2147483647, "next() returned {}", n);
        }
    }

    #[test]
    fn test_next_produces_different_values() {
        let mut r = Random::new(1);
        let mut prev = r.next();
        for _ in 0..1000 {
            let curr = r.next();
            // For a PRNG with period 2^31-2, consecutive values should differ
            // (except in the extremely unlikely case of hitting a fixed point,
            // which does not exist for Park-Miller with valid seeds).
            assert_ne!(curr, prev, "next() produced same value twice in a row");
            prev = curr;
        }
    }

    #[test]
    fn test_deterministic_sequence() {
        let mut r1 = Random::new(12345);
        let mut r2 = Random::new(12345);
        for _ in 0..100 {
            assert_eq!(r1.next(), r2.next());
        }
    }

    #[test]
    fn test_different_seeds_diverge() {
        let mut r1 = Random::new(1);
        let mut r2 = Random::new(2);
        // Different seeds should produce different sequences immediately
        // (or at least very quickly)
        let mut same = true;
        for _ in 0..10 {
            if r1.next() != r2.next() {
                same = false;
                break;
            }
        }
        assert!(!same, "different seeds produced identical sequences");
    }

    #[test]
    fn test_uniform_bounds() {
        let mut r = Random::new(1);
        for n in 1..100 {
            for _ in 0..100 {
                let v = r.uniform(n);
                assert!(v < n, "uniform({}) returned {}", n, v);
            }
        }
    }

    #[test]
    fn test_uniform_always_zero_for_one() {
        let mut r = Random::new(7);
        for _ in 0..100 {
            assert_eq!(r.uniform(1), 0);
        }
    }

    #[test]
    fn test_uniform_covers_odd_range() {
        let mut r = Random::new(99);
        let n = 5; // odd n ensures next() % n can cover all residues
        let mut mask = 0u32;
        for _ in 0..1000 {
            mask |= 1 << r.uniform(n);
        }
        assert_eq!(mask, (1 << n) - 1, "uniform did not cover full range");
    }

    #[test]
    fn test_one_in_always_true_for_one() {
        let mut r = Random::new(9);
        for _ in 0..100 {
            assert!(r.one_in(1));
        }
    }

    #[test]
    fn test_one_in_probability_for_three() {
        let mut r = Random::new(11);
        let mut count = 0;
        for _ in 0..100000 {
            if r.one_in(3) { count += 1; }
        }
        // Expect ~1/3 = 33333, allow 5% tolerance
        assert!(count > 31000 && count < 36000, "count was {}", count);
    }

    #[test]
    fn test_one_in_probability_for_five() {
        let mut r = Random::new(13);
        let mut count = 0;
        for _ in 0..100000 {
            if r.one_in(5) { count += 1; }
        }
        assert!(count > 18000 && count < 22000, "count was {}", count);
    }

    #[test]
    fn test_skewed_bounds() {
        let mut r = Random::new(17);
        for max_log in 0..16 {
            for _ in 0..100 {
                let v = r.skewed(max_log);
                let max = 1u32 << max_log;
                assert!(v < max, "skewed({}) returned {}", max_log, v);
            }
        }
    }

    #[test]
    fn test_skewed_zero() {
        let mut r = Random::new(19);
        assert_eq!(r.skewed(0), 0);
    }

    #[test]
    fn test_skewed_bias_towards_small() {
        let mut r = Random::new(23);
        let mut sum = 0u64;
        for _ in 0..10000 {
            sum += r.skewed(8) as u64;
        }
        let avg = sum as f64 / 10000.0;
        // Uniform [0, 255] has expectation 127.5.
        // skewed(8) should be significantly smaller.
        assert!(avg < 80.0, "average was {}, expected < 80 due to skew", avg);
    }

    #[test]
    fn test_skewed_large_max_log() {
        let mut r = Random::new(29);
        // max_log=20 -> range [0, 2^20)
        let v = r.skewed(20);
        assert!(v < (1 << 20), "skewed(20) returned {}", v);
    }

    #[test]
    fn test_uniform_then_skewed_interaction() {
        let mut r1 = Random::new(31);
        let mut r2 = Random::new(31);
        // Both do uniform(100) then skewed(4)
        assert_eq!(r1.uniform(100), r2.uniform(100));
        assert_eq!(r1.skewed(4), r2.skewed(4));
    }
}
