//! Module to handle Nano's translation functionality.

pub(crate) mod available_langs;
pub(crate) mod detection;
pub(crate) mod translation;
pub(crate) use translation::translate;
