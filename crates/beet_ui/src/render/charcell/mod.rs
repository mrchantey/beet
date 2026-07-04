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
#[cfg(feature = "tui")]
mod clipboard;
mod decorate;
mod disclosure;
mod double_buffer;
mod flex;
mod flex_buffer;
mod font;
pub use font::FontScale;
pub use font::from_fullwidth;
pub(self) use font::*;
mod grid;
pub(self) use grid::*;
#[cfg(feature = "tui")]
mod hit_test;
mod inline;
#[cfg(feature = "tui")]
mod input;
#[cfg(feature = "tui")]
mod input_bridge;
// the `KittyImage` data is platform-neutral (measure/paint read it); the
// attach/emission systems inside are `tui`-gated.
mod kitty;
pub use kitty::*;
mod layout;
mod measure;
mod paint;
mod plugin;
mod prepare;
mod query;
pub(self) use query::*;
mod renderer;
mod scrollbar;
pub(self) use scrollbar::*;
#[cfg(feature = "tui")]
mod scrollbar_hit_test;
#[cfg(feature = "tui")]
pub use scrollbar_hit_test::*;
#[cfg(feature = "tui")]
mod select;
#[cfg(feature = "tui")]
pub use select::*;
mod stacking;
pub(self) use stacking::*;
mod table;
pub(self) use table::*;
#[cfg(feature = "tui")]
mod terminal;
/// In-process test harness for the live TUI, reused by the interaction tasks.
#[cfg(all(test, feature = "tui"))]
pub(crate) mod test_host;
mod text;
#[cfg(feature = "tui")]
mod title;

pub use backend::*;
pub use buffer::*;
#[cfg(feature = "tui")]
pub use clipboard::*;
pub use decorate::*;
// crate-internal: the aria-controls observer + attribute/id helpers, shared
// with `widgets::sync_sidebar_breakpoint`
pub(crate) use disclosure::*;
pub use double_buffer::*;
pub use flex_buffer::*;
#[cfg(feature = "tui")]
pub use hit_test::*;
#[cfg(feature = "tui")]
pub use input::*;
#[cfg(feature = "tui")]
pub use input_bridge::*;
pub use layout::LayoutRect;
pub use measure::IntrinsicSize;
// paint/prepare expose the crate-internal `CharcellTree`, so they stay crate-visible
pub(crate) use paint::*;
pub use plugin::*;
pub(crate) use prepare::*;
#[cfg(feature = "tui")]
pub use terminal::*;
#[cfg(feature = "tui")]
pub(crate) use title::*;

pub(self) use box_model::*;
pub(self) use flex::*;
pub(self) use inline::*;
pub(self) use layout::*;
pub(self) use measure::*;
pub use text::display_width;
pub(self) use text::*;
