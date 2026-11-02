//! What each side may know (3.61, limited intelligence): the engine, not a
//! host, decides it, so the browser and the server show the same thing.

mod aliases;

pub use aliases::Aliases;
mod state;

pub use state::{ENEMY_PIECE_KEEPS, ENEMY_PIECE_REMOVES, view_for};
