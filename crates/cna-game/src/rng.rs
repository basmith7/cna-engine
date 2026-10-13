//! The game's dice: a seeded ChaCha8 stream, serialised as the seed and
//! the number of words drawn, so a loaded game rolls on exactly as the
//! live one would.

use rand_chacha::ChaCha8Rng;
use rand_core::{RngCore, SeedableRng};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(from = "Saved", into = "Saved")]
pub struct GameRng {
    seed: u64,
    draws: u64,
    rng: ChaCha8Rng,
}

#[derive(Serialize, Deserialize)]
struct Saved {
    seed: u64,
    draws: u64,
}

impl From<Saved> for GameRng {
    fn from(s: Saved) -> GameRng {
        let mut r = GameRng::new(s.seed);
        r.rng.set_word_pos(u128::from(s.draws));
        r.draws = s.draws;
        r
    }
}

impl From<GameRng> for Saved {
    fn from(r: GameRng) -> Saved {
        Saved {
            seed: r.seed,
            draws: r.draws,
        }
    }
}

impl PartialEq for GameRng {
    fn eq(&self, other: &GameRng) -> bool {
        self.seed == other.seed && self.draws == other.draws
    }
}

impl Eq for GameRng {}

impl GameRng {
    pub fn new(seed: u64) -> GameRng {
        GameRng {
            seed,
            draws: 0,
            rng: ChaCha8Rng::seed_from_u64(seed),
        }
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    pub fn draws(&self) -> u64 {
        self.draws
    }

    fn word(&mut self) -> u32 {
        self.draws += 1;
        self.rng.next_u32()
    }

    /// One die, 1–6, unbiased: words at or above the largest multiple of
    /// six are drawn again.
    pub fn d6(&mut self) -> u8 {
        const LIMIT: u32 = u32::MAX - (u32::MAX % 6);
        loop {
            let w = self.word();
            if w < LIMIT {
                return (w % 6) as u8 + 1;
            }
        }
    }

    /// Two dice read sequentially, 11–66 (SPI 11.4): the first die is the
    /// tens.
    pub fn two_dice_sequential(&mut self) -> u8 {
        let a = self.d6();
        let b = self.d6();
        a * 10 + b
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_rolls() {
        let mut a = GameRng::new(7);
        let mut b = GameRng::new(7);
        let x: Vec<u8> = (0..20).map(|_| a.d6()).collect();
        let y: Vec<u8> = (0..20).map(|_| b.d6()).collect();
        assert_eq!(x, y);
        assert!(x.iter().all(|&d| (1..=6).contains(&d)));
    }

    #[test]
    fn a_loaded_rng_continues_where_the_live_one_was() {
        let mut live = GameRng::new(42);
        for _ in 0..13 {
            live.two_dice_sequential();
        }
        let json = serde_json::to_string(&live).unwrap();
        assert_eq!(json, format!(r#"{{"seed":42,"draws":{}}}"#, live.draws()));
        let mut loaded: GameRng = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded, live);
        for _ in 0..50 {
            assert_eq!(loaded.d6(), live.d6());
        }
        assert_eq!(loaded, live);
    }

    #[test]
    fn sequential_reading_is_tens_then_units() {
        let mut r = GameRng::new(1);
        for _ in 0..200 {
            let v = r.two_dice_sequential();
            assert!(
                (1..=6).contains(&(v / 10)) && (1..=6).contains(&(v % 10)),
                "{v}"
            );
        }
    }
}
