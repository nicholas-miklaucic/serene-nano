use std::sync::Arc;
use std::io::Cursor;
use std::num::NonZeroUsize;use std::convert::TryInto;
use std::ops::Range;
use comemo::Prehashed;
use typst::diag::{FileError, FileResult, SourceError};
use typst::eval::Library;
use typst::font::{Font, FontBook};
use typst::syntax::{Source, SourceId, ErrorPos};
use typst::util::Buffer;
use typst::geom::{Axis, RgbaColor, Size};
use typst_library::text::lorem;
use time;
const FILE_NAME: &str = "<user input>";
use serde::{Deserialize, Serialize};


pub(crate) fn my_lorem(num:usize)->String{
    //Testing if I got the typst library in properly
    lorem(num).to_string()
}



#[derive(Debug, Clone, Serialize, Deserialize)]
enum Request {
	Render { code: String },
	Ast { code: String },
	Version,
}

#[derive(Debug, Serialize, Deserialize)]
struct Rendered {
	image: Vec<u8>,
	more_pages: Option<NonZeroUsize>,
}

type RenderResponse = Result<Rendered, String>;

type AstResponse = String;

#[derive(Debug, Serialize, Deserialize)]
struct VersionResponse {
	version: String,
	git_hash: String,
}

#[derive(Debug, Serialize, Deserialize)]
enum Response {
	Render(RenderResponse),
	Ast(AstResponse),
	Version(VersionResponse),
}

const DESIRED_RESOLUTION: f32 = 1000.0;
const MAX_SIZE: f32 = 10000.0;
const MAX_PIXELS_PER_POINT: f32 = 5.0;
#[derive(Debug, thiserror::Error)]
#[error(
	"rendered output was too big: the {axis:?} axis was {size} pt but the maximum is {MAX_SIZE}"
)]
struct TooBig {
	size: f32,
	axis: Axis,
}
fn determine_pixels_per_point(size: Size) -> Result<f32, TooBig> {
	// We want to truncate.
	#![allow(clippy::cast_possible_truncation)]

	let x = size.x.to_pt() as f32;
	let y = size.y.to_pt() as f32;

	if x > MAX_SIZE {
		Err(TooBig {
			size: x,
			axis: Axis::X,
		})
	} else if y > MAX_SIZE {
		Err(TooBig {
			size: y,
			axis: Axis::Y,
		})
	} else {
		let area = x * y;
		let nominal = DESIRED_RESOLUTION / area.sqrt();
		Ok(nominal.min(MAX_PIXELS_PER_POINT))
	}
}

#[derive(Debug)]
struct SourceErrorsWithSource {
	source: Source,
	errors: Vec<SourceError>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CharIndex {
	first_byte: usize,
	char_index: usize,
}

impl std::ops::Add for CharIndex {
	type Output = CharIndex;

	fn add(self, other: Self) -> Self {
		Self {
			first_byte: self.first_byte + other.first_byte,
			char_index: self.char_index + other.char_index,
		}
	}
}

fn byte_index_to_char_index(source: &str, byte_index: usize) -> CharIndex {
	let mut ret = CharIndex {
		first_byte: 0,
		char_index: 0,
	};

	for ch in source.chars() {
		if byte_index < ret.first_byte + ch.len_utf8() {
			break;
		}
		ret.char_index += 1;
		ret.first_byte += ch.len_utf8();
	}

	ret
}


fn byte_span_to_char_span(source: &str, span: Range<usize>) -> Option<Range<usize>> {
	if span.start > span.end {
		return None;
	}

	let start = byte_index_to_char_index(source, span.start);
	let end = byte_index_to_char_index(&source[start.first_byte..], span.end - span.start) + start;
	Some(start.char_index..end.char_index)
}



impl std::fmt::Display for SourceErrorsWithSource {
	fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		use ariadne::{Config, Label, Report};

		struct SourceCache(ariadne::Source);

		impl ariadne::Cache<()> for SourceCache {
			fn fetch(&mut self, _id: &()) -> Result<&ariadne::Source, Box<dyn std::fmt::Debug + '_>> {
				Ok(&self.0)
			}

