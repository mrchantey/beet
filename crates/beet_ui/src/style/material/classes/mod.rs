//! The shared class-name vocabulary and the Material Design 3 rules that style it.
//!
//! Each submodule colocates a category's [`ClassName`] constants with the
//! [`Rule`]s that map them to design tokens, so the names that widgets *emit*
//! and the rules that *style* them live side by side. The constants are the
//! contract between the two layers: widgets ([`scene`] feature) emit these
//! classes and this rule set ([`style`] feature, Material Design 3 today) maps
//! them to design tokens.
//!
//! Exported as `pub mod classes`, so callers always reach a class name through
//! the `classes::` prefix, eg `classes::CARD_FILLED` rather than a bare
//! `CARD_FILLED`.
//!
//! [`scene`]: https://docs.rs/beet_ui
//! [`style`]: https://docs.rs/beet_ui
use crate::prelude::*;
use crate::style::*;
use crate::style::material::typography as type_tokens;

pub mod buttons;
pub mod color_scheme;
pub mod forms;
pub mod geometry;
pub mod layout;
pub mod sidebar;
pub mod table;
pub mod typography;
pub mod utilities;

pub use buttons::*;
pub use color_scheme::*;
pub use forms::*;
pub use geometry::*;
pub use layout::*;
pub use sidebar::*;
pub use table::*;
pub use typography::*;
pub use utilities::*;

/// Returns all Material Design component rules.
pub fn all_rules() -> Vec<Rule> {
	let mut rules = vec![
		button_base(),
		button_filled(),
		button_outlined(),
		button_text(),
		button_tonal(),
		button_elevated(),
		button_secondary(),
		button_tertiary(),
		button_error(),
		button_icon(),
		card_filled(),
		card_elevated(),
		card_outlined(),
		text_display_large(),
		text_display_medium(),
		text_display_small(),
		text_headline_large(),
		text_headline_medium(),
		text_headline_small(),
		text_title_large(),
		text_title_medium(),
		text_title_small(),
		text_body_large(),
		text_body_medium(),
		text_body_small(),
		text_label_large(),
		text_label_medium(),
		text_label_small(),
		color_primary(),
		shape_none(),
		shape_extra_small(),
		shape_small(),
		shape_medium(),
		shape_large(),
		shape_extra_large(),
		shape_full(),
		elevation_0(),
		elevation_1(),
		elevation_2(),
		elevation_3(),
		elevation_4(),
		elevation_5(),
		app_bar(),
		app_bar_scrolled(),
		app_bar_nav(),
		app_bar_title(),
		app_bar_terminal(),
		footer(),
		footer_side(),
		container(),
		// prose overrides — appended so they win the tag cascade over the
		// user-agent element defaults (later rule wins ties)
		link_prose(),
		mark_prose(),
		code_prose(),
		pre_prose(),
		blockquote_prose(),
		// terminal-only heading hue, gated behind `@media` Terminal
		terminal_headings(),
		main_content(),
		page(),
		// form controls — state/compound rules first so they win the cascade
		input_focus(),
		select_focus(),
		form_layout(),
		label_field(),
		input_base(),
		input_outlined(),
		input_filled(),
		input_text(),
		select_base(),
		select_outlined(),
		select_filled(),
		select_text(),
		error_text(),
		// table (the `.table-vertical-borders` column dividers are drawn per target:
		// an adjacent-sibling rule in `reset.css` on the web, the
		// `apply_table_vertical_borders` decorate system on the terminal)
		table(),
		table_th(),
		table_td(),
		// sidebar (the generic disclosure rules live in `style::elements`)
		sidebar_summary(),
		sidebar_branch(),
		sidebar_caret(),
		sidebar_caret_collapsed(),
		sidebar_list(),
		sidebar(),
		sidebar_web(),
		sidebar_link(),
		sidebar_active(),
		sidebar_item(),
		sidebar_label(),
		// terminal padding-strip last so it wins the tie over both the web
		// `.sidebar-link` and `.sidebar-label` padding (later rule wins ties)
		sidebar_link_terminal(),
		// utilities
		hidden(),
		text_align(TEXT_LEFT, TextAlign::Left),
		text_align(TEXT_CENTER, TextAlign::Center),
		text_align(TEXT_RIGHT, TextAlign::Right),
		text_size(TEXT_XS, type_tokens::FontSizeLabelSmall),
		text_size(TEXT_SM, type_tokens::FontSizeBodySmall),
		text_size(TEXT_BASE, type_tokens::FontSizeBodyLarge),
		text_size(TEXT_LG, type_tokens::FontSizeTitleLarge),
		text_size(TEXT_XL, type_tokens::FontSizeHeadlineSmall),
		text_size(TEXT_2XL, type_tokens::FontSizeHeadlineMedium),
		// print utilities — gated behind `@media print`
		print_hidden(),
		page_break(),
		// reduced motion — gated behind `@media (prefers-reduced-motion)`
		reduced_motion(),
		// interaction — shared hover affordance for buttons and links
		interactive_transition(),
		hover_dim(),
		// accessibility — global state rules
		disabled_state(),
		// web-only overrides — gated behind `@media screen`, ignored by charcell
		page_fill_viewport(),
		container_grow_web(),
	];
	// prose heading sizes (the MD3 type scale per `<h1>`..`<h6>`); appended so
	// these tag rules win the cascade over the user-agent heading defaults.
	rules.extend(heading_sizes());
	rules
}


