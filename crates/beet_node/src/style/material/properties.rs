#![cfg_attr(rustfmt, rustfmt_skip)]
use crate::style::*;
// use crate::style::material::colors;
// use crate::style::*;
// use crate::token2;
// use beet_core::prelude::*;


pub fn primary_text()->Selector{
	Selector::new().with_rule(Rule::class("text-primary"))
		.with_typed::<props::ForegroundColor2, props::BackgroundColor2>()
}
