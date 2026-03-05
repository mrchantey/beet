#[cfg(feature = "html_parser")]
mod html;
#[cfg(feature = "html_parser")]
pub use html::*;
mod node_parser;
mod plaintext;
mod span_tracker;
pub use node_parser::*;
pub use plaintext::*;
pub use span_tracker::*;
