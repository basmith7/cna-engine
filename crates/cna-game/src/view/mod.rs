//! What each side may know (3.61, limited intelligence): the engine, not a
//! host, decides it, so the browser and the server show the same thing.

mod aliases;
mod events;

pub use aliases::Aliases;
pub use events::events_for;
