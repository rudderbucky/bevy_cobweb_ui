use crate::prelude::*;

//-------------------------------------------------------------------------------------------------------------------

#[derive(Default, Debug, Clone, PartialEq)]
pub struct CafConstants
{
    pub start_fill: CafFill,
}

impl CafConstants
{
    pub fn write_to(&self, writer: &mut impl std::io::Write) -> Result<(), std::io::Error>
    {
        self.start_fill.write_to(writer)
    }

    pub fn eq_ignore_whitespace(&self, _other: &CafConstants) -> bool
    {
        true
    }
}

//-------------------------------------------------------------------------------------------------------------------
