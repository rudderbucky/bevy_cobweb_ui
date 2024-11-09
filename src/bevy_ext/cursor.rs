use std::borrow::Cow;

use bevy::prelude::*;
use bevy::window::{PrimaryWindow, SystemCursorIcon};
use bevy::winit::cursor::{CursorIcon, CustomCursor};
use ui_style::prelude::InteractiveVals;

use crate::prelude::*;

//-------------------------------------------------------------------------------------------------------------------

#[derive(Resource, Default)]
struct CursorSource
{
    primary: Option<LoadableCursor>,
    temporary: Option<LoadableCursor>,
}

impl CursorSource
{
    /// Returns a cursor if either the primary or temporary is set.
    ///
    /// Clears the temporary.
    fn get_next_cursor(&mut self, img_map: &mut ImageMap, asset_server: &AssetServer) -> Option<CursorIcon>
    {
        let cursor = self.temporary.take().or_else(|| self.primary.clone())?;

        Some(cursor.into_cursor_icon(img_map, asset_server)?)
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Iterates available `TempCursors` to extract the current temp cursor.
fn get_temp_cursor(mut source: ResMut<CursorSource>, temps: Query<(Entity, &TempCursor)>)
{
    for (idx, (entity, temp)) in temps
        .iter()
        .filter(|(_, t)| !matches!(***t, LoadableCursor::None))
        .enumerate()
    {
        if idx != 0 {
            warn_once!("multiple TempCursor instances detected (second: {:?} {:?}); only one can be used at a \
                time; this warning only prints once", entity, temp);
            return;
        }

        source.temporary = Some((**temp).clone());
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Sets the cursor if we have anything loaded in the `CursorSource`. Does nothing if no cursor is loaded, in case
/// the user wants to manage the cursor with a custom approach.
fn refresh_cursor_icon(
    asset_server: Res<AssetServer>,
    mut c: Commands,
    mut source: ResMut<CursorSource>,
    window: Query<(Entity, Option<&CursorIcon>), With<PrimaryWindow>>,
    mut img_map: ResMut<ImageMap>,
)
{
    let Ok((window_entity, current_cursor)) = window.get_single() else { return };
    let next_cursor = source.get_next_cursor(&mut img_map, &asset_server);
    if current_cursor == next_cursor.as_ref() {
        return;
    }
    let Some(next_cursor) = next_cursor else { return };

    c.entity(window_entity).insert(next_cursor);
}

//-------------------------------------------------------------------------------------------------------------------

/// A cursor type that can be loaded via [`SetPrimaryCursor`] or [`TempCursor`].
#[derive(Reflect, Default, Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub enum LoadableCursor
{
    /// `None` means the loadable cursor should be ignored.
    ///
    /// Used as a default in `Responsive<TempCursor>`, where you need to set an `idle` value.
    #[default]
    None,
    /// Mirrors [`CustomCursor`].
    Custom
    {
        /// Image path. It is recommended (but not required) to pre-load the image via [`LoadedImages`].
        ///
        /// The image must be in 8 bit int or 32 bit float rgba. PNG images work well for this.
        image: Cow<'static, str>,
        /// X and Y coordinates of the hotspot in pixels. The hotspot must be within the image bounds.
        hotspot: (u16, u16),
    },
    /// A URL to an image to use as the cursor.
    ///
    /// Only usable on WASM targets.
    Url
    {
        /// Web URL to an image to use as the cursor. PNGs preferred. Cursor
        /// creation can fail if the image is invalid or not reachable.
        url: Cow<'static, str>,
        /// X and Y coordinates of the hotspot in pixels. The hotspot must be within the image bounds.
        hotspot: (u16, u16),
    },
    System(SystemCursorIcon),
}

impl LoadableCursor
{
    pub fn into_cursor_icon(self, img_map: &mut ImageMap, asset_server: &AssetServer) -> Option<CursorIcon>
    {
        match self {
            Self::None => None,
            Self::Custom { image, hotspot } => {
                let handle = img_map.get_or_load(image, asset_server);
                Some(CursorIcon::Custom(CustomCursor::Image { handle, hotspot }))
            }
            Self::Url { url, hotspot: _hotspot } => {
                #[cfg(all(target_family = "wasm", target_os = "unknown"))]
                {
                    Some(CursorIcon::Custom(CustomCursor::Url { url, hotspot: _hotspot }))
                }

                #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
                {
                    warn_once!("failed making cursor icon from URL {:?}; only WASM targets are supported, but the target \
                        is not WASM; this warning only prints once", url);
                    None
                }
            }
            Self::System(icon) => Some(CursorIcon::System(icon)),
        }
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Command that sets the primary [`CursorIcon`] on the [`PrimaryWindow`].
///
/// The primary icon can be temporarily overridden by a [`TempCursor`].
#[derive(Reflect, Default, Debug, Clone, PartialEq, Deref, DerefMut)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct SetPrimaryCursor(pub LoadableCursor);

impl Command for SetPrimaryCursor
{
    fn apply(self, world: &mut World)
    {
        world.resource_mut::<CursorSource>().primary = Some(self.0);
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Component that refreshes [`CursorIcon`] on the [`PrimaryWindow`] every tick. Set the value to
/// [`LoadableCursor::None`]` to disable it.
///
/// To set a long-term 'primary cursor', use the [`SetPrimaryCursor`] command.
///
/// See [`HoverCursor`] and [`PressCursor`] for easier ways to use this.
#[derive(Component, Reflect, Default, Debug, Clone, PartialEq, Deref, DerefMut)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct TempCursor(pub LoadableCursor);

impl Instruction for TempCursor
{
    fn apply(self, entity: Entity, world: &mut World)
    {
        let _ = world.get_entity_mut(entity).map(|mut emut| {
            emut.insert(self);
        });
    }

    fn revert(entity: Entity, world: &mut World)
    {
        let _ = world.get_entity_mut(entity).map(|mut emut| {
            emut.remove::<Self>();
        });
    }
}

impl ThemedAttribute for TempCursor
{
    type Value = Self;
    fn construct(value: Self::Value) -> Self
    {
        value
    }
}
impl ResponsiveAttribute for TempCursor {}

//-------------------------------------------------------------------------------------------------------------------

/// Instruction that sets [`TempCursor`] on the entity when it is hovered.
#[derive(Component, Reflect, Default, Debug, Clone, PartialEq, Deref, DerefMut)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct HoverCursor(pub LoadableCursor);

impl Instruction for HoverCursor
{
    fn apply(self, entity: Entity, world: &mut World)
    {
        let responsive = Responsive::<TempCursor> {
            values: InteractiveVals {
                idle: TempCursor(LoadableCursor::None),
                hover: Some(TempCursor(self.0)),
                ..default()
            },
            ..default()
        };
        responsive.apply(entity, world);
    }

    fn revert(entity: Entity, world: &mut World)
    {
        Responsive::<TempCursor>::revert(entity, world);
    }
}

//-------------------------------------------------------------------------------------------------------------------

/// Instruction that sets [`TempCursor`] on the entity when it is pressed.
#[derive(Component, Reflect, Default, Debug, Clone, PartialEq, Deref, DerefMut)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct PressCursor(pub LoadableCursor);

impl Instruction for PressCursor
{
    fn apply(self, entity: Entity, world: &mut World)
    {
        let responsive = Responsive::<TempCursor> {
            values: InteractiveVals {
                idle: TempCursor(LoadableCursor::None),
                press: Some(TempCursor(self.0)),
                ..default()
            },
            ..default()
        };
        responsive.apply(entity, world);
    }

    fn revert(entity: Entity, world: &mut World)
    {
        Responsive::<TempCursor>::revert(entity, world);
    }
}

//-------------------------------------------------------------------------------------------------------------------

pub(crate) struct CursorPlugin;

impl Plugin for CursorPlugin
{
    fn build(&self, app: &mut App)
    {
        app.init_resource::<CursorSource>()
            .register_command_type::<SetPrimaryCursor>()
            .register_responsive::<TempCursor>()
            .register_instruction_type::<HoverCursor>()
            .register_instruction_type::<PressCursor>()
            // Note: bevy's cursor_update system runs in Last but doesn't have a system set, so we need to put
            // these in PostUpdate
            .add_systems(PostUpdate, (get_temp_cursor, refresh_cursor_icon).chain());
    }
}

//-------------------------------------------------------------------------------------------------------------------