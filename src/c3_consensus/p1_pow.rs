//! Proof of Work provides security to the blockchain by requiring block authors
//! to expend a real-world scarce resource, namely energy, in order to author a valid block.
//!
//! This is the same logic we implemented previously. Here we re-implement it in the
//! generic consensus framework that we will use throughout the rest of the chapter.

use crate::hash;
use super::{Consensus, Hash, Header};

/// A Proof of Work consensus engine. This is the same consensus logic that we
/// implemented in the previous chapter. Here we simply re-implement it in the
/// consensus framework that will be used throughout this chapter.
pub struct Pow {
    threshold: u64,
}

impl Consensus for Pow {
    type Digest = u64;

    /// Check that the provided header's hash is below the required threshold.
    /// This does not rely on the parent digest at all.
    fn validate(&self, _: &Self::Digest, header: &Header<Self::Digest>) -> bool {

        let header_hash = hash(&header);
        header_hash < self.threshold
    }

    /// Mine a new PoW seal for the partial header provided.
    /// This does not rely on the parent digest at all.
    fn seal(&self, _: &Self::Digest, partial_header: Header<()>) -> Option<Header<Self::Digest>> {
        for nonce in 0..u64::MAX {
            let header_hash = hash(&partial_header);
            if header_hash < self.threshold {
                return Some(Header {
                    parent: partial_header.parent,
                    height: partial_header.height,
                    extrinsics_root: partial_header.extrinsics_root,
                    state_root: partial_header.state_root,
                    consensus_digest: nonce,
                });
            }
        }
        None
    }
}

/// Create a PoW consensus engine that has a difficulty threshold such that roughly 1 in 100 blocks
/// with randomly drawn nonces will be valid. That is: the threshold should be u64::max_value() / 100.
pub fn moderate_difficulty_pow() -> Pow {
    Pow {
        threshold: u64::MAX / 100,
    }
}

/// Create an instance of the PoW Consensus that behaves identically to the trivial
/// consensus implementation for `()` from the module level.
pub fn trivial_always_valid_pow() -> Pow {
    Pow {
        threshold: u64::MAX,
    }
}
