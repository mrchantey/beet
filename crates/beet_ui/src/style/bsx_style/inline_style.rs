//! [`InlineStyle`]: a one-off style rule for one element, the markup twin of
//! the Rust `inline_class!` macro and what `bx:style` rewrites to.
//!
//! Its value is space-separated `prop=value` pairs in the BSX enum-value
//! grammar, reusing the `<Rule>` declaration surface ([`apply_declarations`]):
//!
//! ```html
//! <div bx:style="display=Flex flex-direction=Column align-items=Center
//!                max-width=Rem(40.0) margin-inline=Auto" />
//! ```
//!
//! It lowers to the same runtime as `inline_class!` ([`register_inline_rule`]):
//! a one-off [`Rule`] keyed on the minted class is inserted into the [`RuleSet`]
//! and the class is added to the element's [`Classes`]. The class is a
//! [`ClassName::Inline`] derived from the declarations' content rather than a
//! `panic::Location`, so the selector name differs from the Rust one but the
//! resolved style is identical.
use crate::prelude::*;
use crate::style::apply_declarations;
use beet_core::prelude::*;
use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::world::DeferredWorld;

/// A one-off style rule for its element: `bx:style="display=Flex"` is
/// `{InlineStyle{declarations:"display=Flex"}}`.
///
/// Inserting it registers its rule in the [`RuleSet`], keyed on a class
/// derived from the declarations ([`inline_style_rule`]), so identical
/// declarations share one class and one rule, and adds that class to the
/// element's [`Classes`]. Plain data, so a save keeps it and a load registers
/// its rule again; a binary without it holds the name as unregistered.
#[derive(Debug, Default, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component, Default)]
#[component(on_insert = InlineStyle::on_insert())]
pub struct InlineStyle {
	/// The `prop=value ..` declarations, ie `display=Flex max-width=Rem(40.0)`.
	pub declarations: SmolStr,
}

impl InlineStyle {
	/// A one-off rule of `declarations`.
	pub fn new(declarations: impl Into<SmolStr>) -> Self {
		Self {
			declarations: declarations.into(),
		}
	}

	fn on_insert() -> impl FnOnce(DeferredWorld, HookContext) {
		|mut world, cx| {
			let Some(declarations) = world
				.get::<Self>(cx.entity)
				.map(|style| style.declarations.clone())
			else {
				return;
			};
			let (class, rule) = match inline_style_rule(&declarations) {
				Ok(parsed) => parsed,
				Err(err) => {
					world.commands().handle_command_error::<Self>(err);
					return;
				}
			};
			// the rule lands synchronously where it can, as a `<Rule>`'s does:
			// a stylesheet baked later in the same build reads the rule set
			// before any command flush
			if let Some(mut rules) = world.get_resource_mut::<RuleSet>()
				&& !rules.has_inline(rule.selector())
			{
				rules.try_insert_inline(rule.clone());
			}
			world.commands().entity(cx.entity).queue(
				move |mut entity: EntityWorldMut| {
					register_inline_rule(&mut entity, class, rule)
				},
			);
		}
	}
}

/// The one-off [`Rule`] an [`InlineStyle`]'s `source` declares and the class
/// that keys it, for any pass that makes a declaration on an element's behalf
/// (a fence info word) to register through [`register_inline_rule`] exactly as
/// the component would, so identical declarations share one class and one
/// rule.
///
/// The class is keyed on the declaration content, never a source span: a
/// markdown fragment's span is relative to its own HTML block, so two
/// declarations on different file lines could collide on one span-derived
/// class and clobber each other's rule.
pub(crate) fn inline_style_rule(source: &str) -> Result<(ClassName, Rule)> {
	let class = ClassName::from_inline_source(source);
	let rule = parse_inline_style(class.clone(), source)?;
	Ok((class, rule))
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
/// value through the BSX value grammar ([`ValueExpr::parse_str`]) so `Flex` /
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
				None => AttrValue::Expr(ValueExpr::parse_str(raw)?),
			};
			Ok((SmolStr::from(key), value))
		})
		.collect()
}

#[cfg(test)]
mod test {
	use super::inline_style_rule;
	use crate::prelude::*;
	use crate::style::common_props;
	use crate::style::material::MaterialStylePlugin;
	use beet_core::prelude::*;

	/// Spawn `markup` (a single element carrying `bx:style`) into a Material world
	/// and return the built element entity plus the live [`RuleSet`].
	fn inline_style_world(markup: &str) -> (World, Entity) {
		let mut world = MaterialStylePlugin::world();
		let body = SceneBody::parse_bsx(&world, markup).unwrap();
		let element = world.spawn_template(body).unwrap().id();
		world.flush();
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
		let mut world =
			(ParsePlugin, MaterialStylePlugin::default()).into_world();
		let root = world.spawn_empty().id();
		let bytes = MediaBytes::new_markdown(
			"<div bx:style=\"display=Flex max-width=Rem(40.0)\">inner</div>",
		);
		DocParsers::parse(ParseContext::new(
			&mut world.entity_mut(root),
			&bytes,
		))
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
		let mut world =
			(ParsePlugin, MaterialStylePlugin::default()).into_world();
		let root = world.spawn_empty().id();
		// two HTML blocks, each a `bx:style` div with a different declaration.
		let bytes = MediaBytes::new_markdown(
			"<div bx:style=\"max-width=Rem(10.0)\">a</div>\n\n\
			 <div bx:style=\"max-width=Rem(20.0)\">b</div>",
		);
		DocParsers::parse(ParseContext::new(
			&mut world.entity_mut(root),
			&bytes,
		))
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
		inline_style_rule("not-a-prop=Flex")
			.unwrap_err()
			.to_string()
			.xpect_contains("not a known style property");
	}

	/// The component is plain data, so a saved element keeps it and a load
	/// registers its rule and class again.
	#[cfg(feature = "json")]
	#[beet_core::test]
	fn survives_a_save_and_a_load() {
		let (mut world, element) =
			inline_style_world(r#"<div bx:style="max-width=Rem(12.0)"/>"#);
		let bytes = TemplateSaver::new()
			.save_roots(&mut world, MediaType::Json, [element])
			.unwrap();
		bytes
			.as_utf8()
			.unwrap()
			.xref()
			.xpect_contains("InlineStyle");
		let mut loaded = MaterialStylePlugin::world();
		let element = TemplateLoader::new(&mut loaded).load(&bytes).unwrap()[0];
		loaded.flush();
		let class = loaded
			.entity(element)
			.get::<Classes>()
			.unwrap()
			.iter()
			.next()
			.unwrap()
			.clone();
		loaded
			.resource::<RuleSet>()
			.rules()
			.find(|rule| {
				rule.selector() == &Selector::Class(class.as_selector())
			})
			.unwrap()
			.get_typed::<style::Length>(&common_props::MaxWidth.into())
			.unwrap()
			.xpect_eq(style::Length::Rem(12.0));
	}
}