#[cfg(test)]
mod tests {
	use super::*;
	use crate::style::material::*;
	use beet_core::prelude::*;

	/// CSS map covering both the material tokens and the common props the
	/// component rules reference.
	fn css_map() -> CssTokenMap {
		default_token_map().with_extend(common_props::token_map())
	}

	#[beet_core::test]
	fn component_rules_css() {
		let rule_set = RuleSet::new(Rule::new()).with_rules(vec![
			error_text(),
			input_base(),
			input_outlined(),
			input_focus(),
			table_th(),
			sidebar_summary(),
			hidden(),
			text_align(TEXT_CENTER, TextAlign::Center),
		]);
		CssBuilder::default()
			.with_minify(false)
			.with_format_variables(FormatVariables::short())
			.build(&css_map(), &rule_set)
			.unwrap()
			.xpect_snapshot();
	}

	#[beet_core::test]
	fn all_rules_emit_selectors() {
		let css = CssBuilder::default()
			.with_format_variables(FormatVariables::short())
			.build(&css_map(), &RuleSet::new(Rule::new()).with_rules(all_rules()))
			.unwrap();
		// compound `.input:focus` exercises Selector::AllOf serialization
		css.as_str()
			.xpect_contains(".input:focus")
			.xpect_contains(".btn")
			.xpect_contains(".btn-error")
			.xpect_contains(".error-text")
			.xpect_contains(".sidebar-summary")
			.xpect_contains(".hidden")
			.xpect_contains(".text-center")
			.xpect_contains(":disabled")
			// print utilities serialize wrapped in an `@media print` at-rule
			.xpect_contains("@media print")
			.xpect_contains(".print-hidden")
			.xpect_contains("break-after")
			// reduced-motion serializes wrapped in its own `@media` at-rule
			.xpect_contains("@media (prefers-reduced-motion: reduce)")
			.xpect_contains("transition-duration");
	}

	/// A media-gated rule serializes wrapped in its `@media` at-rule, with the
	/// selector + declaration nested inside the block.
	#[beet_core::test]
	fn print_rule_wraps_in_media_block() {
		let css = CssBuilder::default()
			.with_minify(true)
			.with_format_variables(FormatVariables::short())
			.build(
				&css_map(),
				&RuleSet::new(Rule::new()).with_rules(vec![print_hidden()]),
			)
			.unwrap();
		// `@media print{ .print-hidden { display: none; } }`
		css.as_str()
			.xpect_contains("@media print{")
			.xpect_contains(".print-hidden")
			.xpect_contains("display: none;");
		// the at-rule wraps the selector (appears before it in the output)
		(css.find("@media print").unwrap() < css.find(".print-hidden").unwrap())
			.xpect_true();
	}

	/// The reduced-motion rule serializes wrapped in its `@media` at-rule and
	/// zeroes both transition and animation duration.
	#[beet_core::test]
	fn reduced_motion_wraps_in_media_block() {
		let css = CssBuilder::default()
			.with_minify(true)
			.with_format_variables(FormatVariables::short())
			.build(
				&css_map(),
				&RuleSet::new(Rule::new()).with_rules(vec![reduced_motion()]),
			)
			.unwrap();
		css.as_str()
			.xpect_contains("@media (prefers-reduced-motion: reduce){")
			.xpect_contains("transition-duration: 0ms;")
			.xpect_contains("animation-duration: 0ms;");
	}

	/// Terminal cascade: every heading level resolves its foreground to the
	/// theme's `Primary` via the `Terminal`-gated [`terminal_headings`] rule,
	/// overriding the plain-bold user-agent heading default.
	#[beet_core::test]
	fn terminal_headings_resolve_to_primary() {
		let mut world = (MaterialStylePlugin::default(), StylePlugin).into_world();
		let heading = world.spawn(rsx! { <h1/> }).id();
		world.with_state::<RuleSetQuery, _>(|query| {
			let foreground =
				query.resolve(heading, common_props::ForegroundColor).unwrap();
			let primary = query.resolve(heading, colors::Primary).unwrap();
			foreground.xpect_eq(primary);
		});
	}

	/// Charcell path: a `.error-text` span resolves its foreground through the
	/// cascade to the same color as the `Error` token (light `:root` fallback).
	#[beet_core::test]
	fn error_text_resolves_to_error_color() {
		let mut world = MaterialStylePlugin::world();
		let entity = world
			.spawn(rsx! { <span {Classes::new([ERROR_TEXT])}/> })
			.id();
		world.with_state::<RuleSetQuery, _>(|query| {
			let foreground =
				query.resolve(entity, common_props::ForegroundColor).unwrap();
			let error = query.resolve(entity, colors::Error).unwrap();
			foreground.xpect_eq(error);
		});
	}
}
