//! Common bsx utilities
use crate::prelude::*;
use beet_core::prelude::*;

/// An empty entity bundle, shorthand of `<entity/>` also works
#[template]
pub fn empty_entity() -> impl Bundle { () }
