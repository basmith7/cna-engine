//! TOE losses (SPI 12.45, 15.8x): points come off the piece's largest
//! entry first — the engine's choice standing in for the owner's, until
//! a pending choice is worth its weight — and a piece at zero leaves
//! the map.

use crate::event::Event;
use crate::piece::{Location, PieceId};
use crate::state::GameState;

impl GameState {
    /// Destroys up to `points` TOE points of `id`; returns what went.
    pub(crate) fn take_loss(&mut self, id: &PieceId, points: u32, events: &mut Vec<Event>) -> u32 {
        let piece = self.pieces.get_mut(id).expect("a piece");
        let mut taken = 0;
        while taken < points && piece.toe > 0 {
            let listed: u32 = piece.weapons.iter().map(|w| w.points).sum();
            let unlisted = piece.toe.saturating_sub(listed);
            let biggest = piece
                .weapons
                .iter_mut()
                .max_by_key(|w| w.points)
                .filter(|w| w.points > unlisted);
            if let Some(w) = biggest {
                w.points -= 1;
            }
            piece.toe -= 1;
            taken += 1;
        }
        if taken == 0 {
            return 0;
        }
        events.push(Event::Losses {
            piece: id.clone(),
            points: taken,
            toe: piece.toe,
        });
        if piece.toe == 0 {
            piece.location = Location::Eliminated;
            events.push(Event::Eliminated { piece: id.clone() });
        }
        taken
    }
}
