//! Building templates is the cheapest step of the beet build process,
//! it depends entirely on static analysis of source files so requires no compilation.
mod extract_lang_templates;
pub use extract_lang_templates::*;
mod file_to_templates;
pub use file_to_templates::*;
mod md_to_web_tokens;
pub use md_to_web_tokens::*;
mod rs_to_web_tokens;
pub use rs_to_web_tokens::*;
mod build_template_maps;
mod hash_file;
mod template_watcher;
pub use build_template_maps::*;
pub use hash_file::*;
pub use template_watcher::*;
