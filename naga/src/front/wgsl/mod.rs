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
        self.parser.parse(source).map_err(|x| x.as_parse_error(source))
    }

    fn inner<'a>(&mut self, source: &'a str) -> Result<crate::Module, Error<'a>> {
        let translation_unit = self.parser.parse(source)?;

        lower(&translation_unit, &HashMap::new())
    }
}

fn lower<'a>(
    unit: &ast::TranslationUnit<'a>, 
    modules: &HashMap<PathBuf, crate::Module>,
) -> Result<crate::Module, Error<'a>> {
    
    let index = index::Index::generate(unit)?;
    let module = Lowerer::new(&index).lower(unit, modules)?;

    Ok(module)
}

pub fn parse_str(source: &str) -> Result<crate::Module, ParseError> {
    Frontend::new().parse(source)
}

pub fn parse_module(provider: &impl SourceProvider, path: &str, source: &str) -> Result<(), ParseError> {
    let mut modules: HashMap<PathBuf, crate::Module> = HashMap::new(); 
    let units = parse_translation_units(provider, path, source)?;

    for unit in units.iter().rev() {
        let module = lower(unit, &modules)
            .map_err(|x| x.as_parse_error(source))?;

        modules.insert(unit.path.clone().unwrap(), module); 
    }

    Ok(())
}

// Returns translation units in depth-first order
fn parse_translation_units<'a>(
    provider: &'a impl SourceProvider,
    path: &'a str,
    source: &'a str, 
) -> Result<Vec<ast::TranslationUnit<'a>>, ParseError> {
    
    let mut out = vec![];
    let mut handled = HashSet::new(); 
    let mut stack = vec![(PathBuf::from(path), source, Span::new(0, 0))];

    while let Some((path, source, span)) = stack.pop() {
        let mut translation_unit = Frontend::new().translation_unit(source)?;

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

        out.push(translation_unit); 
    }

    Ok(out)
}
