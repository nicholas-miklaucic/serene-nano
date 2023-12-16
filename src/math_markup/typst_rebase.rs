use comemo::Prehashed;

use once_cell::unsync::Lazy;
// use once_cell::sync::Lazy
use poise::structs;
use regex::{self, Regex};
use std::cell::{RefCell, RefMut};
use std::collections::HashMap;
use std::convert::TryInto;
use std::fs;
use std::ops::DerefMut;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use toml::Table;
use typst::diag::{FileError, FileResult, Severity, SourceDiagnostic, SourceResult};
use typst::diag::{PackageError, PackageResult};
use typst::foundations::{Bytes, Datetime};
use typst::syntax::{FileId, PackageSpec, PackageVersion, Source};
use typst::text::{Font, FontBook};
use typst::Library;

//Needed for reading files for packages
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

//The rewrite of TypstEssentials
struct WithoutSource {
    library: Prehashed<Library>,
    fontbook: Prehashed<FontBook>,
    fonts: Vec<Font>,
    files: RefCell<Vec<(FileId, FileEntry)>>, //This used to be a hashmap, but bruh, there are like 3 to 5 files here maximum, its easier to traverse a list
}

struct WithSource {
    base: Arc<WithoutSource>,
    source: Source,
}

impl WithSource {
    fn file_entry(&mut self, id: FileId) -> FileResult<&FileEntry> {
        //Check if file is already in vector
        if let Some(a) = self.base.from_file_id(&id) {
            return Ok(a);
        }

        //Check if it is a package, should be infallible, but yeah
        let p = id
            .package()
            .ok_or(FileError::NotFound(id.vpath().as_rootless_path().into()))?;

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

        self.base.new_entry((id, fe));
        self.base.files.push((id.clone(), fe));
        //Should return the above file_entry, and there should be an easy way to do it, but i dont know;

        Ok(self.base.from_file_id(&id).unwrap())
    }
}

impl typst::World for WithSource {
    #[doc = " The standard library."]
    #[doc = ""]
    #[doc = " Can be created through `Library::build()`."]
    fn library(&self) -> &Prehashed<Library> {
        &self.base.library
    }

    #[doc = " Metadata about all known fonts."]
    fn book(&self) -> &Prehashed<FontBook> {
        &self.base.fontbook
    }

    #[doc = " Access the main source file."]
    fn main(&self) -> Source {
        self.source.clone()
    }

    #[doc = " Try to access the specified source file."]
    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.source.id() {
            return Ok(self.source.clone());
        } else {
            if let Ok(a) = self.file_entry(id) {}
        }

        todo!()
    }

    #[doc = " Try to access the specified file."]
    fn file(&self, id: FileId) -> FileResult<Bytes> {
        todo!()
    }

    #[doc = " Try to access the font with the given index in the font book."]
    fn font(&self, index: usize) -> Option<Font> {
        self.base.fonts.get(index).cloned()
    }

    #[doc = " Get the current date."]
    #[doc = ""]
    #[doc = " If no offset is specified, the local date should be chosen. Otherwise,"]
    #[doc = " the UTC date should be chosen with the corresponding offset in hours."]
    #[doc = ""]
    #[doc = " If this function returns `None`, Typst\'s `datetime` function will"]
    #[doc = " return an error."]
    fn today(&self, offset: Option<i64>) -> Option<Datetime> {
        None
    }
}

impl WithoutSource {
    fn from_file_id(&self, file_id: &FileId) -> Option<&FileEntry> {
        self.files
            .iter()
            .find_map(|(a, b)| if a == file_id { Some(b) } else { None })
    }
    fn new() -> Self {
        let fonts = get_fonts();
        Self {
            library: Prehashed::new(Library::build()),
            fontbook: Prehashed::new(FontBook::from_fonts(&fonts)),
            fonts,
            files: Vec::new(),
        }
    }

    fn new_entry(&mut self, t: (FileId, FileEntry)) {
        self.files.push(t);
    }
    fn preamble() -> String {
        let imports = r#"
            #import "@preview/whalogen:0.1.0": *
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
            };
        "#;
        imports.to_string() + theme + page_size + text_options + math_remaining
    }
}

static TYPST_BASE: Lazy<Rc<WithoutSource>> = Lazy::new(|| Rc::new(WithoutSource::new()));

fn with_source(source: &str) -> WithSource {
    WithSource {
        base: TYPST_BASE.clone(),
        source: Source::detached(WithoutSource::preamble() + source),
    }
}

//Wanted to check if the logic was sound
#[cfg(test)]
mod sample_tests {
    use super::*;
    #[test]
    fn test_regex() {
        let name_package = Regex::new(r"(\w+)-((\d+)\.(\d+)\.(\d+))").unwrap();
        let s = "whalogen-0.11.1";
        let Some(a) = name_package.captures(s) else {
            assert!(false);
            return;
        };
        assert_eq!("whalogen", &a[1]);
        assert_eq!("0.11.1", &a[2]);
        assert_eq!("0", &a[3]);
        assert_eq!("11", &a[4]);
        assert_eq!("1", &a[5]);

        let captured_strings = name_package.captures(s).unwrap();
        let pv = PackageVersion {
            major: captured_strings[3].parse::<u32>().unwrap(),
            minor: captured_strings[4].parse::<u32>().unwrap(),
            patch: captured_strings[5].parse::<u32>().unwrap(),
        };

        let ps = PackageSpec {
            namespace: "preview".into(),
            name: captured_strings[1].into(),
            version: pv,
        };
        assert_eq!(pv.to_string(), "0.11.1");

        assert_eq!(ps.version, pv);
    }

    #[test]
    fn test_folders() {
        let mut packages_path = std::path::PathBuf::new();
        packages_path.push("packages");
        let m = std::fs::read_dir(packages_path).unwrap();
        let mut v = m.into_iter();
        let Some(Ok(p)) = v.next() else {
            assert!(false);
            return;
        };
        assert_eq!(p.file_name(), "whalogen-0.1.0");
    }
}
