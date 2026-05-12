//! ECS Character cell layout and rendering engine.
//!
//! Charcell represents each cell as an entity.
//!
//! The rendering pipeline runs three distinct phases:
//! - **Measure** (post-order): computes [`IntrinsicSize`] for each node
//! - **Layout** (pre-order): assigns a [`LayoutRect`] to each node
//! - **Paint** (per node): draws box model and text into the [`Buffer`]
mod backend;
// escape sequences are used by both terminal and ansi_term renderers
pub mod escape;
#[cfg(feature = "terminal")]
mod input;
mod plugin;
mod renderer;
#[cfg(feature = "terminal")]
mod terminal;
pub use backend::*;
pub use buffer::*;
#[cfg(feature = "terminal")]
pub use input::*;
pub use layout::LayoutRect;
pub use measure::IntrinsicSize;
pub use plugin::*;
pub use renderer::*;
#[cfg(feature = "terminal")]
pub use terminal::*;
mod buffer;
mod layout;
mod measure;
mod paint;
pub use paint::*;
mod box_model;
mod flex;
mod text;
pub(self) use box_model::*;
pub(self) use flex::*;
pub(self) use layout::*;
pub(self) use measure::*;
pub(self) use text::*;
