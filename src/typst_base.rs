use std::sync::Arc;
use std::convert::TryInto;
use typst::util::Buffer;
use typst::font::{Font, FontBook};
use typst::diag::SourceError;
use typst::syntax::{Source, SourceId};
use typst::eval::Library;
use typst::geom::Size;
use comemo::Prehashed;

pub(crate)trait Preamble{
    fn preamble(&self)->String;
}

//TODO, allow changing the customisation
enum PageSize{
    Default, Auto
}

impl Preamble for PageSize {
    fn preamble(&self)->String {
        match &self {
            PageSize::Auto=>  "#set page(width: auto, height: auto, margin: 10pt)\n".to_string(),
            PageSize::Default => "".to_string()
        }
    }
}
enum Theme{
    Dark, Light
}


impl Preamble for Theme {
    fn preamble(&self)->String {
        match self {
            Theme::Light=>"#set page(fill: white)\n".to_string(),
            Theme::Dark=> "#set page(fill: rgb(49, 51, 56))\n#set text(fill: rgb(219, 222, 225))\n".to_string(),
        }
    }
}

pub(crate)struct CustomisePage{
    page_size: PageSize,
    theme: Theme
}

impl Preamble for CustomisePage {
    fn preamble(&self)->String {
        self.page_size.preamble() + self.theme.preamble().as_str()
    }
}

pub(crate) struct TypstEssentials {
    library: Prehashed<Library>,
    fontbook: Prehashed<FontBook>,
    fonts: Vec<Font>,
    choices: CustomisePage   
}

fn get_fonts() -> Vec<Font> {
    std::fs::read_dir("fonts")
        .unwrap()
        .map(Result::unwrap)
        .flat_map(|entry| {
            let bytes = std::fs::read(entry.path()).unwrap();
            let buffer = Buffer::from(bytes);
            Font::iter(buffer)
        })
        .collect()
}

impl TypstEssentials {
    pub(crate) fn new() -> Self {
        let fonts = get_fonts();

        Self {
            library: Prehashed::new(typst_library::build()),
            fontbook: Prehashed::new(FontBook::from_fonts(&fonts)),
            fonts,
            choices: CustomisePage { 
                page_size: PageSize::Auto, theme: Theme::Dark 
            }
        }
    }
}

impl Preamble for TypstEssentials {
    fn preamble(&self)->String {
        self.choices.preamble()
    }
}

const DESIRED_RESOLUTION: f32 = 1000.0;
const MAX_SIZE: f32 = 10000.0;
const MAX_PIXELS_PER_POINT: f32 = 5.0;

pub(crate) fn determine_pixels_per_point(size: Size) -> Result<f32, RenderErrors> {
    let x = size.x.to_pt() as f32;
    let y = size.y.to_pt() as f32;

    if x > MAX_SIZE || y > MAX_SIZE {
        Err(RenderErrors::PageSizeTooBig)
    } else {
        let area = x * y;
        Ok((DESIRED_RESOLUTION / area.sqrt()).min(MAX_PIXELS_PER_POINT))
    }
}

pub(crate)struct ToCompile {
    typst_essentials: Arc<TypstEssentials>,
    source: Source,
    time: time::OffsetDateTime,
}

fn string2source(source: String) -> Source {
    Source::new(SourceId::from_u16(0), "not needed".as_ref(), source)
}

impl ToCompile {
    pub(crate)fn new(te: Arc<TypstEssentials>, source: String) -> Self {
        ToCompile {
            typst_essentials:te,
            source: string2source(source),
            time: time::OffsetDateTime::now_utc(),
        }
    }
}

impl typst::World for ToCompile {
    fn book(&self) -> &Prehashed<FontBook> {
        &self.typst_essentials.fontbook
    }

    fn file(&self, path: &std::path::Path) -> typst::diag::FileResult<Buffer> {
        Err(typst::diag::FileError::NotFound(path.into()))
    }

    fn font(&self, id: usize) -> Option<Font> {
        self.typst_essentials.fonts.get(id).cloned()
    }
    fn library(&self) -> &Prehashed<Library> {
        &self.typst_essentials.library
    }
    fn main(&self) -> &Source {
        &self.source
    }
    fn resolve(&self, path: &std::path::Path) -> typst::diag::FileResult<SourceId> {
        Err(typst::diag::FileError::NotFound(path.into()))
    }

    fn source(&self, _id: SourceId) -> &Source {
        &self.source
    }
    fn today(&self, offset: Option<i64>) -> Option<typst::eval::Datetime> {
        let offset = offset.unwrap_or(0);
        let offset = time::UtcOffset::from_hms(offset.try_into().ok()?, 0, 0).ok()?;
        let time = self.time.checked_to_offset(offset)?;
        Some(typst::eval::Datetime::Date(time.date()))
    }
}

pub(crate) enum RenderErrors {
    SourceError(Box<Vec<SourceError>>),
    NoPageError,
    PageSizeTooBig,
    
    NotSourceError,
}

