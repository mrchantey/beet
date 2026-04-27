//! General purpose rules that may cover multiple
//! schemas
#![cfg_attr(rustfmt, rustfmt_skip)]
use crate::style::*;
use crate::style::material::*;

pub fn hero_heading()->Rule{
	Rule::new()
		.with_rule(Selector::class("hero-heading"))
		.with_token::<common_props::ForegroundColor, colors::OnPrimary>()
		// .with_typed::<common_props::BackgroundColor, colors::Primary>()
		// .with_typed::<common_props::Font, typography::TitleMedium>()
}
