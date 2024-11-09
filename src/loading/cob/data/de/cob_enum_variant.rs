use serde::de::{DeserializeSeed, EnumAccess, IntoDeserializer, Unexpected, VariantAccess, Visitor};

use super::{visit_map_ref, visit_tuple_ref, visit_wrapped_erased_ref, ErasedNewtypeStruct};
use crate::prelude::*;

//-------------------------------------------------------------------------------------------------------------------

impl CobEnum
{
    #[cold]
    pub(super) fn unexpected(&self) -> Unexpected
    {
        match &self.variant {
            CobEnumVariant::Unit => Unexpected::UnitVariant,
            CobEnumVariant::Tuple(tuple) => {
                if tuple.entries.len() == 1 {
                    Unexpected::NewtypeVariant
                } else {
                    Unexpected::TupleVariant
                }
            }
            CobEnumVariant::Array(..) => Unexpected::NewtypeVariant,
            CobEnumVariant::Map(..) => Unexpected::StructVariant,
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------

pub(super) struct EnumRefDeserializer<'de>
{
    pub(super) variant: &'de CobEnum,
}

impl<'de> EnumAccess<'de> for EnumRefDeserializer<'de>
{
    type Error = CobError;
    type Variant = VariantRefAccess<'de>;

    fn variant_seed<V>(self, seed: V) -> CobResult<(V::Value, Self::Variant)>
    where
        V: DeserializeSeed<'de>,
    {
        let variant = self.variant.id.as_str().into_deserializer();
        let visitor = VariantRefAccess { variant: self.variant };
        seed.deserialize(variant).map(|v| (v, visitor))
    }
}

//-------------------------------------------------------------------------------------------------------------------

pub struct VariantRefAccess<'de>
{
    variant: &'de CobEnum,
}

impl<'de> VariantAccess<'de> for VariantRefAccess<'de>
{
    type Error = CobError;

    fn unit_variant(self) -> CobResult<()>
    {
        match &self.variant.variant {
            CobEnumVariant::Unit => Ok(()),
            _ => Err(serde::de::Error::invalid_type(
                self.variant.unexpected(),
                &"unit variant",
            )),
        }
    }

    fn newtype_variant_seed<T>(self, seed: T) -> CobResult<T::Value>
    where
        T: DeserializeSeed<'de>,
    {
        match &self.variant.variant {
            CobEnumVariant::Tuple(tuple) => {
                if tuple.entries.len() == 1 {
                    seed.deserialize(&tuple.entries[0])
                } else {
                    seed.deserialize(tuple)
                }
            }
            // Special cases: enum variant flattening
            CobEnumVariant::Array(array) => seed.deserialize(array),
            CobEnumVariant::Map(map) => seed.deserialize(map),
            CobEnumVariant::Unit => seed.deserialize(ErasedNewtypeStruct),
        }
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> CobResult<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.variant.variant {
            CobEnumVariant::Tuple(tuple) => visit_tuple_ref(&tuple.entries, visitor),
            // Special case: flattened
            CobEnumVariant::Unit => {
                if len == 0 {
                    visit_tuple_ref(&[], visitor)
                } else if len == 1 {
                    visit_wrapped_erased_ref(visitor)
                } else {
                    Err(serde::de::Error::invalid_type(
                        self.variant.unexpected(),
                        &"tuple variant",
                    ))
                }
            }
            _ => Err(serde::de::Error::invalid_type(
                self.variant.unexpected(),
                &"tuple variant",
            )),
        }
    }

    fn struct_variant<V>(self, _fields: &'static [&'static str], visitor: V) -> CobResult<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.variant.variant {
            CobEnumVariant::Map(map) => visit_map_ref(&map.entries, visitor),
            // Special case: flattened
            CobEnumVariant::Unit => visit_map_ref(&[], visitor),
            _ => Err(serde::de::Error::invalid_type(
                self.variant.unexpected(),
                &"struct variant",
            )),
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------