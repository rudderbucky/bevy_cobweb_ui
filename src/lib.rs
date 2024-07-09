#![doc = include_str!("../README.md")]
#[allow(unused_imports)]
use crate as bevy_cobweb_ui;

mod assets;
mod assets_ext;
mod bevy_ext;
mod loading;
mod localization;
mod plugin;
mod react_ext;
mod sickle_ext;
mod tools;
//mod ui_2d;
mod ui_bevy;

pub(crate) use assets::*;
pub use assets_ext::*;
pub use bevy_cobweb_ui_derive::*;
pub use bevy_ext::*;
pub use loading::*;
pub use localization::*;
pub use plugin::*;
pub use react_ext::*;
pub use sickle_ext::*;
pub use tools::*;
//pub use ui_2d::*;
pub use ui_bevy::*;

pub mod prelude
{
    pub use crate::*;
}
