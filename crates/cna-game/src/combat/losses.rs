//! TOE losses (SPI 12.45, 14.4x, 15.8x): the engine chooses which points
//! go, standing in for the owner — the entry that absorbs most per point
//! first — until a pending choice is worth its weight. A piece at zero
//! leaves the map.

use super::strength::Part;
use crate::data::Data;
use crate::event::Event;
use crate::piece::{Location, PieceId};
use crate::state::GameState;

/// A piece and how many of its TOE points may be taken.
#[derive(Debug, Clone)]
pub(crate) struct Share {
    pub piece: PieceId,
    pub points: u32,
}

impl GameState {
    /// Destroys up to `points` TOE points of `id`, largest entry first;
    /// returns what went.
    pub(crate) fn take_loss(&mut self, id: &PieceId, points: u32, events: &mut Vec<Event>) -> u32 {
        let mut taken = 0;
        while taken < points {
            let piece = &self.pieces[id];
            let listed: u32 = piece.weapons.iter().map(|w| w.points).sum();
            let unlisted = piece.toe.saturating_sub(listed);
            let biggest = piece
                .weapons
                .iter()
                .max_by_key(|w| w.points)
                .filter(|w| w.points > unlisted)
                .map(|w| w.weapon.clone());
            if !self.take_loss_entry(id, biggest.as_deref()) {
                break;
            }
            taken += 1;
        }
        self.note_losses(id, taken, events);
        taken
    }

    /// Takes one TOE point off the named weapon entry, or off the ID
    /// code's unlisted points; false when there is none to take.
    pub(crate) fn take_loss_entry(&mut self, id: &PieceId, weapon: Option<&str>) -> bool {
        let piece = self.pieces.get_mut(id).expect("a piece");
        if piece.toe == 0 {
            return false;
        }
        match weapon {
            Some(name) => {
                let Some(w) = piece
                    .weapons
                    .iter_mut()
                    .find(|w| w.weapon == name && w.points > 0)
                else {
                    return false;
                };
                w.points -= 1;
            }
            None => {
                let listed: u32 = piece.weapons.iter().map(|w| w.points).sum();
                if piece.toe <= listed {
                    return false;
                }
            }
        }
        piece.toe -= 1;
        true
    }

    /// Records `points` lost by `id`, and its elimination at zero.
    pub(crate) fn note_losses(&mut self, id: &PieceId, points: u32, events: &mut Vec<Event>) {
        if points == 0 {
            return;
        }
        let piece = self.pieces.get_mut(id).expect("a piece");
        events.push(Event::Losses {
            piece: id.clone(),
            points,
            toe: piece.toe,
        });
        if piece.toe == 0 {
            piece.location = Location::Eliminated;
            events.push(Event::Eliminated { piece: id.clone() });
            self.release_attached(id);
        }
    }

    /// Removes TOE points from `shares` until their `rate` (raw rating,
    /// armour protection, vulnerability) absorbs at least `needed`, the
    /// highest rate first; points rated zero absorb nothing and stay.
    /// Returns (absorbed, TOE points removed).
    pub(crate) fn absorb(
        &mut self,
        data: &Data,
        shares: &mut [Share],
        rate: impl Fn(&Part) -> u32,
        needed: u32,
        events: &mut Vec<Event>,
    ) -> (u32, u32) {
        let mut absorbed = 0;
        let mut removed = 0;
        let mut per_piece: Vec<(PieceId, u32)> = Vec::new();
        while absorbed < needed {
            let mut best: Option<(usize, Option<String>, u32)> = None;
            for (i, share) in shares.iter().enumerate() {
                if share.points == 0 {
                    continue;
                }
                let piece = &self.pieces[&share.piece];
                for part in self.parts(data, piece) {
                    let r = rate(&part);
                    if r == 0 || part.points == 0 {
                        continue;
                    }
                    if best.as_ref().is_none_or(|b| r > b.2) {
                        best = Some((i, part.weapon.clone(), r));
                    }
                }
            }
            let Some((i, weapon, r)) = best else {
                break;
            };
            let id = shares[i].piece.clone();
            if !self.take_loss_entry(&id, weapon.as_deref()) {
                break;
            }
            shares[i].points -= 1;
            absorbed += r;
            removed += 1;
            match per_piece.iter_mut().find(|(p, _)| *p == id) {
                Some((_, n)) => *n += 1,
                None => per_piece.push((id, 1)),
            }
        }
        for (id, n) in per_piece {
            self.note_losses(&id, n, events);
        }
        (absorbed, removed)
    }
}
