#[cfg(feature = "html_parser")]
mod html;
#[cfg(feature = "html_parser")]
pub use html::*;
#[cfg(feature = "markdown_parser")]
mod markdown;
#[cfg(feature = "markdown_parser")]
pub use markdown::*;
mod node_parser;
mod plaintext;
mod span_tracker;
pub use node_parser::*;
pub use plaintext::*;
pub use span_tracker::*;
