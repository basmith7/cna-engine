//! The game state for one scenario, headless: pieces, the clock, orders,
//! events and the log. Movement and combat follow in later slices.

pub mod clock;
mod cycle;
pub mod data;
pub mod event;
pub mod log;
pub mod movement;
pub mod order;
mod phases;
pub mod piece;
mod reaction;
mod reserve;
mod place;
pub mod rng;
pub mod setup;
pub mod state;
pub mod zoc;
