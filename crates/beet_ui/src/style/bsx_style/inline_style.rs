//! The `bx:style` directive: the no-code analogue of the Rust `inline_class!`
//! macro.
//!
//! `bx:style` declares a **one-off** style rule for a single element and attaches
//! a unique, source-derived class to it, exactly as `inline_class!` does in Rust.
//! Its value is space-separated `prop=value` pairs in the BSX enum-value grammar,
//! reusing the `<Rule>` declaration surface ([`apply_declarations`]):
//!
//! ```html
//! <div bx:style="display=Flex flex-direction=Column align-items=Center
//!                max-width=Rem(40.0) margin-inline=Auto" />
//! ```
//!
//! It lowers to the same runtime as `inline_class!` ([`register_inline_rule`]):
//! a one-off [`Rule`] keyed on the minted class is inserted into the [`RuleSet`]
//! and the class is added to the element's [`Classes`]. The class is a
//! [`ClassName::Inline`] derived from the directive's BSX [`FileSpan`] rather than
//! a `panic::Location`, so the selector name differs from the Rust one but the
//! resolved style is identical.
use crate::prelude::*;
use crate::style::apply_declarations;
use beet_core::prelude::*;

/// Register the `bx:style` handler into the [`StyleResolver`] seam, so a
/// `bx:style` directive declares a one-off rule and attaches a span-derived
/// inline class at build time.
pub fn register_inline_style(world: &mut World) {
	world.get_resource_or_init::<StyleResolver>().set(
		|entity, source, _span| {
			// key the class on the declaration content, not the BSX span: a
			// markdown fragment's span is relative to its own HTML block, so two
			// `bx:style` directives on different file lines can collide on one
			// span-derived class and clobber each other's rule.
			let class = ClassName::from_inline_source(source);
			let rule = parse_inline_style(class.clone(), source)?;
			register_inline_rule(entity, class, rule)
		},
	);
}

/// Build the one-off [`Rule`] of a `bx:style` directive: seed it with the
/// element's inline-class selector, then apply each `prop=value` declaration
/// through the shared [`apply_declarations`] helper (the same path `<Rule>`
/// uses), so a value parses identically to the typed `Rule::with_value`.
fn parse_inline_style(class: ClassName, source: &str) -> Result<Rule> {
	let rule = Rule::new().with_selector(Selector::Class(class.as_selector()));
	let declarations = parse_declaration_pairs(source)?;
	let borrowed = declarations
		.iter()
		.map(|(key, value)| (key.as_str(), value));
	apply_declarations(rule, "bx:style", borrowed)
}

