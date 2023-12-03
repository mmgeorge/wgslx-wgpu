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

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::front::wgsl::error::Error;
use crate::front::wgsl::parse::Parser;
use thiserror::Error;

pub use crate::front::wgsl::error::ParseError;
use crate::front::wgsl::lower::Lowerer;
use crate::{Scalar, Span};

use self::parse::ast;


pub struct TranslatedFile<'a> {
    translation_unit: ast::TranslationUnit<'a>,
    path: Path,
}

pub trait SourceProvider {
    fn get_source(&self, path: &Path) -> Option<&str>; 
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
        self.inner(source).map_err(|x| x.as_parse_error(source))
    }

    pub fn translation_unit<'a>(&mut self, source: &'a str) -> Result<ast::TranslationUnit<'a>, ParseError> {
        self.parser.parse(source).map_err(|x| x.as_parse_error(source))
    }

    fn inner<'a>(&mut self, source: &'a str) -> Result<crate::Module, Error<'a>> {
        let translation_unit = self.parser.parse(source)?;

        let index = index::Index::generate(&translation_unit)?;
        let module = Lowerer::new(&index).lower(&translation_unit)?;

        Ok(module)
    }
}

pub fn parse_str(source: &str) -> Result<crate::Module, ParseError> {
    Frontend::new().parse(source)
}

pub fn parse_translation_units<'a, TSourceProvider: SourceProvider>(provider: &'a TSourceProvider, path: &'a str) -> Result<HashMap<PathBuf, ast::TranslationUnit<'a>>, ParseError> {
    let mut out = HashMap::new();
    let mut stack = vec![(PathBuf::from(path), Span::new(0, 0))];

    while let Some((path, path_span)) = stack.pop() {
        let source = provider.get_source(path.as_path())
            .ok_or(Error::BadPath { span: path_span })
            // TODO: Fixme
            .map_err(|x| x.as_parse_error(""))?; 

        let translation_unit = Frontend::new().translation_unit(source)?;

        for import_token in &translation_unit.imports {
            let import = import_token.path.to_string().chars().filter(|c| *c != '"').collect::<String>(); 
            let import_path = path
                .parent()
                .ok_or(Error::BadPath { span: import_token.span })
                .map_err(|x| x.as_parse_error(source))?
                .join(import); 

            if out.contains_key(import_path.as_path()) {
                continue; 
            }

            stack.push((import_path, import_token.span));
        }

        eprintln!("Adding {:?}", path); 
        out.insert(path, translation_unit); 
    }

    Ok(out)
}
