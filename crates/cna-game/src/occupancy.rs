//! Who stands in each hex. Most rules ask it of one hex at a time, and
//! a scan of every piece answers well enough; a search that asks it of
//! thousands of hexes (`reachable`) indexes the pieces once for its
//! length, since the state cannot change under it.

use crate::piece::{Piece, PieceId};
use crate::state::GameState;
use cna_data::map::HexId;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

type Index = BTreeMap<HexId, Vec<PieceId>>;

/// The index a search installs; empty otherwise. It holds nothing the
/// pieces do not, so a copy starts empty and any two compare equal.
#[derive(Default)]
pub struct HexIndex(Mutex<Option<Arc<Index>>>);

impl Clone for HexIndex {
    fn clone(&self) -> Self {
        HexIndex::default()
    }
}

impl PartialEq for HexIndex {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl Eq for HexIndex {}

impl std::fmt::Debug for HexIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("HexIndex")
    }
}

/// Removes the index when the search that installed it ends.
pub(crate) struct Indexed<'a> {
    state: &'a GameState,
    installed: bool,
}

impl Drop for Indexed<'_> {
    fn drop(&mut self) {
        if self.installed {
            *self.state.hex_index.0.lock().expect("unpoisoned") = None;
        }
    }
}

impl GameState {
    /// The pieces in `hex`, in piece order.
    pub(crate) fn pieces_at(&self, hex: &HexId) -> Vec<&Piece> {
        let index = self.hex_index.0.lock().expect("unpoisoned").clone();
        match index {
            Some(index) => index
                .get(hex)
                .map(|ids| ids.iter().map(|id| &self.pieces[id]).collect())
                .unwrap_or_default(),
            None => self
                .pieces
                .values()
                .filter(|p| p.hex() == Some(hex))
                .collect(),
        }
    }

    /// Indexes the pieces by hex until the guard drops; a search inside
    /// another keeps the outer one's index.
    pub(crate) fn index_hexes(&self) -> Indexed<'_> {
        let mut slot = self.hex_index.0.lock().expect("unpoisoned");
        if slot.is_some() {
            return Indexed {
                state: self,
                installed: false,
            };
        }
        let mut index = Index::new();
        for p in self.pieces.values() {
            if let Some(h) = p.hex() {
                index.entry(h.clone()).or_default().push(p.id.clone());
            }
        }
        *slot = Some(Arc::new(index));
        Indexed {
            state: self,
            installed: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::data::Data;
    use crate::movement::test_support::{hex, moving};
    use cna_data::scenario::Side;

    #[test]
    fn the_index_answers_as_the_scan_does_and_goes_with_its_guard() {
        let data = Data::load("grazianis-offensive").unwrap();
        let state = moving(&data, 3, Side::Cw);
        let hexes: Vec<_> = data.map.hexes().map(|h| h.id.clone()).collect();
        let scan: Vec<Vec<_>> = hexes
            .iter()
            .map(|h| state.pieces_at(h).iter().map(|p| p.id.clone()).collect())
            .collect();
        assert!(scan.iter().any(|v| v.len() > 1), "some stack");
        {
            let _indexed = state.index_hexes();
            let _nested = state.index_hexes();
            for (h, want) in hexes.iter().zip(&scan) {
                let got: Vec<_> = state.pieces_at(h).iter().map(|p| p.id.clone()).collect();
                assert_eq!(&got, want, "{h}");
            }
        }
        assert!(state.hex_index.0.lock().unwrap().is_none());
        assert_eq!(
            state.pieces_at(&hex("C4020")).len(),
            scan[hexes.iter().position(|h| h == &hex("C4020")).unwrap()].len()
        );
    }
}
