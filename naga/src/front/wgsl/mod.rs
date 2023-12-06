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

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::front::wgsl::error::Error;
use crate::front::wgsl::parse::Parser;
use thiserror::Error;

pub use crate::front::wgsl::error::ParseError;
use crate::front::wgsl::lower::Lowerer;
use crate::{Scalar, Span};

use self::parse::ast::{self};


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
        let mut tu = ast::TranslationUnit::default(); 

        self.parser.parse(&mut tu, source).map_err(|x| x.as_parse_error(source))?; 

        Ok(tu)
    }

    pub fn parse_into<'a>(&mut self, tu: &mut ast::TranslationUnit<'a>,  source: &'a str) -> Result<(), ParseError> {
        self.parser.parse(tu, source).map_err(|x| x.as_parse_error(source))
    }

    fn inner<'a>(&mut self, source: &'a str) -> Result<crate::Module, Error<'a>> {
        let mut translation_unit = ast::TranslationUnit::default(); 

        self.parser.parse(&mut translation_unit, source)?;
        
        lower(&translation_unit)
    }
}

fn lower<'a>(unit: &ast::TranslationUnit<'a>) -> Result<crate::Module, Error<'a>> {
    
    let index = index::Index::generate(unit)?;

    Lowerer::new(&index).lower(unit)
}

pub fn parse_str(source: &str) -> Result<crate::Module, ParseError> {
    Frontend::new().parse(source)
}

pub fn parse_module(provider: &impl SourceProvider, path: &str, source: &str) -> Result<crate::Module, ParseError> {
    let unit = parse_translation_unit(provider, path, source)?;
    let module = lower(&unit).map_err(|x| x.as_parse_error(source))?;

    Ok(module)
}

// Returns translation units in depth-first order
fn parse_translation_unit<'a>(
    provider: &'a impl SourceProvider,
    path: &'a str,
    source: &'a str, 
) -> Result<ast::TranslationUnit<'a>, ParseError> {
    
    let mut handled = HashSet::new(); 
    let mut stack = vec![(PathBuf::from(path), source, Span::new(0, 0))];

    let mut translation_unit = ast::TranslationUnit::default(); 

    while let Some((path, source, span)) = stack.pop() {
        Frontend::new().parse_into(&mut translation_unit, source)?;

        translation_unit.path = Some(path.clone()); 

        let parent_path = path.parent()
            .ok_or(Error::BadPath { span })
            .map_err(|x| x.as_parse_error(source))?; 

        for import in &mut translation_unit.imports {
            let path = import.resolve(parent_path); 

            if handled.contains(&path) {
                continue; 
            }

            let source = provider.get_source(path.as_path())
                .ok_or(Error::BadPath { span })
                .map_err(|x| x.as_parse_error(source))?; 

            stack.push((path.clone(), source, import.span));
            handled.insert(path); 
        }
    }

    Ok(translation_unit)
}
