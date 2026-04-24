#![cfg_attr(rustfmt, rustfmt_skip)]
use crate::prelude::*;
use crate::style::material::colors;
use crate::style::*;
use crate::token2;
use beet_core::prelude::*;

token2!(BackgroundColor, Property2, Property2::new("background-color", Inheritance::Inherited, colors::Primary::default()));
token2!(ForegroundColor, Property2, Property2::new("color", Inheritance::Inherited, colors::OnPrimary::default()));
