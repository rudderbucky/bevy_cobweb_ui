// CobMacroParam
// - @, ?, ..
// - Param id potentially nested.
// CobMacroParamDef
// - Unassigned
// - Assigned
// - Nested
// - Catch-all into flatten group
// - type params for generics: use ^param notation without whitespace, cannot be assigned (non-optional)

use nom::IResult;

use crate::prelude::*;

//-------------------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct CobMacroParam;

impl CobMacroParam
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

    pub fn parse_nomlike(content: Span) -> IResult<Span, Self>
    {
        // TODO
        Err(span_verify_error(content))
    }

    pub fn try_parse(fill: CobFill, content: Span) -> Result<(Option<Self>, CobFill, Span), SpanError>
    {
        // TODO
        Ok((None, fill, content))
    }

    pub fn is_required(&self) -> bool
    {
        // TODO
        false
    }

    pub fn is_catch_all(&self) -> bool
    {
        // TODO
        false
    }

    pub fn recover_fill(&mut self, _other: &Self) {}
}

//-------------------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct CobMacroParamDef;

impl CobMacroParamDef
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
