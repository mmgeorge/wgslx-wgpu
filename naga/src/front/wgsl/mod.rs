/*!
Frontend for [WGSL][wgsl] (WebGPU Shading Language).

[wgsl]: https://gpuweb.github.io/gpuweb/wgsl.html
*/

mod error;
mod index;
mod lower;
mod parse;
#[cfg(test)]
mod tests;
mod to_wgsl;

use std::collections::{HashSet};
use std::ops::Range;
use std::path::{Path, PathBuf};

use crate::front::wgsl::error::Error;
use crate::front::wgsl::parse::Parser;
use crate::span::FileId;
use codespan_reporting::files::{Files, self, line_starts};
use thiserror::Error;

pub use crate::front::wgsl::error::ParseError;
use crate::front::wgsl::lower::Lowerer;
use crate::{Scalar, Span};

use self::parse::ast::{self};


pub trait SourceProvider<'a>: Files<'a> {
    fn visit(&self, path: &Path) -> Option<FileId>;
    fn get(&self, id: FileId) -> Option<&File>;

    fn source_at(&self, span: Span) -> Option<&str> {
        let id = span.file_id?; 
        let file = self.get(id)?;

        Some(&file.source.as_str()[span])
    }

    fn source_at_unchecked(&self, span: Span) -> &str {
        self.source_at(span)
            .expect("Unable to get source")
    }
}

#[derive(Debug, Clone)]
pub struct File {
    id: FileId, 
    path: PathBuf,
    source: String,
    name: String, 
    line_starts: Vec<usize>,
}

impl File {
    /// Create a new source file.
    pub fn new(id: FileId, path: PathBuf, source: String) -> File {
        File {
            id,
            name: path.clone().to_string_lossy().to_string(),
            path,
            line_starts: line_starts(source.as_ref()).collect(),
            source,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn id(&self) -> FileId {
        self.id
    }

    /// Return the name of the file.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return the source of the file.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Return the starting byte index of the line with the specified line index.
    /// Convenience method that already generates errors if necessary.
    pub fn line_start(&self, line_index: usize) -> Result<usize, files::Error> {
        use std::cmp::Ordering;

        match line_index.cmp(&self.line_starts.len()) {
            Ordering::Less => Ok(self
                .line_starts
                .get(line_index)
                .cloned()
                .expect("failed despite previous check")),
            Ordering::Equal => Ok(self.source.len()),
            Ordering::Greater => Err(files::Error::LineTooLarge {
                given: line_index,
                max: self.line_starts.len() - 1,
            }),
        }
    }

    pub fn line_index(&self, (): (), byte_index: usize) -> Result<usize, files::Error> {
        Ok(self
            .line_starts
            .binary_search(&byte_index)
            .unwrap_or_else(|next_line| next_line - 1))
    }

    pub fn line_range(&self, (): (), line_index: usize) -> Result<Range<usize>, files::Error> {
        let line_start = self.line_start(line_index)?;
        let next_line_start = self.line_start(line_index + 1)?;

        Ok(line_start..next_line_start)
    }
}

pub struct Frontend {
    parser: Parser,
}

impl Frontend {
    pub const fn new() -> Self {
        Self {
            parser: Parser::new(),
        }
    }

    pub fn parse(&mut self, source: &str) -> Result<crate::Module, ParseError> {
        todo!()
    }

    pub fn parse_into<'a>(
        &mut self,
        unit: &mut ast::TranslationUnit<'a>,
        file: &'a File
    ) -> Result<(), Error<'a>> {
        self.parser.parse(unit, file.source(), file.id())
    }
}

fn lower<'a>(unit: &ast::TranslationUnit<'a>) -> Result<crate::Module, Error<'a>> {
    let index = index::Index::generate(unit)?;

    Lowerer::new(&index).lower(unit)
}


pub fn parse_module<'a>(provider: &'a impl SourceProvider<'a>, id: FileId) -> Result<crate::Module, ParseError> {
    let unit = parse_translation_unit(provider, id)?;
    let module = lower(&unit).map_err(|x| x.as_parse_error(provider))?;

    Ok(module)
}


pub fn parse_str<'a>(source: &'a str) -> Result<crate::Module, ParseError> {
    todo!()
}


// Returns translation units in depth-first order
fn parse_translation_unit<'a>(
    provider: &'a impl SourceProvider<'a>,
    file_id: FileId,
) -> Result<ast::TranslationUnit<'a>, ParseError> {
    let mut handled = HashSet::new(); 
    let mut stack = vec![(file_id, Span::new(0, 0, None))];

    let mut translation_unit = ast::TranslationUnit::default(); 

    while let Some((file_id, span)) = stack.pop() {
        // Some temporary state specific only to the current file is added to the translation
        // unit on each parse. We only want to capture the global state.
        translation_unit.reset();

        let file = provider.get(file_id).expect("File not found in source provider");
        let source = file.source();
        let path = file.path(); 
            
        Frontend::new().parse_into(&mut translation_unit, file)
            .map_err(|x| x.as_parse_error(provider))?; 
            
        let parent_path = path.parent()
            .ok_or(Error::BadPath { span })
            .map_err(|x| x.as_parse_error(provider))?; 

        for import in &mut translation_unit.imports {
            let path = import.resolve(parent_path); 

            if handled.contains(&path) {
                continue; 
            }

            let file_id = provider.visit(path.as_path())
                .ok_or(Error::BadPath { span })
                .map_err(|x| x.as_parse_error(provider))?; 

            stack.push((file_id, import.span));
            handled.insert(path); 
        }
    }

    Ok(translation_unit)
}
