// #![allow(unused, dead_code)]
mod beet_finished_loading;
#[cfg(target_arch = "wasm32")]
mod postmessage_input;
#[cfg(target_arch = "wasm32")]
pub use postmessage_input::*;

mod example_plugin;
mod example_plugin_2d;
mod example_plugin_3d;
pub use example_plugin::*;
pub use example_plugin_2d::*;
pub use example_plugin_3d::*;
mod dialog_panel;
pub use beet_finished_loading::*;
pub use dialog_panel::*;
mod auto_spawn;
pub use auto_spawn::*;
mod follow_cursor;
pub use follow_cursor::*;
mod randomize_position;
pub use randomize_position::*;
mod render_text;
pub use render_text::*;
mod wrap_around;
pub use wrap_around::*;
