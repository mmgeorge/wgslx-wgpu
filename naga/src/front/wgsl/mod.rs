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

use std::path::Path;

use crate::front::wgsl::error::Error;
use crate::front::wgsl::parse::Parser;
use thiserror::Error;

pub use crate::front::wgsl::error::ParseError;
use crate::front::wgsl::lower::Lowerer;
use crate::Scalar;

use self::parse::ast;


pub struct TranslatedFile<'a> {
    translation_unit: ast::TranslationUnit<'a>,
    path: Path,
}

pub trait GetSource {
    fn get_source(path: &Path) -> &str; 
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

    pub fn translation_unit<'a>(&mut self, source: &'a str) -> Result<ast::TranslationUnit<'a>, Error<'a>> {
        self.parser.parse(source)
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

pub fn parse_translation_units<'a, TGetSource: GetSource>(source: &'a str) -> Result<Vec<ast::TranslationUnit<'a>>, Error<'a>> {
    let mut out = Vec::new();
    let translation_unit = Frontend::new().translation_unit(source)?;

    out.push(translation_unit); 

    Ok(out)
}
