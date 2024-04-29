use std::any::type_name;

use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;
use bevy_cobweb::prelude::*;
use serde::{Deserialize, Serialize};

use crate::*;

//-------------------------------------------------------------------------------------------------------------------

trait ApplyToDims: Send + Sync + 'static
{
    fn apply_to_dims(self, dims: &mut Dims);
}

trait ApplyToContentFlex: Send + Sync + 'static
{
    fn apply_to_content_flex(self, content: &mut ContentFlex);
}

trait ApplyToSelfFlex: Send + Sync + 'static
{
    fn apply_to_self_flex(self, flex: &mut SelfFlex);
}

//-------------------------------------------------------------------------------------------------------------------

fn initialize_absolute_style(
    In(entity): In<Entity>,
    mut c: Commands,
    query: Query<(Has<React<AbsoluteStyle>>, Has<React<FlexStyle>>)>,
)
{
    let Ok((maybe_absolute, maybe_flex)) = query.get(entity) else { return };

    // Check absolute style.
    if maybe_absolute {
        return;
    }

    // Check flex style.
    if maybe_flex {
        tracing::warn!("tried initializing absolute style on entity {:?} that has flex style", entity);
        return;
    }

    // Insert absolute style.
    c.react().insert(entity, AbsoluteStyle::default());
}

//-------------------------------------------------------------------------------------------------------------------

fn initialize_flex_style(
    In(entity): In<Entity>,
    mut c: Commands,
    query: Query<(Has<React<AbsoluteStyle>>, Has<React<FlexStyle>>)>,
)
{
    let Ok((maybe_absolute, maybe_flex)) = query.get(entity) else { return };

    // Check flex style.
    if maybe_flex {
        return;
    }

    // Check absolute style.
    if maybe_absolute {
        tracing::warn!("tried initializing flex style on entity {:?} that has absolute style", entity);
        return;
    }

    // Insert flex style.
    c.react().insert(entity, FlexStyle::default());
}

//-------------------------------------------------------------------------------------------------------------------

fn apply_to_dims<T: ApplyToDims>(
    In((entity, param)): In<(Entity, T)>,
    mut c: Commands,
    mut query: Query<(Option<&mut React<AbsoluteStyle>>, Option<&mut React<FlexStyle>>)>,
)
{
    let Ok((maybe_absolute, maybe_flex)) = query.get_mut(entity) else { return };

    // Prioritize absolute style.
    if let Some(mut absolute) = maybe_absolute {
        param.apply_to_dims(&mut absolute.get_mut(&mut c).dims);
        return;
    }

    // Check flex style.
    if let Some(mut flex) = maybe_flex {
        param.apply_to_dims(&mut flex.get_mut(&mut c).dims);
        return;
    }

    // Fall back to inserting absolute style.
    let mut style = AbsoluteStyle::default();
    param.apply_to_dims(&mut style.dims);
    c.react().insert(entity, style);
}

//-------------------------------------------------------------------------------------------------------------------

fn _apply_to_content_flex<T: ApplyToContentFlex>(
    In((entity, param)): In<(Entity, T)>,
    mut c: Commands,
    mut query: Query<(Option<&mut React<AbsoluteStyle>>, Option<&mut React<FlexStyle>>)>,
)
{
    let Ok((maybe_absolute, maybe_flex)) = query.get_mut(entity) else { return };

    // Prioritize absolute style.
    if let Some(mut absolute) = maybe_absolute {
        param.apply_to_content_flex(&mut absolute.get_mut(&mut c).content);
        return;
    }

    // Check flex style.
    if let Some(mut flex) = maybe_flex {
        param.apply_to_content_flex(&mut flex.get_mut(&mut c).content);
        return;
    }

    // Fall back to inserting absolute style.
    let mut style = AbsoluteStyle::default();
    param.apply_to_content_flex(&mut style.content);
    c.react().insert(entity, style);
}

//-------------------------------------------------------------------------------------------------------------------

fn _apply_to_self_flex<T: ApplyToSelfFlex>(
    In((entity, param)): In<(Entity, T)>,
    mut c: Commands,
    mut query: Query<(Has<React<AbsoluteStyle>>, Option<&mut React<FlexStyle>>)>,
)
{
    let Ok((has_absolute, maybe_flex)) = query.get_mut(entity) else { return };

    // Check absolute style.
    if has_absolute {
        tracing::warn!("tried to apply {} to {:?} that has AbsoluteStyle; only FlexStyle is supported",
            type_name::<T>(), entity);
        return;
    }

    // Check flex style.
    if let Some(mut flex) = maybe_flex {
        param.apply_to_self_flex(&mut flex.get_mut(&mut c).flex);
        return;
    }

    // Fall back to inserting flex style.
    let mut style = FlexStyle::default();
    param.apply_to_self_flex(&mut style.flex);
    c.react().insert(entity, style);
}

//-------------------------------------------------------------------------------------------------------------------

/// Initializes [`AbsoluteStyle`] on an entity.
///
/// Should be inserted before all other style field wrappers.
#[derive(Reflect, Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WithAbsoluteStyle;

impl ApplyLoadable for WithAbsoluteStyle
{
    fn apply(self, ec: &mut EntityCommands)
    {
        let id = ec.id();
        ec.commands().syscall(id, initialize_absolute_style);
    }
}
impl ThemedAttribute for WithAbsoluteStyle
{
    type Value = ();
    fn update(ec: &mut EntityCommands, _value: Self::Value)
    {
        Self.apply(ec);
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Initializes [`FlexStyle`] on an entity.
///
/// Should be inserted before all other style field wrappers.
#[derive(Reflect, Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WithFlexStyle;

impl ApplyLoadable for WithFlexStyle
{
    fn apply(self, ec: &mut EntityCommands)
    {
        let id = ec.id();
        ec.commands().syscall(id, initialize_flex_style);
    }
}
impl ThemedAttribute for WithFlexStyle
{
    type Value = ();
    fn update(ec: &mut EntityCommands, _value: Self::Value)
    {
        Self.apply(ec);
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Mirrors [`Dims::width`], can be loaded as a style.
#[derive(Reflect, Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Width(pub Val);

impl ApplyToDims for Width
{
    fn apply_to_dims(self, dims: &mut Dims)
    {
        dims.width = self.0;
    }
}

impl ApplyLoadable for Width
{
    fn apply(self, ec: &mut EntityCommands)
    {
        let id = ec.id();
        ec.commands().syscall((id, self), apply_to_dims);
    }
}

impl ThemedAttribute for Width
{
    type Value = Val;
    fn update(ec: &mut EntityCommands, value: Self::Value)
    {
        Width(value).apply(ec);
    }
}
impl ResponsiveAttribute for Width
{
    type Interactive = Interactive;
}
impl AnimatableAttribute for Width
{
    type Interactive = Interactive;
}

//-------------------------------------------------------------------------------------------------------------------

pub(crate) struct UiStyleFieldWrappersPlugin;

impl Plugin for UiStyleFieldWrappersPlugin
{
    fn build(&self, app: &mut App)
    {
        app.register_themed::<WithAbsoluteStyle>()
            .register_themed::<WithFlexStyle>()
            .register_animatable::<Width>();
    }
}

//-------------------------------------------------------------------------------------------------------------------