use smol_str::SmolStr;

use crate::prelude::*;

//-------------------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum CafMapKey
{
    FieldName
    {
        fill: CafFill,
        name: SmolStr,
    },
    Value(CafValue),
}

impl CafMapKey
{
    pub fn write_to(&self, writer: &mut impl std::io::Write) -> Result<(), std::io::Error>
    {
        self.write_to_with_space(writer, "")
    }

    pub fn write_to_with_space(&self, writer: &mut impl std::io::Write, space: &str)
        -> Result<(), std::io::Error>
    {
        match self {
            Self::FieldName { fill, name } => {
                fill.write_to_or_else(writer, space)?;
                writer.write(name.as_bytes())?;
            }
            Self::Value(value) => {
                value.write_to_with_space(writer, space)?;
            }
        }
        Ok(())
    }

    /// Currently only strings and ints/floats are supported as map keys, since we are tied to JSON limitations.
    //todo
    pub fn to_json_map_key(&self) -> Result<String, std::io::Error>
    {
        match self {
            Self::FieldName { name, .. } => Ok(String::from(name.as_str())),
            Self::Value(value) => match value {
                CafValue::String(string) => {
                    let serde_json::Value::String(string) = string.to_json()? else { unreachable!() };
                    Ok(string)
                }
                CafValue::Number(number) => {
                    let string = String::from(number.number.original.as_str());
                    Ok(string)
                }
                _ => todo!(),
            },
        }
    }

    pub fn recover_fill(&mut self, other: &Self)
    {
        match (self, other) {
            (Self::FieldName { fill, .. }, Self::FieldName { fill: other_fill, .. }) => {
                fill.recover(other_fill);
            }
            (Self::Value(value), Self::Value(other_value)) => {
                value.recover_fill(other_value);
            }
            _ => (),
        }
    }

    pub fn field_name(name: impl AsRef<str>) -> Self
    {
        Self::FieldName { fill: CafFill::default(), name: SmolStr::from(name.as_ref()) }
    }

    pub fn value(value: CafValue) -> Self
    {
        Self::Value(value)
    }
}

//-------------------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct CafMapKeyValue
{
    pub key: CafMapKey,
    pub semicolon_fill: CafFill, //todo: does allowing fill between key and semicolon create parsing ambiguities?
    pub value: CafValue,
}

impl CafMapKeyValue
{
    pub fn write_to(&self, writer: &mut impl std::io::Write) -> Result<(), std::io::Error>
    {
        self.write_to_with_space(writer, "")
    }

    pub fn write_to_with_space(&self, writer: &mut impl std::io::Write, space: &str)
        -> Result<(), std::io::Error>
    {
        self.key.write_to_with_space(writer, space)?;
        self.semicolon_fill.write_to(writer)?;
        writer.write(":".as_bytes())?;
        self.value.write_to(writer)?;
        Ok(())
    }

    pub fn add_to_json(&self, map: &mut serde_json::Map<String, serde_json::Value>) -> Result<(), std::io::Error>
    {
        map.insert(self.key.to_json_map_key()?, self.value.to_json()?);
        Ok(())
    }

    pub fn recover_fill(&mut self, other: &Self)
    {
        self.key.recover_fill(&other.key);
        self.semicolon_fill.recover(&other.semicolon_fill);
        self.value.recover_fill(&other.value);
    }

    pub fn struct_field(key: &str, value: CafValue) -> Self
    {
        Self {
            key: CafMapKey::field_name(key),
            semicolon_fill: CafFill::default(),
            value,
        }
    }

    pub fn map_entry(key: CafValue, value: CafValue) -> Self
    {
        Self {
            key: CafMapKey::value(key),
            semicolon_fill: CafFill::default(),
            value,
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum CafMapEntry
{
    KeyValue(CafMapKeyValue),
    /// Only catch-all params are allowed.
    MacroParam(CafMacroParam),
}

impl CafMapEntry
{
    pub fn write_to(&self, writer: &mut impl std::io::Write) -> Result<(), std::io::Error>
    {
        self.write_to_with_space(writer, "")
    }

    pub fn write_to_with_space(&self, writer: &mut impl std::io::Write, space: &str)
        -> Result<(), std::io::Error>
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

    pub fn add_to_json(&self, map: &mut serde_json::Map<String, serde_json::Value>) -> Result<(), std::io::Error>
    {
        match self {
            Self::KeyValue(keyvalue) => {
                keyvalue.add_to_json(map)?;
            }
            Self::MacroParam(param) => {
                return Err(std::io::Error::other(format!(
                    "macro param {:?} in caf map entry when converting to JSON", param
                )));
            }
        }
        Ok(())
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

    pub fn struct_field(key: &str, value: CafValue) -> Self
    {
        Self::KeyValue(CafMapKeyValue::struct_field(key, value))
    }

    pub fn map_entry(key: CafValue, value: CafValue) -> Self
    {
        Self::KeyValue(CafMapKeyValue::map_entry(key, value))
    }
}

// Parsing:
// - only catch-all data macro params are allowed

//-------------------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct CafMap
{
    /// Fill before opening `{`.
    pub start_fill: CafFill,
    pub entries: Vec<CafMapEntry>,
    /// Fill before ending `}`.
    pub end_fill: CafFill,
}

impl CafMap
{
    pub fn write_to(&self, writer: &mut impl std::io::Write) -> Result<(), std::io::Error>
    {
        self.write_to_with_space(writer, "")
    }

    pub fn write_to_with_space(&self, writer: &mut impl std::io::Write, space: &str)
        -> Result<(), std::io::Error>
    {
        self.start_fill.write_to_or_else(writer, space)?;
        writer.write("{".as_bytes())?;
        for (idx, entry) in self.entries.iter().enumerate() {
            if idx == 0 {
                entry.write_to(writer)?;
            } else {
                entry.write_to_with_space(writer, " ")?;
            }
        }
        self.end_fill.write_to(writer)?;
        writer.write("}".as_bytes())?;
        Ok(())
    }

    pub fn to_json(&self) -> Result<serde_json::Value, std::io::Error>
    {
        let mut map = serde_json::Map::with_capacity(self.entries.len());
        for entry in self.entries.iter() {
            entry.add_to_json(&mut map)?;
        }
        Ok(serde_json::Value::Object(map))
    }

    pub fn recover_fill(&mut self, other: &Self)
    {
        self.start_fill.recover(&other.start_fill);
        for (entry, other_entry) in self.entries.iter_mut().zip(other.entries.iter()) {
            entry.recover_fill(other_entry);
        }
        self.end_fill.recover(&other.end_fill);
    }
}

impl From<Vec<CafMapEntry>> for CafMap
{
    fn from(entries: Vec<CafMapEntry>) -> Self
    {
        Self {
            start_fill: CafFill::default(),
            entries,
            end_fill: CafFill::default(),
        }
    }
}

/*
Parsing:
*/

//-------------------------------------------------------------------------------------------------------------------