			fn display<'a>(&self, _id: &'a ()) -> Option<Box<dyn std::fmt::Display + 'a>> {
				Some(Box::new(FILE_NAME))
			}
		}

		let source_text = self.source.text();
		let mut cache = SourceCache(ariadne::Source::from(source_text));

		let mut bytes = Vec::new();

		for error in self
			.errors
			.iter()
			.filter(|error| error.span.source() == self.source.id())
		{
			bytes.clear();

			let span = self.source.range(error.span);
			let span = match error.pos {
				ErrorPos::Full => span,
				ErrorPos::Start => span.start..span.start,
				ErrorPos::End => span.end..span.end,
			};
			let span = byte_span_to_char_span(source_text, span).ok_or(std::fmt::Error)?;

			let report = Report::build(ariadne::ReportKind::Error, (), span.start)
				.with_config(Config::default().with_tab_width(2).with_color(false))
				.with_message(&error.message)
				.with_label(Label::new(span))
				.finish();
			// The unwrap will never fail since `Vec`'s `Write` implementation is infallible.
			report.write(&mut cache, &mut bytes).unwrap();

			// The unwrap will never fail since the output string is always valid UTF-8.
			formatter.write_str(std::str::from_utf8(&bytes).unwrap())?;
		}

		Ok(())
	}
}

impl std::error::Error for SourceErrorsWithSource {}

#[derive(Debug, thiserror::Error)]
enum Error {
	#[error(transparent)]
	Source(#[from] SourceErrorsWithSource),
	#[error(transparent)]
	TooBig(#[from] TooBig),
	#[error("no pages in rendered output")]
	NoPages,
}

fn render(sandbox: Arc<Sandbox>, source: String) -> Result<Rendered, Error> {
	let world = sandbox.with_source(source);

	let document = typst::compile(&world).map_err(|errors| SourceErrorsWithSource {
		source: world.into_source(),
		errors: *errors,
	})?;
	let frame = &document.pages.get(0).ok_or(Error::NoPages)?;
	let more_pages = NonZeroUsize::new(document.pages.len().saturating_sub(1));

	let pixels_per_point = determine_pixels_per_point(frame.size())?;

	let pixmap = typst::export::render(frame, pixels_per_point, RgbaColor::new(0, 0, 0, 0).into());

	let mut writer = Cursor::new(Vec::new());

	// The unwrap will never fail since `Vec`'s `Write` implementation is infallible.
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
	Ok(Rendered { image, more_pages })
}




// fn my_render(source: String)->Result<Rendered, Error>{

// }




struct Sandbox {
	library: Prehashed<Library>,
	book: Prehashed<FontBook>,
	fonts: Vec<Font>,
}

fn fonts() -> Vec<Font> {
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

fn make_source(source: String) -> Source {
	Source::new(SourceId::from_u16(0), FILE_NAME.as_ref(), source)
}

fn get_time() -> time::OffsetDateTime {
	time::OffsetDateTime::now_utc()
}

struct WithSource {
	sandbox: Arc<Sandbox>,
	source: Source,
	time: time::OffsetDateTime,
}

impl Sandbox {
	fn new() -> Self {
		let fonts = fonts();

		Self {
			library: Prehashed::new(typst_library::build()),
			book: Prehashed::new(FontBook::from_fonts(&fonts)),
			fonts,
		}
	}

	fn with_source(self: Arc<Self>, source: String) -> WithSource {
		WithSource {
			sandbox: self,
			source: make_source(source),
			time: get_time(),
		}
	}
}

impl WithSource {
	fn into_source(self) -> Source {
		self.source
	}
}

impl typst::World for WithSource {
	fn library(&self) -> &Prehashed<Library> {
		&self.sandbox.library
	}

	fn main(&self) -> &Source {
		&self.source
	}

	fn resolve(&self, path: &std::path::Path) -> FileResult<SourceId> {
		Err(FileError::NotFound(path.into()))
	}

	fn source(&self, id: SourceId) -> &Source {
		assert_eq!(id, self.source.id());
		&self.source
	}

	fn book(&self) -> &Prehashed<FontBook> {
		&self.sandbox.book
	}

	fn font(&self, id: usize) -> Option<Font> {
		self.sandbox.fonts.get(id).cloned()
	}

	fn file(&self, path: &std::path::Path) -> FileResult<Buffer> {

		Err(FileError::NotFound(path.into()))
	}

	fn today(&self, offset: Option<i64>) -> Option<typst::eval::Datetime> {
		// We are in UTC.
		let offset = offset.unwrap_or(0);
		let offset = time::UtcOffset::from_hms(offset.try_into().ok()?, 0, 0).ok()?;
		let time = self.time.checked_to_offset(offset)?;


		Some(typst::eval::Datetime::Date(time.date()))
	}

}
