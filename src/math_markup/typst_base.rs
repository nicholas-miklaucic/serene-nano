use comemo::Prehashed;
use serenity::futures::lock::Mutex;
use std::cell::RefCell;
use std::io::Cursor;
use std::sync::OnceLock;
use typst::diag::{FileError, FileResult, Severity, SourceDiagnostic};
use typst::eval::Tracer;
use typst::foundations::{Bytes, Datetime};
use typst::layout::{Abs, Axes};
use typst::syntax::{FileId, Source};
use typst::text::{Font, FontBook};
use typst::Library;

//Needed for reading files for packages
#[derive(Clone)]
struct FileEntry {
    bytes: Bytes,
    source: Option<Source>,
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
/*
To explain the package layout; because this might not be obvious
packages/
    {package_name}-{version}/
        entry_point.typ -> The important file
        LICENSE -> I need to check if this is legal, milord
        typst.toml -> tells us where the entrypoint is

I do not plan on doing stuff that involves downloading packages as needed, but if needed, the packages can be downloaded from
https://packages.typst.org/{namespace}/{name}-{version}.tar.gz
namespace will be preview, as per typst 0.10.0; the tar file should just be extracted into the folder
*/

impl FileEntry {
    fn source(&mut self, id: FileId) -> FileResult<Source> {
        // Fallible `get_or_insert`.
        let src = match &self.source {
            Some(e) => e,
            None => {
                let contents =
                    std::str::from_utf8(&self.bytes).map_err(|_| FileError::InvalidUtf8)?;
                // Defuse the BOM!
                let contents = contents.trim_start_matches('\u{feff}');
                let source = Source::new(id, contents.into());
                self.source.insert(source)
            }
        };
        Ok(src.clone())
    }
}

pub(crate) struct TypstRendered {
    library: Prehashed<Library>,
    fontbook: Prehashed<FontBook>,
    fonts: Vec<Font>,
    files: RefCell<Vec<(FileId, FileEntry)>>, //This used to be a hashmap, but bruh, there are like 3 to 5 files here maximum, its easier to traverse a list; RefCell is needed because I need to mutate this sometimes.
    source: Option<Source>,
}
impl TypstRendered {
    pub(crate) fn new() -> Self {
        let fonts = get_fonts();
        Self {
            library: Prehashed::new(Library::build()),
            fontbook: Prehashed::new(FontBook::from_fonts(&fonts)),
            fonts,
            files: RefCell::new(Vec::new()),
            source: None,
        }
    }
    fn file_id(&self, file_id: &FileId) -> Option<FileEntry> {
        self.files
            .borrow()
            .iter()
            .find_map(|(a, b)| if a == file_id { Some(b.clone()) } else { None })
    }
    fn file_entry(&self, id: &FileId) -> FileResult<FileEntry> {
        //Check if file is already in vector
        if let Some(a) = self.file_id(id) {
            return Ok(a);
        }

        //Check if it is a package, should be infallible, but yeah
        let p = id
            .package()
            .ok_or(FileError::NotFound(id.vpath().as_rootless_path().into()))?;

        println!(
            "Loading new file: {}, {} - {} is the size of list of files",
            p.name.as_str(),
            id.vpath().as_rooted_path().to_str().unwrap_or("oopsie!"),
            self.files.borrow().len()
        );
        let mut dir = std::path::PathBuf::new();
        dir.push("packages");
        let package_folder = format!(
            "{}-{}.{}.{}",
            p.name, p.version.major, p.version.minor, p.version.patch
        );
        dir.push(&package_folder);

        let path = id.vpath().resolve(dir.as_path()).unwrap();
        let contents = std::fs::read(&path).map_err(|error| FileError::from_io(error, &path))?;

        let fe = FileEntry {
            bytes: contents.into(),
            source: None,
        };
        self.files.borrow_mut().push((*id, fe));
        // self.files.push((id.clone(), fe));
        //Should return the above file_entry, and there should be an easy way to do it, but i dont know;

        Ok(self.file_id(id).unwrap().clone())
    }
    fn preamble() -> String {
        let imports = r#"
            #import "@preview/whalogen:0.1.0": *
            #import "@preview/mitex:0.2.1": *        
            "#;
        let theme = r#"
            #let fg = rgb(219, 222, 225)
            #let bg = rgb(49, 51, 56)
        "#;
        let page_size = "#set page(width: auto, height: auto, margin: 10pt)\n";
        let text_options = r#"
            #set text(
                font: (
                    "EB Garamond 12",
                    "DejaVu Sans Mono"
                      ),
                size: 18pt,
                number-type: "lining",
                number-width: "tabular",
                weight: "regular",
            );
            // set a size that more closely approximates the zoom-in on Discord


