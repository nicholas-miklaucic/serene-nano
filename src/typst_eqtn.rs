use typst::diag::SourceError;
use typst::geom::{Axis, RgbaColor, Size};
use typst::syntax::{ErrorPos, Source};
use typst_library::text::lorem;


pub(crate) fn my_lorem(num:usize)->String{
    //Testing if I got the typst library in properly
    lorem(num).to_string()
}