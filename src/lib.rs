pub mod actions;
pub mod click;
pub mod code;
mod diff;
pub mod editor;
#[cfg(feature = "crossterm")]
pub mod editor_crossterm;
pub mod history;
pub mod render;
pub mod selection;
pub mod theme;
pub mod types;
pub mod utils;
mod view;