            // Preamble starts here
            #set page(
              fill: bg
            )
            #set text(
              fill: fg,
            )
        "#;
        let math_remaining = r#"
            #let infty = [#sym.infinity];

            #let di = [#math.dif i];
            #let du = [#math.dif u];
            #let dv = [#math.dif v];
            #let dr = [#math.dif r];
            #let ds = [#math.dif s];
            #let dt = [#math.dif t];
            #let dx = [#math.dif x];
            #let dv = [#math.dif v];
            #let dy = [#math.dif y];
            #let dz = [#math.dif z];
            #let ddt = [#math.frac([#math.dif], dt)];
            #let ddx = [#math.frac([#math.dif], dx)];
            #let ddy = [#math.frac([#math.dif], dy)];
            #let ddz = [#math.frac([#math.dif], dz)];
            #let ddu = [#math.frac([#math.dif], du)];
            #let ddv = [#math.frac([#math.dif], dv)];
            
            
            #let int = [#sym.integral]
            #let iint = [#sym.integral.double]
            #let infty = [#sym.infinity]
            #let to = [#sym.arrow]

            #let mathbox(content) = {
              style(styles => {
                let size = measure(content, styles);
                block(
                    radius: 0.2em,
                    stroke: fg,
                    inset: size.height / 1.5,
                    content)})
            };
        "#;
        imports.to_string() + theme + page_size + text_options + math_remaining
    }
    fn add_source(&mut self, src: &str) {
        self.source = Some(Source::detached(Self::preamble() + src))
    }
    fn revert(&mut self) {
        self.source = None;
    }

    pub(crate) fn render(&mut self, msg: &str) -> Result<Vec<u8>, RenderErrors> {
        self.add_source(msg);
        let mut tracer = Tracer::default();
        let document =
            typst::compile(self, &mut tracer).map_err(|e| RenderErrors::SourceError(e.to_vec()))?;
        let frame = document.pages.first().ok_or(RenderErrors::NoPageError)?;
        let pixels = determine_pixels_per_point(frame.size())?;

        let pixmap = typst_render::render(
            frame,
            pixels as f32,
            typst::visualize::Color::from_u8(0, 0, 0, 0),
        );

        let mut writer = Cursor::new(Vec::new());

        image::write_buffer_with_format(
            &mut writer,
            bytemuck::cast_slice(pixmap.pixels()),
            pixmap.width(),
            pixmap.height(),
            image::ColorType::Rgba8,
            image::ImageFormat::Png,
        )
        .unwrap();

        let image = writer.into_inner();
        self.revert();

        Ok(image)
    }
}

fn determine_pixels_per_point(size: Axes<Abs>) -> Result<f64, RenderErrors> {
    let desired_resolution = 2000.0;
    let max_size = 10000.0;
    let max_pixels_per_point = 15.0;

    let x = size.x.to_pt();
    let y = size.y.to_pt();

    if x > max_size || y > max_size {
        Err(RenderErrors::PageSizeTooBig)
    } else {
        let area = x * y;
        Ok((desired_resolution / area.sqrt()).min(max_pixels_per_point))
    }
}

impl typst::World for TypstRendered {
    #[doc = " The standard library."]
    #[doc = ""]
    #[doc = " Can be created through `Library::build()`."]
    fn library(&self) -> &Prehashed<Library> {
        &self.library
    }

    #[doc = " Metadata about all known fonts."]
    fn book(&self) -> &Prehashed<FontBook> {
        &self.fontbook
    }

    #[doc = " Access the main source file."]
    fn main(&self) -> Source {
        let Some(e) = &self.source else {
            unreachable!()
        };
        e.clone()
    }

    #[doc = " Try to access the specified source file."]
    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main().id() {
            return Ok(self.main());
        }
        self.file_entry(&id)
            .map_err(|_| FileError::NotSource)
            .and_then(|f| f.clone().source(id))
    }

    #[doc = " Try to access the specified file."]
    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.file_entry(&id)
            .map_err(|_| FileError::NotSource)
            .map(|f| f.clone().bytes)
    }

    #[doc = " Try to access the font with the given index in the font book."]
    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
    }

    #[doc = " Get the current date."]
    #[doc = ""]
    #[doc = " If no offset is specified, the local date should be chosen. Otherwise,"]
    #[doc = " the UTC date should be chosen with the corresponding offset in hours."]
    #[doc = ""]
    #[doc = " If this function returns `None`, Typst\'s `datetime` function will"]
    #[doc = " return an error."]
    fn today(&self, _offset: Option<i64>) -> Option<Datetime> {
        None
    }
}
#[derive(Debug)]
pub(crate) enum RenderErrors {
    SourceError(Vec<SourceDiagnostic>),
    NoPageError,
    PageSizeTooBig,
}

impl std::fmt::Display for RenderErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RenderErrors::SourceError(e) => {
                let s = String::new();
                let a = e.iter().fold(s, |acc, x| {
                    let q = x
                        .hints
                        .clone()
                        .into_iter()
                        .filter(|a| !a.as_str().trim().is_empty())
                        .fold(String::new(), |acc, e| acc + e.as_str() + "\n");
                    let ret = match x.severity {
                        Severity::Error => acc + "Error:",
                        Severity::Warning => acc + "Warning:",
                    } + " "
                        + x.message.as_str()
                        + if !q.is_empty() { "\nHints:" } else { "" }
                        + q.as_str();
                    ret
                });
                write!(f, "{a}")
            }
            RenderErrors::NoPageError => {
                write!(f, "No pages found...")
            }
            RenderErrors::PageSizeTooBig => {
                write!(f, "Page too big...")
            }
        }
    }
}

impl std::error::Error for RenderErrors {}

fn typst_init() -> &'static Mutex<TypstRendered> {
    static TYPST: OnceLock<Mutex<TypstRendered>> = OnceLock::new();
    TYPST.get_or_init(|| Mutex::new(TypstRendered::new()))
}

pub(crate) async fn typst_render(msg: &str) -> Result<Vec<u8>, RenderErrors> {
    typst_init().lock().await.render(msg)
}
