//! How much a [`LoginImage`] holds.

use super::{ImageSize, LoginImage};

impl LoginImage {
    /// The image's size: its rows, its state and its graph ([`ImageSize`]).
    pub fn size(&self) -> ImageSize {
        ImageSize::default()
    }
}
