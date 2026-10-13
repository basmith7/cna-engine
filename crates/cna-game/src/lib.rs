//! The game state for one scenario, headless: pieces, the clock, orders,
//! events and the log. Movement and combat follow in later slices.

pub mod clock;
pub mod data;
pub mod event;
pub mod order;
pub mod piece;
pub mod rng;
pub mod setup;
pub mod state;
