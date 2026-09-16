//! `Stylesheet` widget — emits the active rule set as an inline `<style>`.
//!
//! Web target only: it reads the global [`RuleSet`](crate::style) at scene
//! build via [`StyleQuery`] and bakes the built CSS into a `<style>` element,
//! so callers drop it into the document `<head>` instead of hand-assembling a
//! `<style>{css}</style>` string. A rule registered after the bake (an async
//! widget's inline rule, a `<Mermaid src>` pinning its picture's width once
//! the file lands) reaches the sheet through [`refresh_stylesheets`].
use crate::prelude::RuleSet;
use crate::style::CssBuilder;
use crate::style::FormatVariables;
use crate::style::StyleQuery;
use beet_core::prelude::*;

/// Emits the CSS built from the active rule set as an inline `<style>` element.
///
/// `minify` toggles whitespace stripping (off by default, for readable dev
/// output). Variable names use [`FormatVariables::short`].
#[template(system)]
pub fn Stylesheet(#[prop] minify: bool, style: StyleQuery) -> impl Bundle {
	let css = bake(&style, minify);
	rsx! {
		<style {BakedStylesheet { minify }}>{css}</style>
	}
}

/// A `<style>` [`Stylesheet`] baked from the rule set, its one text child the
/// css, re-baked by [`refresh_stylesheets`] when the rule set changes after.
#[derive(Component)]
pub struct BakedStylesheet {
	minify: bool,
}

/// The rule set as css, empty on a build failure (logged).
fn bake(style: &StyleQuery, minify: bool) -> String {
	style
		.build_css(
			&CssBuilder::default()
				.with_minify(minify)
				.with_format_variables(FormatVariables::short()),
		)
		.unwrap_or_else(|err| {
			error!("Stylesheet: failed to build css: {err}");
			String::new()
		})
}

/// Re-bake every [`BakedStylesheet`] when the rule set changed since this last
/// ran, so a rule registered after a sheet was built is in it. Runs in
/// `PostParseTree`, and through [`settle_stylesheets`] before a one-shot
/// render, which builds a page between frames.
pub(crate) fn refresh_stylesheets(
	rule_set: Res<RuleSet>,
	// the style query reads every `Value`, the bake lands after it
	mut params: ParamSet<(StyleQuery, Query<&mut Value>)>,
	sheets: Query<(&BakedStylesheet, &Children)>,
) {
	if !rule_set.is_changed() {
		return;
	}
	let baked: Vec<(Entity, String)> = sheets
		.iter()
		.filter_map(|(sheet, children)| {
			children
				.first()
				.map(|text| (*text, bake(&params.p0(), sheet.minify)))
		})
		.collect();
	for (text, css) in baked {
		if let Ok(mut value) = params.p1().get_mut(text) {
			value.set_if_neq(Value::str(css));
		}
	}
}

/// Bring every [`BakedStylesheet`] up to the rule set before a one-shot
/// render. A no-op in a world without one (a sheet only bakes where the style
/// resources are).
pub(crate) fn settle_stylesheets(world: &mut World) {
	if world
		.query_filtered::<(), With<BakedStylesheet>>()
		.iter(world)
		.next()
		.is_none()
	{
		return;
	}
	if let Err(err) = world.run_system_cached(refresh_stylesheets) {
		world.handle_command_error::<BakedStylesheet>(err.into());
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::prelude::*;
	use crate::style::Length;
	use crate::style::common_props::MaxWidth;
	use crate::style::material::MaterialStylePlugin;

	/// A rule registered after the sheet baked (an async widget's) is in the
	/// sheet once it settles, and a settle with nothing new leaves it be.
	#[beet_core::test]
	fn follows_late_rules() {
		let mut world =
			(TemplatePlugin, MaterialStylePlugin, StylePlugin).into_world();
		let head = world.spawn_template(rsx! { <Stylesheet/> }).unwrap().id();
		let html = |world: &mut World| {
			HtmlRenderer::new()
				.render(&mut RenderContext::new(head, world))
				.unwrap()
				.to_string()
		};
		let baked = html(&mut world);
		baked
			.xref()
			.xpect_contains("<style>")
			.xnot()
			.xpect_contains("max-width: 42px");
		// a late rule, the way a widget's inline class registers one
		world.spawn((Element::new("div"), inline_class![(
			MaxWidth,
			Length::Px(42.)
		)]));
		settle_stylesheets(&mut world);
		html(&mut world).xpect_contains("max-width: 42px");
		// steady: another settle changes nothing
		let settled = html(&mut world);
		settle_stylesheets(&mut world);
		html(&mut world).xpect_eq(settled);
	}
}
