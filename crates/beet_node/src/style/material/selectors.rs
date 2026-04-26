//! General purpose selectors that may cover multiple
//! schemas
#![cfg_attr(rustfmt, rustfmt_skip)]
use crate::style::*;
use crate::style::material::*;

pub fn hero_heading()->Selector{
	Selector::new()
		.with_rule(Rule::class("hero-heading"))
		.with_token::<common_props::ForegroundColor, colors::OnPrimary>()
		// .with_typed::<common_props::BackgroundColor, colors::Primary>()
		// .with_typed::<common_props::Font, typography::TitleMedium>()
}
