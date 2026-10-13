//! The game state for one scenario, headless: pieces, the clock, orders,
//! events and the log, with movement; combat follows in slice 5.

mod arrival;
pub mod clock;
mod cycle;
pub mod data;
pub mod event;
pub mod log;
pub mod movement;
pub mod order;
mod phases;
mod off_map;
pub mod piece;
mod place;
mod reaction;
mod reserve;
pub mod rng;
pub mod setup;
pub mod state;
pub mod zoc;
