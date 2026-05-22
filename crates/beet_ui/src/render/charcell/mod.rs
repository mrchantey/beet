//! ECS Character cell layout and rendering engine.
//!
//! The rendering pipeline runs four distinct phases:
//! - **Prepare**: inserts [`IntrinsicSize`] and [`LayoutRect`] on new nodes
//! - **Measure** (post-order): computes [`IntrinsicSize`] for each node
//! - **Layout** (pre-order): assigns a [`LayoutRect`] to each node
//! - **Paint** (per node): draws box model and text into the [`DoubleBuffer`]
mod backend;
mod box_model;
mod buffer;
mod double_buffer;
mod flex;
mod inline;
#[cfg(feature = "terminal")]
mod input;
mod layout;
mod measure;
mod paint;
mod plugin;
mod prepare;
mod query;
pub(self) use query::*;
mod renderer;
#[cfg(feature = "terminal")]
mod terminal;
mod text;

pub use backend::*;
pub use buffer::*;
pub use double_buffer::*;
#[cfg(feature = "terminal")]
pub use input::*;
pub use layout::LayoutRect;
pub use measure::IntrinsicSize;
pub use paint::*;
pub use plugin::*;
pub use prepare::*;
#[cfg(feature = "terminal")]
pub use terminal::*;

pub(self) use box_model::*;
pub(self) use flex::*;
pub(self) use inline::*;
pub(self) use layout::*;
pub(self) use measure::*;
pub(self) use text::*;
