pub use slotmap::secondary::{Iter, Keys, Values};

use crate::NodeId;

pub type SecondaryMap<T> = slotmap::SecondaryMap<NodeId, T>;
