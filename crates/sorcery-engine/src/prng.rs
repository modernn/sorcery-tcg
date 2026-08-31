//! Versioned deterministic pseudo-random number generation.

use serde::{Deserialize, Serialize};

const MULBERRY32_INCREMENT: u32 = 0x6d2b_79f5;

/// Serializable Mulberry32 state.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrngState {
    /// Frozen algorithm identifier.
    pub algorithm: PrngAlgorithm,
    /// Number of consumed values.
    pub draws: u64,
    /// Current 32-bit word.
    pub word: u32,
}

/// Frozen PRNG algorithm identifier.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum PrngAlgorithm {
    /// The current deterministic kernel.
    #[serde(rename = "mulberry32-v1")]
    Mulberry32V1,
}

impl PrngState {
    /// Creates a stream from an unsigned 32-bit seed.
    #[must_use]
    pub const fn new(seed: u32) -> Self {
        Self {
            algorithm: PrngAlgorithm::Mulberry32V1,
            draws: 0,
            word: seed,
        }
    }

    /// Returns the next value and advances the stream.
    #[must_use]
    pub fn draw_u32(&mut self) -> u32 {
        self.word = self.word.wrapping_add(MULBERRY32_INCREMENT);
        let mut value = self.word;
        value = (value ^ (value >> 15)).wrapping_mul(value | 1);
        value ^= value.wrapping_add((value ^ (value >> 7)).wrapping_mul(value | 0x3d));
        value ^= value >> 14;
        self.draws += 1;
        value
    }
}

#[cfg(test)]
mod tests {
    use super::PrngState;

    #[test]
    fn mulberry32_should_match_existing_five_draw_vector() {
        let mut state = PrngState::new(0);
        let values = std::array::from_fn::<_, 5, _>(|_| state.draw_u32());

        assert_eq!(
            values,
            [
                1_144_304_738,
                1_416_247,
                958_946_056,
                627_933_444,
                2_007_157_716
            ]
        );
        assert_eq!(state.word, 567_894_473);
        assert_eq!(state.draws, 5);
    }
}
