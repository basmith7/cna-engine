//! The game state for one scenario, headless: pieces, the clock, orders,
//! events and the log, with movement; combat follows in slice 5.

mod arrival;
pub mod clock;
pub mod combat;
mod cycle;
pub mod data;
pub mod event;
mod determinism;
mod golden;
pub mod log;
pub mod movement;
mod off_map;
pub mod order;
mod phases;
pub mod piece;
mod place;
mod reaction;
mod reserve;
pub mod rng;
pub mod setup;
pub mod state;
pub mod victory;
pub mod zoc;