/// Split a `bx:style` value into `(kebab-prop-name, value)` pairs, parsing each
/// value through the BSX value grammar ([`parse_value_expr_str`]) so `Flex` /
/// `Rem(40.0)` reflect exactly as in the typed API. A `"@token:Role"` binding is
/// kept as a string for [`apply_declarations`] to recognise.
fn parse_declaration_pairs(source: &str) -> Result<Vec<(SmolStr, AttrValue)>> {
	source
		.split_whitespace()
		.map(|pair| {
			let (key, raw) = pair.split_once('=').ok_or_else(|| {
				bevyhow!(
					"`bx:style`: `{pair}` must be a `prop=value` pair, \
					 ie `display=Flex`"
				)
			})?;
			// a `@token:Role` binding stays a string; everything else is a value
			// expression resolved against the property's type downstream.
			let value = match raw.strip_prefix("@token:") {
				Some(_) => AttrValue::Str(raw.into()),
				None => AttrValue::Expr(parse_value_expr_str(raw)?),
			};
			Ok((SmolStr::from(key), value))
		})
		.collect()
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use crate::style::common_props;
	use crate::style::material::MaterialStylePlugin;
	use beet_core::prelude::*;

	/// Spawn `markup` (a single element carrying `bx:style`) into a Material world
	/// and return the built element entity plus the live [`RuleSet`].
	fn inline_style_world(markup: &str) -> (World, Entity) {
		let mut world = MaterialStylePlugin::world();
		let container = world
			.spawn_template(BsxTemplate::container(
				parse_document(markup, &BsxParseConfig::bsx()).unwrap(),
				BsxTemplateRegistry::default(),
			))
			.unwrap()
			.id();
		world.flush();
		let element = world.entity(container).get::<Children>().unwrap()[0];
		(world, element)
	}

	#[beet_core::test]
	fn lowers_to_inline_class_and_rule() {
		let (world, element) = inline_style_world(
			r#"<div bx:style="display=Flex flex-direction=Vertical align-items=Center max-width=Rem(40.0)"/>"#,
		);
		// the element gained a source-derived inline class.
		let classes = world.entity(element).get::<Classes>().unwrap();
		let class = classes.iter().next().unwrap();
		matches!(class, ClassName::Inline { .. }).xpect_true();
		let selector = Selector::Class(class.as_selector());

		// a matching one-off rule landed in the RuleSet with the parsed
		// declarations, identical to what `inline_class!` would register.
		let rule = world
			.resource::<RuleSet>()
			.rules()
			.find(|rule| rule.selector() == &selector)
			.unwrap();
		rule.get_typed::<style::Display>(&common_props::DisplayProp.into())
			.unwrap()
			.xpect_eq(style::Display::Flex);
		rule.get_typed::<style::Direction>(
			&common_props::FlexDirectionProp.into(),
		)
		.unwrap()
		.xpect_eq(style::Direction::Vertical);
		rule.get_typed::<style::AlignItems>(
			&common_props::AlignItemsProp.into(),
		)
		.unwrap()
		.xpect_eq(style::AlignItems::Center);
		rule.get_typed::<style::Length>(&common_props::MaxWidth.into())
			.unwrap()
			.xpect_eq(style::Length::Rem(40.0));
	}

	/// A `bx:style` directive on a block element embedded in **markdown** lowers
	/// the same as in `.bsx`: the element gains a source-derived inline class and
	/// a matching one-off rule lands in the `RuleSet`. Regression for the markdown
	/// path silently dropping the directive (the hero stayed unstyled on the
	/// landing page), the `.md`/`.bsx` parity gap the A8 rehearsal surfaced.
	#[cfg(feature = "markdown_parser")]
	#[beet_core::test]
	fn markdown_bx_style_lowers_to_inline_class() {
		let mut world = MaterialStylePlugin::world();
		let root = world.spawn_empty().id();
		let bytes = MediaBytes::new_markdown(
			"<div bx:style=\"display=Flex max-width=Rem(40.0)\">inner</div>",
		);
		MarkdownParser::new()
			.parse(ParseContext::new(&mut world.entity_mut(root), &bytes))
			.unwrap();
		world.flush();
		// the embedded div gained a source-derived inline class.
		let div = world.entity(root).get::<Children>().unwrap()[0];
		let classes = world.entity(div).get::<Classes>().unwrap();
		let class = classes.iter().next().unwrap();
		matches!(class, ClassName::Inline { .. }).xpect_true();
		// and its declarations landed in the live RuleSet.
		let selector = Selector::Class(class.as_selector());
		world
			.resource::<RuleSet>()
			.rules()
			.find(|rule| rule.selector() == &selector)
			.unwrap()
			.get_typed::<style::Length>(&common_props::MaxWidth.into())
			.unwrap()
			.xpect_eq(style::Length::Rem(40.0));
	}

	/// Two distinct `bx:style` directives embedded in **markdown** mint distinct
	/// inline classes and both rules land. Regression for the span collision: a
	/// markdown fragment's span is relative to its own HTML block, so the two
	/// directives shared one span-derived class and the second rule was dropped
	/// by `try_insert_inline` (the `docs/design/grid` showcase lost its third
	/// grid's column/row template).
	#[cfg(feature = "markdown_parser")]
	#[beet_core::test]
	fn markdown_distinct_bx_styles_get_distinct_classes() {
		let mut world = MaterialStylePlugin::world();
		let root = world.spawn_empty().id();
		// two HTML blocks, each a `bx:style` div with a different declaration.
		let bytes = MediaBytes::new_markdown(
			"<div bx:style=\"max-width=Rem(10.0)\">a</div>\n\n\
			 <div bx:style=\"max-width=Rem(20.0)\">b</div>",
		);
		MarkdownParser::new()
			.parse(ParseContext::new(&mut world.entity_mut(root), &bytes))
			.unwrap();
		world.flush();
		// each div carries its own distinct inline class.
		let kids = world.entity(root).get::<Children>().unwrap();
		let class_of = |entity: Entity| {
			world
				.entity(entity)
				.get::<Classes>()
				.unwrap()
				.iter()
				.next()
				.unwrap()
				.clone()
		};
		let first = class_of(kids[0]);
		let second = class_of(kids[1]);
		first.clone().xpect_not_eq(second.clone());
		// and both one-off rules landed (the second was previously clobbered).
		let rule_set = world.resource::<RuleSet>();
		let width_of = |class: ClassName| {
			rule_set
				.rules()
				.find(|rule| {
					rule.selector() == &Selector::Class(class.as_selector())
				})
				.unwrap()
				.get_typed::<style::Length>(&common_props::MaxWidth.into())
				.unwrap()
		};
		width_of(first).xpect_eq(style::Length::Rem(10.0));
		width_of(second).xpect_eq(style::Length::Rem(20.0));
	}

	#[beet_core::test]
	fn unknown_prop_errors() {
		let mut world = MaterialStylePlugin::world();
		world
			.spawn_template(BsxTemplate::container(
				parse_document(
					r#"<div bx:style="not-a-prop=Flex"/>"#,
					&BsxParseConfig::bsx(),
				)
				.unwrap(),
				BsxTemplateRegistry::default(),
			))
			.err()
			.unwrap()
			.to_string()
			.xpect_contains("not a known style property");
	}
}
