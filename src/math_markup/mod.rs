//! Utilities to deal with math markup.

mod preferred_markup;
mod typst_base;
mod typst_main;

pub(crate) use preferred_markup::{
    get_preferred_markup, set_default_math_markup, set_preferred_markup,
};
pub(crate) use typst_main::{catch_typst_message, render_math, typst};
