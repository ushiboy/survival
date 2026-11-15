//! Deterministic hashing / random numbers. No global state, so world generation
//! and AI are reproducible from a seed.

/// Integer hash of a 2D coordinate. Good enough for map decoration and noise.
pub fn hash2(x: i32, y: i32, seed: u32) -> u32 {
    let mut h = seed ^ (x as u32).wrapping_mul(0x27d4_eb2d) ^ (y as u32).wrapping_mul(0x1656_67b1);
    h ^= h >> 15;
    h = h.wrapping_mul(0x85eb_ca6b);
    h ^= h >> 13;
    h = h.wrapping_mul(0xc2b2_ae35);
    h ^= h >> 16;
    h
}

/// Hash mapped to `[0, 1)`.
pub fn hash_unit(x: i32, y: i32, seed: u32) -> f32 {
    (hash2(x, y, seed) >> 8) as f32 / (1u32 << 24) as f32
}

/// Small xorshift PRNG for per-entity behaviour.
#[derive(Clone, Debug)]
pub struct Rng {
    state: u32,
}

impl Rng {
    pub fn new(seed: u32) -> Self {
        Self { state: hash2(seed as i32, 0x5eed, 0x9e37_79b9) | 1 }
    }

    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        x
    }

    /// Uniform integer in `[0, n)`.
    pub fn range(&mut self, n: u32) -> u32 {
        if n == 0 { 0 } else { self.next_u32() % n }
    }

    /// Uniform integer in `[lo, hi]`.
    pub fn between(&mut self, lo: i32, hi: i32) -> i32 {
        lo + self.range((hi - lo + 1) as u32) as i32
    }

    pub fn chance(&mut self, p: f32) -> bool {
        ((self.next_u32() >> 8) as f32 / (1u32 << 24) as f32) < p
    }
}
