use nom::character::complete::char;
use nom::Parser;
use smol_str::SmolStr;

use crate::prelude::*;

//-------------------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum CobMapKey
{
    Value(CobValue),
    FieldName
    {
        fill: CobFill,
        name: SmolStr,
    },
}

impl CobMapKey
{
    pub fn write_to(&self, writer: &mut impl RawSerializer) -> Result<(), std::io::Error>
    {
        self.write_to_with_space(writer, "")
    }

    pub fn write_to_with_space(&self, writer: &mut impl RawSerializer, space: &str) -> Result<(), std::io::Error>
    {
        match self {
            Self::Value(value) => {
                value.write_to_with_space(writer, space)?;
            }
            Self::FieldName { fill, name } => {
                fill.write_to_or_else(writer, space)?;
                writer.write_bytes(name.as_bytes())?;
            }
        }
        Ok(())
    }

    pub fn try_parse(fill: CobFill, content: Span) -> Result<(Option<Self>, CobFill, Span), SpanError>
    {
        // Try to parse value first in case it's a field-name-like value such as 'true' or 'none'.
        let fill = match CobValue::try_parse(fill, content)? {
            (Some(value), next_fill, remaining) => return Ok((Some(Self::Value(value)), next_fill, remaining)),
            (None, fill, _) => fill,
        };
        match snake_identifier(content) {
            Ok((remaining, id)) => {
                let (next_fill, remaining) = CobFill::parse(remaining);
                Ok((
                    Some(Self::FieldName { fill, name: SmolStr::from(*id.fragment()) }),
                    next_fill,
                    remaining,
                ))
            }
            _ => Ok((None, fill, content)),
        }
    }

    pub fn recover_fill(&mut self, other: &Self)
    {
        match (self, other) {
            (Self::Value(value), Self::Value(other_value)) => {
                value.recover_fill(other_value);
            }
            (Self::FieldName { fill, .. }, Self::FieldName { fill: other_fill, .. }) => {
                fill.recover(other_fill);
            }
            _ => (),
        }
    }

    pub fn value(value: CobValue) -> Self
    {
        Self::Value(value)
    }

    pub fn field_name(name: impl AsRef<str>) -> Self
    {
        Self::FieldName { fill: CobFill::default(), name: SmolStr::from(name.as_ref()) }
    }

    pub fn is_struct_field(&self) -> bool
    {
        matches!(*self, Self::FieldName{ .. })
    }
}

//-------------------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct CobMapKeyValue
{
    pub key: CobMapKey,
    pub semicolon_fill: CobFill,
    pub value: CobValue,
}

impl CobMapKeyValue
{
    pub fn write_to(&self, writer: &mut impl RawSerializer) -> Result<(), std::io::Error>
    {
        self.write_to_with_space(writer, "")
    }

    pub fn write_to_with_space(&self, writer: &mut impl RawSerializer, space: &str) -> Result<(), std::io::Error>
    {
        self.key.write_to_with_space(writer, space)?;
        self.semicolon_fill.write_to(writer)?;
        writer.write_bytes(":".as_bytes())?;
        self.value.write_to(writer)?;
        Ok(())
    }

    pub fn try_parse(fill: CobFill, content: Span) -> Result<(Option<Self>, CobFill, Span), SpanError>
    {
        let (maybe_key, semicolon_fill, remaining) = CobMapKey::try_parse(fill, content)?;
        let Some(key) = maybe_key else { return Ok((None, semicolon_fill, content)) };
        let (remaining, _) = char(':').parse(remaining)?;
        let (value_fill, remaining) = CobFill::parse(remaining);
        let (Some(value), next_fill, remaining) = CobValue::try_parse(value_fill, remaining)? else {
            tracing::warn!("failed parsing value for map entry at {}; no valid value found", get_location(remaining));
            return Err(span_verify_error(content));
        };
        Ok((Some(Self { key, semicolon_fill, value }), next_fill, remaining))
    }

    pub fn recover_fill(&mut self, other: &Self)
    {
        self.key.recover_fill(&other.key);
        self.semicolon_fill.recover(&other.semicolon_fill);
        self.value.recover_fill(&other.value);
    }

    pub fn struct_field(key: &str, value: CobValue) -> Self
    {
        Self {
            key: CobMapKey::field_name(key),
            semicolon_fill: CobFill::default(),
            value,
        }
    }

    pub fn map_entry(key: CobValue, value: CobValue) -> Self
    {
        Self {
            key: CobMapKey::value(key),
            semicolon_fill: CobFill::default(),
            value,
        }
    }

    pub fn is_struct_field(&self) -> bool
    {
        self.key.is_struct_field()
    }
}

//-------------------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum CobMapEntry
{
    KeyValue(CobMapKeyValue),
    /// Only catch-all params are allowed.
    MacroParam(CobMacroParam),
}

impl CobMapEntry
{
    pub fn write_to(&self, writer: &mut impl RawSerializer) -> Result<(), std::io::Error>
    {
        self.write_to_with_space(writer, "")
    }

    pub fn write_to_with_space(&self, writer: &mut impl RawSerializer, space: &str) -> Result<(), std::io::Error>
    {
        match self {
            Self::KeyValue(keyvalue) => {
                keyvalue.write_to_with_space(writer, space)?;
            }
            Self::MacroParam(param) => {
                param.write_to_with_space(writer, space)?;
            }
        }
        Ok(())
    }

