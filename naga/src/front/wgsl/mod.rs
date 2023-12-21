/*!
Frontend for [WGSL][wgsl] (WebGPU Shading Language).

[wgsl]: https://gpuweb.github.io/gpuweb/wgsl.html
 */

pub mod source_provider;

mod error;
mod index;
mod lower;
mod parse;
#[cfg(test)]
mod tests;
mod to_wgsl;

use std::collections::{HashSet};

use crate::front::wgsl::error::Error;
use crate::front::wgsl::parse::Parser;
use crate::span::FileId;
use thiserror::Error;

pub use crate::front::wgsl::error::ParseError;
use crate::front::wgsl::lower::Lowerer;
use crate::{Scalar, Span};

use self::parse::ast::{self};
use self::source_provider::{File, SourceProvider};


pub struct Frontend {
    parser: Parser,
}

impl Frontend {
    pub const fn new() -> Self {
        Self {
            parser: Parser::new(),
        }
    }

    fn inner<'a>(&mut self, source: &'a str) -> Result<crate::Module, Error<'a>> {
        let mut tu = ast::TranslationUnit::default();

        self.parser.parse(&mut tu, source, 0)?; 
        
        let index = index::Index::generate(&tu)?;
        let module = Lowerer::new(&index).lower(&tu)?;

        Ok(module)
    }

    pub fn parse(&mut self, _source: &str) -> Result<crate::Module, ParseError> {
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


pub fn parse_str(_source: &str) -> Result<crate::Module, ParseError> {
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
        let path = file.path().to_owned(); 
            
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

            let file_id = provider.visit(&path)
                .ok_or(Error::BadPath { span })
                .map_err(|x| x.as_parse_error(provider))?; 

            stack.push((file_id, import.span));
            handled.insert(path); 
        }
    }

    Ok(translation_unit)
}
