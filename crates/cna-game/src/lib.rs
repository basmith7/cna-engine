//! The game state for one scenario, headless: pieces, the clock, orders,
//! events and the log, with movement; combat follows in slice 5.

mod air_lift;
mod ammo;
mod arrival;
mod attachment;
pub mod breakdown;
pub mod clock;
pub mod combat;
mod convoy;
mod cycle;
pub mod data;
mod determinism;
pub mod event;
mod fuel;
mod golden;
pub mod log;
pub mod movement;
mod off_map;
pub mod order;
mod organisation;
mod phases;
pub mod piece;
mod place;
mod prisoners;
mod rail;
pub mod reach;
mod reaction;
mod repair;
mod reserve;
pub mod rng;
pub mod setup;
pub mod state;
mod supply;
pub mod victory;
pub mod zoc;
