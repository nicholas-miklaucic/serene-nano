use comemo::Prehashed;
use once_cell::sync::Lazy;
use poise::structs;
use std::cell::{RefCell, RefMut};
use std::collections::HashMap;
use std::convert::TryInto;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use typst::diag::{FileError, FileResult, Severity, SourceDiagnostic};
use typst::diag::{PackageError, PackageResult};
use typst::doc::Document;
use typst::eval::{Bytes, Library, Tracer};
use typst::font::{Font, FontBook};
use typst::geom::Size;
use typst::syntax::{FileId, PackageSpec, Source};

/*
TODO

Okay, time for a good rewrite

Usage should be

render_str(s: String)-> Result<Vec<u8>, RenderErrors>
The above essentially needs to call typst::compile

But before that, the string has to be modified to include all the preambles

*/

struct WithoutSource {
    library: Prehashed<Library>,
    fontbook: Prehashed<FontBook>,
    fonts: Vec<Font>,
    choices: CustomisePage,
    files: HashMap<FileId, FileEntry>,
}

struct WithSource {
    ws: WithoutSource,
    source: Source,
}

impl typst::World for WithSource {
    fn library(&self) -> &Prehashed<Library> {
        &self.ws.library
    }

    fn book(&self) -> &Prehashed<FontBook> {
        &self.ws.fontbook
    }

    fn main(&self) -> Source {
        self.source.clone()
    }

    #[doc = " Try to access the specified source file."]
    #[doc = ""]
    #[doc = " The returned `Source` file\'s [id](Source::id) does not have to match the"]
    #[doc = " given `id`. Due to symlinks, two different file id\'s can point to the"]
    #[doc = " same on-disk file. Implementors can deduplicate and return the same"]
    #[doc = " `Source` if they want to, but do not have to."]
    fn source(&self, id: FileId) -> FileResult<Source> {
        let Some(p) = self.ws.files.get(&id) else {
            todo!();
        };
        p.source
    }

    #[doc = " Try to access the specified file."]
    fn file(&self, id: FileId) -> FileResult<Bytes> {
        todo!()
    }

    #[doc = " Try to access the font with the given index in the font book."]
    fn font(&self, index: usize) -> Option<Font> {
        todo!()
    }

    fn today(&self, offset: Option<i64>) -> Option<Datetime> {
        None
    }
}

pub(crate) fn better_render(s: &str) -> Result<Document, RenderErrors> {
    let source = TYPST_BASE.preamble() + s;

    let to_compile = ToCompile::new(TYPST_BASE, source.clone());
    let mut tracer = Tracer::default();

    typst::compile(&to_compile, &mut tracer).map_err(|errs| format_diagnostics(&errs))
}

struct FileEntry {
    bytes: Bytes,
    source: Option<Source>,
}

impl FileEntry {
    fn source(&mut self, id: FileId) -> FileResult<Source> {
        // Fallible `get_or_insert`.
        let source = if let Some(source) = &self.source {
            source
        } else {
            let contents = std::str::from_utf8(&self.bytes).map_err(|_| FileError::InvalidUtf8)?;
            // Defuse the BOM!
            let contents = contents.trim_start_matches('\u{feff}');
            let source = Source::new(id, contents.into());
            self.source.insert(source)
        };
        Ok(source.clone())
    }
}

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


#import \"@preview/whalogen:0.1.0\": *
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

#let di = [#math.dif i];
#let du = [#math.dif u];
#let dr = [#math.dif r];
#let ds = [#math.dif s];
#let dt = [#math.dif t];
#let dx = [#math.dif x];
#let dx = [#math.dif x];
#let dy = [#math.dif y];
#let dz = [#math.dif z];

#let int = [#sym.integral]
#let iint = [#sym.integral.double]
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
    files: RefCell<HashMap<FileId, FileEntry>>,
}

impl ToCompile {
    pub(crate) fn new(te: Arc<TypstEssentials>, source: String) -> Self {
        ToCompile {
            typst_essentials: te,
            source: Source::detached(source),
            time: time::OffsetDateTime::now_utc(),
            files: RefCell::new(HashMap::new()),
        }
    }
}

pub(crate) fn format_diagnostics(errs: &[SourceDiagnostic]) -> RenderErrors {
    //TODO: Change this for better error messages with hints

    let mut string_errors = Vec::new();
    for e in errs {
        if matches!(e.severity, Severity::Warning) {
            continue;
        }
        string_errors.push(e.message.to_string());
    }
    RenderErrors::SourceError(string_errors)
}

impl typst::World for ToCompile {
    fn book(&self) -> &Prehashed<FontBook> {
        &self.typst_essentials.fontbook
    }

    /// Returns the system path of the unpacked package.
    fn file(&self, id: FileId) -> FileResult<Bytes> {
        //TODO: This needs to be fixed, for the imports
        let Some(package) = id.package() else {
            return Err(typst::diag::FileError::Other(None));
        };
        if let Ok(ent) = RefMut::filter_map(self.files.borrow_mut(), |file| file.get_mut(&id)) {
            return Ok(ent.bytes.clone());
        }
        let package_directory = format!("./packages/{}-{}", package.name, package.version);
        let pt = std::path::Path::new(&package_directory);
        let temp = id.vpath().resolve(pt).unwrap();
        let netwmp = temp.into_os_string();
        let temp = id.vpath().resolve(pt).unwrap();
        println!("{netwmp:?}");
        match fs::read(&temp) {
            Ok(a) => {
                return Ok(self
                    .files
                    .borrow_mut()
                    .entry(id)
                    .or_insert(FileEntry {
                        bytes: a.into(),
                        source: None,
                    })
                    .bytes
                    .clone());
            }
            Err(q) => {
                println!("fuck");
            }
        }

        Err(typst::diag::FileError::Other(None))
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

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.source.id() {
            Ok(self.source.clone())
        } else {
            self.files
        }
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
    SourceError(Vec<String>),
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
                            + se
                            + "\n")
                )
            }
        }
    }
}

impl std::error::Error for RenderErrors {}

pub(crate) static TYPST_BASE: Arc<TypstEssentials> = Arc::new(TypstEssentials::new());
