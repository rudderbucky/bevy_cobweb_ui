// CafLoadableMacroCall
// CafLoadableMacroDef
// - Flatten group or single loadable only

// def must start at beginning of line

use crate::prelude::*;

//-------------------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct CafLoadableMacroCall;

impl CafLoadableMacroCall
{
    pub fn write_to(&self, writer: &mut impl RawSerializer) -> Result<(), std::io::Error>
    {
        self.write_to_with_space(writer, "")
    }

    pub fn write_to_with_space(&self, _writer: &mut impl RawSerializer, _space: &str)
        -> Result<(), std::io::Error>
    {
        Ok(())
    }

    pub fn try_parse(fill: CafFill, content: Span) -> Result<(Option<Self>, CafFill, Span), SpanError>
    {
        // TODO
        Ok((None, fill, content))
    }

    pub fn recover_fill(&mut self, _other: &Self) {}
}

//-------------------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct CafLoadableMacroDef;

impl CafLoadableMacroDef
{
    pub fn write_to(&self, writer: &mut impl RawSerializer) -> Result<(), std::io::Error>
    {
        self.write_to_with_space(writer, "")
    }

    pub fn write_to_with_space(&self, _writer: &mut impl RawSerializer, _space: &str)
        -> Result<(), std::io::Error>
    {
        Ok(())
    }

    pub fn recover_fill(&mut self, _other: &Self) {}
}

//-------------------------------------------------------------------------------------------------------------------