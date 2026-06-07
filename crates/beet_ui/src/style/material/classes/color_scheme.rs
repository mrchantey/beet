//! Color-scheme class names.
//!
//! Applied to an ancestor element (eg the document root); styled by the
//! [`light_scheme`](crate::style::material::themes::light_scheme) /
//! [`dark_scheme`](crate::style::material::themes::dark_scheme) rules and themed
//! down the cascade.
#![cfg_attr(rustfmt, rustfmt_skip)]
use crate::prelude::*;

pub const LIGHT_SCHEME: ClassName = ClassName::new_static("light-scheme");
pub const DARK_SCHEME: ClassName = ClassName::new_static("dark-scheme");