    pub fn try_parse(fill: CobFill, content: Span) -> Result<(Option<Self>, CobFill, Span), SpanError>
    {
        let fill = match rc(content, move |c| CobMapKeyValue::try_parse(fill, c))? {
            (Some(kv), next_fill, remaining) => return Ok((Some(Self::KeyValue(kv)), next_fill, remaining)),
            (None, next_fill, _) => next_fill,
        };
        let fill = match rc(content, move |c| CobMacroParam::try_parse(fill, c))? {
            (Some(param), next_fill, remaining) => {
                if !param.is_catch_all() {
                    tracing::warn!("failed parsing map entry at {}; found macro param that isn't a 'catch all'",
                        get_location(content));
                    return Err(span_verify_error(content));
                }
                return Ok((Some(Self::MacroParam(param)), next_fill, remaining));
            }
            (None, next_fill, _) => next_fill,
        };

        Ok((None, fill, content))
    }

    pub fn recover_fill(&mut self, other: &Self)
    {
        match (self, other) {
            (Self::KeyValue(keyvalue), Self::KeyValue(other_keyvalue)) => {
                keyvalue.recover_fill(other_keyvalue);
            }
            (Self::MacroParam(param), Self::MacroParam(other_param)) => {
                param.recover_fill(other_param);
            }
            _ => (),
        }
    }

    pub fn struct_field(key: &str, value: CobValue) -> Self
    {
        Self::KeyValue(CobMapKeyValue::struct_field(key, value))
    }

    pub fn map_entry(key: CobValue, value: CobValue) -> Self
    {
        Self::KeyValue(CobMapKeyValue::map_entry(key, value))
    }

    pub fn is_struct_field(&self) -> bool
    {
        match self {
            Self::KeyValue(kv) => kv.is_struct_field(),
            Self::MacroParam(param) => param.is_catch_all(),
        }
    }

    /// Returns `true` if the value is a key-value type.
    pub fn is_keyvalue(&self) -> bool
    {
        matches!(*self, Self::KeyValue(..))
    }
}

//-------------------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct CobMap
{
    /// Fill before opening `{`.
    pub start_fill: CobFill,
    pub entries: Vec<CobMapEntry>,
    /// Fill before ending `}`.
    pub end_fill: CobFill,
}

impl CobMap
{
    pub fn write_to(&self, writer: &mut impl RawSerializer) -> Result<(), std::io::Error>
    {
        self.write_to_with_space(writer, "")
    }

    pub fn write_to_with_space(&self, writer: &mut impl RawSerializer, space: &str) -> Result<(), std::io::Error>
    {
        self.start_fill.write_to_or_else(writer, space)?;
        writer.write_bytes("{".as_bytes())?;
        for (idx, entry) in self.entries.iter().enumerate() {
            if idx == 0 {
                entry.write_to(writer)?;
            } else {
                entry.write_to_with_space(writer, " ")?;
            }
        }
        self.end_fill.write_to(writer)?;
        writer.write_bytes("}".as_bytes())?;
        Ok(())
    }

    pub fn try_parse(start_fill: CobFill, content: Span) -> Result<(Option<Self>, CobFill, Span), SpanError>
    {
        let Ok((remaining, _)) = char::<_, ()>('{').parse(content) else { return Ok((None, start_fill, content)) };

        let (mut item_fill, mut remaining) = CobFill::parse(remaining);
        let mut entries = vec![];

        let end_fill = loop {
            let fill_len = item_fill.len();
            match rc(remaining, move |rm| CobMapEntry::try_parse(item_fill, rm))? {
                (Some(entry), next_fill, after_entry) => {
                    if entries.len() > 0 {
                        if fill_len == 0 {
                            tracing::warn!("failed parsing map at {}; entry #{} is not preceded by fill/whitespace",
                                get_location(content), entries.len() + 1);
                            return Err(span_verify_error(content));
                        }
                    }
                    entries.push(entry);
                    item_fill = next_fill;
                    remaining = after_entry;
                }
                (None, end_fill, after_end) => {
                    remaining = after_end;
                    break end_fill;
                }
            }
        };

        let (remaining, _) = char('}').parse(remaining)?;
        let (post_fill, remaining) = CobFill::parse(remaining);
        Ok((Some(Self { start_fill, entries, end_fill }), post_fill, remaining))
    }

    pub fn recover_fill(&mut self, other: &Self)
    {
        self.start_fill.recover(&other.start_fill);
        for (entry, other_entry) in self.entries.iter_mut().zip(other.entries.iter()) {
            entry.recover_fill(other_entry);
        }
        self.end_fill.recover(&other.end_fill);
    }

    /// Returns `true` if all entries are either field-name:value pairs or macro 'catch all' params.
    pub fn is_structlike(&self) -> bool
    {
        !self.entries.iter().any(|e| !e.is_struct_field())
    }
}

impl From<Vec<CobMapEntry>> for CobMap
{
    fn from(entries: Vec<CobMapEntry>) -> Self
    {
        Self {
            start_fill: CobFill::default(),
            entries,
            end_fill: CobFill::default(),
        }
    }
}

/*
Parsing:
*/

//-------------------------------------------------------------------------------------------------------------------