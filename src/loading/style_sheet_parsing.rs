use crate::*;

use bevy::reflect::TypeRegistry;
use bevy::reflect::serde::TypedReflectDeserializer;
use serde::de::DeserializeSeed;
use ron::{Map, Value};

use std::any::TypeId;
use std::collections::HashMap;
use std::sync::Arc;

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn is_using_entry(depth: usize, count: usize, key: &str) -> bool
{
    // Check if the very first key is "using".
    depth == 0 && count == 0 && key == "using"
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn is_style_entry(key: &str) -> bool
{
    // Check if camelcase
    let Some(first_char) = key.chars().next() else { return false; };
    first_char.is_uppercase()
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn get_style_meta<'a>(
    type_registry  : &'a TypeRegistry,
    file           : &StyleFile,
    current_path   : &StylePath,
    short_name     : &str,
    name_shortcuts : &mut HashMap<&'static str, &'static str>,
) -> Option<(&'static str, &'static str, TypeId, TypedReflectDeserializer<'a>)>
{
    // Check if we already have this mapping.
    let mut found_mapping = false;
    let registration = match name_shortcuts.get(short_name)
    {
        Some(long_name) =>
        {
            found_mapping = true;
            type_registry.get_with_type_path(long_name)
        }
        None => type_registry.get_with_short_type_path(short_name)
    };

    // Look up the longname
    let Some(registration) = registration
    else
    {
        tracing::error!("failed getting long type name for {:?} at {:?} in {:?}; if the type is ambiguous because \
            there are multiple types with this short name, add its long name to the stylesheet file's 'using' section",
            short_name, current_path, file);
        return None;
    };

    let short_name = registration.type_info().type_path_table().short_path();  //get static version
    let long_name = registration.type_info().type_path_table().path();

    // Save this mapping for later.
    if !found_mapping { name_shortcuts.insert(short_name, long_name); }

    // Deserializer
    let deserializer = TypedReflectDeserializer::new(registration, type_registry);

    Some((short_name, long_name, registration.type_info().type_id(), deserializer))
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn get_inherited_style_value(
    file         : &StyleFile,
    current_path : &StylePath,
    short_name   : &str,
    style_entry  : &Vec<ReflectedStyle>, 
) -> Option<ReflectedStyle>
{
    // Try to inherit the last style entry in the stack.
    let Some(inherited) = style_entry.last()
    else
    {
        tracing::error!("failed inheriting {:?} at {:?} in {:?}, no inheritable value found",
            short_name, current_path, file);
        return None;
    };

    Some(inherited.clone())
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn get_style_value(
    file         : &StyleFile,
    current_path : &StylePath,
    short_name   : &str,
    value        : Value,
    style_entry  : &Vec<ReflectedStyle>,
    deserializer : TypedReflectDeserializer,
) -> Option<ReflectedStyle>
{
    match &value
    {
        Value::String(stringvalue) if (stringvalue.as_str() == "inherited") =>
        {
            get_inherited_style_value(file, current_path, short_name, style_entry)
        },
        _ =>
        {
            match deserializer.deserialize(value)
            {
                Ok(value) => Some(ReflectedStyle::Value(Arc::new(value))),
                Err(err)  => Some(ReflectedStyle::DeserializationFailed(Arc::new(err))),
            }
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn handle_using_entry(
    type_registry  : &TypeRegistry,
    file           : &StyleFile,
    value          : Value,
    name_shortcuts : &mut HashMap<&'static str, &'static str>,
){
    let Value::Seq(longnames) = value
    else
    {
        tracing::error!("failed parsing 'using' section in {:?}, it is not an Array", file);
        return;
    };

    for longname in longnames.iter()
    {
        let Value::String(longname) = longname
        else
        {
            tracing::error!("failed parsing longname {:?} in 'using' section of {:?}, it is not a String",
                longname, file);
            continue;
        };

        let Some(registration) = type_registry.get_with_type_path(longname.as_str())
        else
        {
            tracing::error!("longname {:?} in 'using' section of {:?} not found in type registry",
                longname, file);
            continue;
        };
        let short_name = registration.type_info().type_path_table().short_path();
        let long_name = registration.type_info().type_path_table().path();  //get static version

        name_shortcuts.insert(short_name, long_name);
    }
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn handle_style_entry(
    type_registry  : &TypeRegistry,
    stylesheet     : &mut StyleSheet,
    file           : &StyleFile,
    current_path   : &StylePath,
    short_name     : &str,
    value          : Value,
    name_shortcuts : &mut HashMap<&'static str, &'static str>,
    style_stack    : &mut HashMap<&'static str, Vec<ReflectedStyle>>,
    stack_tracker  : &mut Vec<(&'static str, usize)>,
){
    // Get the style's longname.
    let Some((short_name, long_name, type_id, deserializer)) =
        get_style_meta(type_registry, file, current_path, short_name, name_shortcuts)
    else { return; };

    // Get the style's value.
    let style_entry = style_stack.entry(short_name).or_insert_with(|| Vec::default());
    let starting_len = style_entry.len();

    let Some(style_value) = get_style_value(file, current_path, short_name, value, &style_entry, deserializer)
    else { return; };

    // Save this style.
    style_entry.push(style_value.clone());
    stack_tracker.push((short_name, starting_len));

    stylesheet.insert(&StyleRef{ file: file.clone(), path: current_path.clone() }, style_value, type_id, long_name);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn handle_branch_entry(
    type_registry  : &TypeRegistry,
    stylesheet     : &mut StyleSheet,
    file           : &StyleFile,
    current_path   : &StylePath,
    key            : &str,
    value          : Value,
    depth          : usize,
    name_shortcuts : &mut HashMap<&'static str, &'static str>,
    style_stack    : &mut HashMap<&'static str, Vec<ReflectedStyle>>,
    stack_trackers : &mut Vec<Vec<(&'static str, usize)>>,
){
    let Value::Map(data) = value
    else
    {
        tracing::error!("failed parsing extension {:?} at {:?} in {:?}, extension is not an Object",
            key, current_path, file);
        return;
    };

    let extended_path = current_path.extend(key);
    parse_branch(
        type_registry,
        stylesheet,
        &file,
        &extended_path,
        data,
        depth + 1,
        name_shortcuts,
        style_stack,
        stack_trackers,
    );
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

fn parse_branch(
    type_registry  : &TypeRegistry,
    stylesheet     : &mut StyleSheet,
    file           : &StyleFile,
    current_path   : &StylePath,
    mut data       : Map,
    depth          : usize,
    name_shortcuts : &mut HashMap<&'static str, &'static str>,
    style_stack    : &mut HashMap<&'static str, Vec<ReflectedStyle>>,
    stack_trackers : &mut Vec<Vec<(&'static str, usize)>>,
){
    let mut stack_tracker = stack_trackers.pop().unwrap_or_default();

    for (count, (key, value)) in data.iter_mut().enumerate()
    {
        let Value::String(key) = key
        else
        {
            tracing::warn!("ignoring non-string key in stylesheet {:?} at {:?}", file, current_path);
            continue;
        };
        let value = std::mem::replace(value, Value::Unit);

        if is_using_entry(depth, count, key)
        {
            handle_using_entry(
                type_registry,
                file,
                value,
                name_shortcuts,
            );
        }
        else if is_style_entry(key)
        {
            handle_style_entry(
                type_registry,
                stylesheet,
                file,
                current_path,
                key.as_str(),
                value,
                name_shortcuts,
                style_stack,
                &mut stack_tracker,
            );
        }
        else
        {
            handle_branch_entry(
                type_registry,
                stylesheet,
                file,
                current_path,
                key.as_str(),
                value,
                depth,
                name_shortcuts,
                style_stack,
                stack_trackers,
            );
        }
    }

    // Clear styles tracked for inheritance.
    for (shortname, initial_size) in stack_tracker.drain(..)
    {
        style_stack.get_mut(&shortname)
            .unwrap()
            .truncate(initial_size);
    }
    stack_trackers.push(stack_tracker);
}

//-------------------------------------------------------------------------------------------------------------------
//-------------------------------------------------------------------------------------------------------------------

/// Consumes a stylesheet file's data and loads it into [`StyleSheet`].
pub(crate) fn parse_stylesheet_file(type_registry: &TypeRegistry, stylesheet: &mut StyleSheet, file: StyleFile, data: Value)
{
    tracing::info!("parsing stylesheet {:?}", file.file);
    stylesheet.initialize_file(file.clone());

    let Value::Map(data) = data
    else
    {
        tracing::error!("failed parsing stylesheet {:?}, data base layer is not an Object", file);
        return;
    };

    // [ shortname : longname ]
    let mut name_shortcuts: HashMap<&'static str, &'static str> = HashMap::default();
    // [ shortname : [ style value ] ]
    let mut style_stack: HashMap<&'static str, Vec<ReflectedStyle>> = HashMap::default();
    // [ {shortname, top index into stylestack when first stack added this frame} ]
    let mut stack_trackers: Vec<Vec<(&'static str, usize)>> = Vec::default();

    // Recursively consume the file contents.
    parse_branch(
        type_registry,
        stylesheet,
        &file,
        &StylePath::new(""),
        data,
        0,
        &mut name_shortcuts,
        &mut style_stack,
        &mut stack_trackers,
    );
}

//-------------------------------------------------------------------------------------------------------------------
