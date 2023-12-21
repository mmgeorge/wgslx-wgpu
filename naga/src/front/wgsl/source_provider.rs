use std::ops::Range;
use std::path::{Path, PathBuf};

use crate::Span;

pub use codespan_reporting::files::{Files, self, line_starts};
pub use codespan_reporting::files::*;

pub type FileId = u32;

pub trait SourceProvider<'a>: Files<'a> {
    fn visit(&self, path: impl AsRef<Path>) -> Option<FileId>;
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
    pub source: String,
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

    pub const fn id(&self) -> FileId {
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
    pub fn line_start(&self, line_index: usize) -> Result<usize, Error> {
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

    pub fn line_index(&self, (): (), byte_index: usize) -> Result<usize, Error> {
        Ok(self
            .line_starts
            .binary_search(&byte_index)
            .unwrap_or_else(|next_line| next_line - 1))
    }

    pub fn line_range(&self, (): (), line_index: usize) -> Result<Range<usize>, Error> {
        let line_start = self.line_start(line_index)?;
        let next_line_start = self.line_start(line_index + 1)?;

        Ok(line_start..next_line_start)
    }
}
