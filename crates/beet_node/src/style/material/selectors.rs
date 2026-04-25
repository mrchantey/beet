//! General purpose selectors that may cover multiple
//! schemas
#![cfg_attr(rustfmt, rustfmt_skip)]
use crate::style::*;
use crate::style::material::colors;
use crate::style::material::tones;

pub fn light_scheme()->Selector{
	Selector::new().with_rule(Rule::class("light-scheme"))
		.with_typed::<colors::Primary, tones::Primary40>()

}

pub fn primary_text()->Selector{
	Selector::new().with_rule(Rule::class("text-primary"))
		.with_typed::<common_props::BackgroundColor, colors::Primary>()
		.with_typed::<common_props::ForegroundColor, colors::OnPrimary>()
}
