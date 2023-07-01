use comemo::Prehashed;
use std::convert::TryInto;
use std::path::Path;
use std::sync::Arc;
use typst::diag::{FileResult, SourceError};
use typst::eval::Library;
use typst::file::FileId;
use typst::font::{Font, FontBook};
use typst::geom::Size;
use typst::syntax::Source;
use typst::util::Bytes;

pub(crate) trait Preamble {
    fn preamble(&self) -> String;
}

//TODO, allow changing the customisation
#[derive(Debug, Clone, Copy)]
pub(crate) enum PageSize {
    Auto,
}

impl Preamble for PageSize {
    fn preamble(&self) -> String {
        match &self {
            PageSize::Auto => "#set page(width: auto, height: auto, margin: 10pt)\n".to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum Theme {
    Dark,
}

impl Preamble for Theme {
    fn preamble(&self) -> String {
        match self {
            //             Theme::Light => "
            // #let bg = rgb(219, 222, 225)
            // #let fg = rgb(49, 51, 56)
            // "
            //            .to_string(),
            Theme::Dark => "
#let fg = rgb(219, 222, 225)
#let bg = rgb(49, 51, 56)"
                .to_string(),
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub(crate) struct CustomisePage {
    pub(crate) page_size: PageSize,
    pub(crate) theme: Theme,
}

impl Preamble for CustomisePage {
    fn preamble(&self) -> String {
        self.page_size.preamble() + self.theme.preamble().as_str()
    }
}

pub(crate) struct TypstEssentials {
    library: Prehashed<Library>,
    fontbook: Prehashed<FontBook>,
    fonts: Vec<Font>,
    choices: CustomisePage,
}

fn get_fonts() -> Vec<Font> {
    std::fs::read_dir("fonts")
        .unwrap()
        .map(Result::unwrap)
        .flat_map(|entry| {
            let bytes = std::fs::read(entry.path()).unwrap();
            let buffer = Bytes::from(bytes);
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
                page_size: PageSize::Auto,
                theme: Theme::Dark,
            },
        }
    }
}

impl Preamble for TypstEssentials {
    fn preamble(&self) -> String {
        self.choices.preamble()
            + "
#set text(
  font: (
    \"EB Garamond 12\",
    \"DejaVu Sans Mono\"
  ),
  size: 18pt,
  number-type: \"lining\",
  number-width: \"tabular\",
  weight: \"regular\",
);
// set a size that more closely approximates the zoom-in on Discord


// Preamble starts here

#set page(
  fill: bg
)
#set text(
  fill: fg,
)

#let infty = [#sym.infinity];

#let dx = [#math.dif x];
#let dy = [#math.dif y];
#let dz = [#math.dif z];

#let int = [#sym.integral]
#let infty = [#sym.infinity]

#let mathbox(content) = {
  style(styles => {
    let size = measure(content, styles);
      block(
        radius: 0.2em,
        stroke: fg,
        inset: size.height / 1.5,
        content)})
};"
    }
}

const DESIRED_RESOLUTION: f32 = 2000.0;
const MAX_SIZE: f32 = 10000.0;
const MAX_PIXELS_PER_POINT: f32 = 15.0;

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

pub(crate) struct ToCompile {
    typst_essentials: Arc<TypstEssentials>,
    source: Source,
    time: time::OffsetDateTime,
}

fn string2source(source: String) -> Source {
    Source::new(FileId::new(None, Path::new("/")), source)
}

impl ToCompile {
    pub(crate) fn new(te: Arc<TypstEssentials>, source: String) -> Self {
        ToCompile {
            typst_essentials: te,
            source: string2source(source),
            time: time::OffsetDateTime::now_utc(),
        }
    }
}

impl typst::World for ToCompile {
    fn book(&self) -> &Prehashed<FontBook> {
        &self.typst_essentials.fontbook
    }

    fn file(&self, _id: FileId) -> FileResult<Bytes> {
        Err(typst::diag::FileError::Other)
    }

    fn font(&self, id: usize) -> Option<Font> {
        self.typst_essentials.fonts.get(id).cloned()
    }
    fn library(&self) -> &Prehashed<Library> {
        &self.typst_essentials.library
    }

    fn main(&self) -> Source {
        self.source.clone()
    }

    fn source(&self, _id: FileId) -> FileResult<Source> {
        Ok(self.source.clone())
    }
    fn today(&self, offset: Option<i64>) -> Option<typst::eval::Datetime> {
        let offset = offset.unwrap_or(0);
        let offset = time::UtcOffset::from_hms(offset.try_into().ok()?, 0, 0).ok()?;
        let time = self.time.checked_to_offset(offset)?;
        Some(typst::eval::Datetime::Date(time.date()))
    }
}

#[derive(Debug, Clone)]
pub(crate) enum RenderErrors {
    SourceError(Vec<SourceError>),
    NoPageError,
    PageSizeTooBig,
}

impl std::fmt::Display for RenderErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RenderErrors::NoPageError => {
                write!(f, "No pages found...")
            }
            RenderErrors::PageSizeTooBig => {
                write!(f, "Page too big...")
            }
            RenderErrors::SourceError(err) => {
                write!(
                    f,
                    "{}",
                    err.iter()
                        .fold(String::from("Syntax error(s):\n"), |acc, se| acc
                            + se.message.as_str()
                            + "\n")
                )
            }
        }
    }
}

impl std::error::Error for RenderErrors {}
