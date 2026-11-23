//! The standard [`BsxDirective`]s, in every [`BsxDirectives`] set.
//!
//! Each rewrites to explicit BSX naming its target type as text, so the
//! rewrite runs in a binary lacking that type, which then holds it as any
//! unregistered name:
//!
//! ```text
//! bx:scope="user"                      {DocumentScope{path:"user"}}
//! bx:for="entries" bx:key="name"       {(ReactiveChildren{key:"name"}, @doc:entries)},
//!                                      the children wrapped in a <template>
//! bx:onclick="src"                     a child <script {OnEvent{event:"click"}}>src</script>
//! bx:style="decls"                     {InlineStyle{declarations:"decls"}}
//! ```
use crate::prelude::*;

/// `bx:scope="user"` is `{DocumentScope{path:"user"}}`: a descendant's
/// document binding reads under `user`.
#[derive(Debug, Default, Clone)]
pub struct ScopeDirective;

impl BsxDirective for ScopeDirective {
	fn pattern(&self) -> SmolStr { "bx:scope".into() }

	fn matches(&self, key: &str) -> bool { key == "bx:scope" }

	fn rewrite(&self, key: &str, element: &mut BsxElement) -> Result {
		let path = element.take_str_attribute(key)?;
		element.push_spread(SpreadItem::component("DocumentScope", [(
			"path",
			DataLiteral::Scalar(Value::new(path)),
		)]));
		Ok(())
	}
}

impl Installer for ScopeDirective {
	fn install(self, app: &mut App) { BsxDirectives::register_on(app, self); }
}

/// `bx:for="entries" bx:key="name"` is
/// `{(ReactiveChildren{key:"name"}, @doc:entries)}` with the element's
/// children wrapped in a `<template>`, the item every row clones. `bx:key`
/// is optional and means nothing without `bx:for`.
#[derive(Debug, Default, Clone)]
pub struct ForDirective;

impl ForDirective {
	const FOR: &'static str = "bx:for";
	const KEY: &'static str = "bx:key";
}

impl BsxDirective for ForDirective {
	fn pattern(&self) -> SmolStr { "bx:for".into() }

	fn matches(&self, key: &str) -> bool {
		key == Self::FOR || key == Self::KEY
	}

	fn rewrite(&self, _key: &str, element: &mut BsxElement) -> Result {
		if !element.attributes.iter().any(|attr| attr.key == Self::FOR) {
			bevybail!(
				"`{}` on `<{}>` keys the rows of a `{}`, and there is none",
				Self::KEY,
				element.tag,
				Self::FOR
			);
		}
		let field = element.take_str_attribute(Self::FOR)?;
		let reactive = match element.take_attribute(Self::KEY) {
			Some(AttrValue::Str(key)) => SpreadItem::component(
				"ReactiveChildren",
				[("key", DataLiteral::Scalar(Value::new(key)))],
			),
			Some(_) => bevybail!(
				"`{}` on `<{}>` expects a field path string, ie `{}=\"id\"`",
				Self::KEY,
				element.tag,
				Self::KEY
			),
			None => SpreadItem::component("ReactiveChildren", []),
		};
		element.push_spread_items([
			reactive,
			SpreadItem::Binding(BindingExpr::doc(FieldPath::parse(&field))),
		]);
		let children = core::mem::take(&mut element.children);
		element.self_closing = false;
		element.children = vec![BsxNode::Element(BsxElement::new(
			"template",
			Vec::new(),
			children,
		))];
		Ok(())
	}
}

impl Installer for ForDirective {
	fn install(self, app: &mut App) { BsxDirectives::register_on(app, self); }
}

/// `bx:on<event>="src"` is a child `<script {OnEvent{event:"<event>"}}>`
/// holding `src`, one directive for every event: the event name is data, so
/// a new event needs no rebuild, and the plugin owning the event runs the
/// script when it reaches the element.
#[derive(Debug, Default, Clone)]
pub struct OnEventDirective;

impl OnEventDirective {
	const PREFIX: &'static str = "bx:on";
}

impl BsxDirective for OnEventDirective {
	fn pattern(&self) -> SmolStr { "bx:on<event>".into() }

	fn matches(&self, key: &str) -> bool {
		key.len() > Self::PREFIX.len() && key.starts_with(Self::PREFIX)
	}

	fn rewrite(&self, key: &str, element: &mut BsxElement) -> Result {
		let source = element.take_attribute(key);
		let Some(AttrValue::Str(source)) = source else {
			bevybail!(
				"`{key}` expects a script string, ie \
				 `{key}=\"target.with_field('count', count => count + 1)\"`"
			);
		};
		let event = &key[Self::PREFIX.len()..];
		element.self_closing = false;
		element.children.push(BsxNode::Element(BsxElement::new(
			"script",
			vec![BsxAttribute::spread(SpreadExpr::Named(
				NamedLiteral::component("OnEvent", [(
					"event",
					DataLiteral::Scalar(Value::new(event)),
				)]),
			))],
			vec![BsxNode::Text(source)],
		)));
		Ok(())
	}
}

impl Installer for OnEventDirective {
	fn install(self, app: &mut App) { BsxDirectives::register_on(app, self); }
}

/// `bx:style="decls"` is `{InlineStyle{declarations:"decls"}}`: a one-off
/// style rule for this element, its class derived from the declarations.
/// `InlineStyle` lives where the style types do (`beet_ui`), so a binary
/// without it holds the name as unregistered.
#[derive(Debug, Default, Clone)]
pub struct StyleDirective;

impl BsxDirective for StyleDirective {
	fn pattern(&self) -> SmolStr { "bx:style".into() }

	fn matches(&self, key: &str) -> bool { key == "bx:style" }

	fn rewrite(&self, key: &str, element: &mut BsxElement) -> Result {
		let declarations = element.take_str_attribute(key)?;
		element.push_spread(SpreadItem::component("InlineStyle", [(
			"declarations",
			DataLiteral::Scalar(Value::new(declarations)),
		)]));
		Ok(())
	}
}

impl Installer for StyleDirective {
	fn install(self, app: &mut App) { BsxDirectives::register_on(app, self); }
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	/// `markup` with its directives rewritten.
	fn rewritten(markup: &str) -> Vec<BsxNode> {
		let mut nodes =
			BsxNode::parse_document(markup, &BsxParseConfig::bsx()).unwrap();
		BsxDirectives::default().rewrite(&mut nodes).unwrap();
		nodes
	}

	/// `markup` parsed as written.
	fn parsed(markup: &str) -> Vec<BsxNode> {
		BsxNode::parse_document(markup, &BsxParseConfig::bsx()).unwrap()
	}

	#[crate::test]
	fn scope_is_a_document_scope() {
		rewritten(r#"<div bx:scope="user"/>"#)
			.xpect_eq(parsed(r#"<div {DocumentScope{path:"user"}}/>"#));
	}

	#[crate::test]
	fn for_wraps_its_children_in_a_template() {
		rewritten(r#"<ul bx:key="name" bx:for="entries"><li>x</li></ul>"#)
			.xpect_eq(parsed(
				r#"<ul {(ReactiveChildren{key:"name"}, @doc:entries)}><template><li>x</li></template></ul>"#,
			));
		rewritten(r#"<ul bx:for="entries"><li>x</li></ul>"#).xpect_eq(parsed(
			r#"<ul {(ReactiveChildren, @doc:entries)}><template><li>x</li></template></ul>"#,
		));
	}

	#[crate::test]
	fn an_event_is_a_script_child() {
		rewritten(r#"<button bx:onclick="go()">+</button>"#).xpect_eq(parsed(
			r#"<button>+<script {OnEvent{event:"click"}}>go()</script></button>"#,
		));
	}

	#[crate::test]
	fn style_is_an_inline_style() {
		rewritten(r#"<div bx:style="display=Flex"/>"#).xpect_eq(parsed(
			r#"<div {InlineStyle{declarations:"display=Flex"}}/>"#,
		));
	}

	/// A rewrite's output is rewritten in turn, so a directive inside a
	/// `bx:for`'s template still applies.
	#[crate::test]
	fn rewrites_nest() {
		rewritten(r#"<ul bx:for="items"><li bx:scope="a"/></ul>"#).xpect_eq(
			parsed(
				r#"<ul {(ReactiveChildren, @doc:items)}><template><li {DocumentScope{path:"a"}}/></template></ul>"#,
			),
		);
	}

	#[crate::test]
	fn an_unmatched_key_refuses_the_document() {
		let mut nodes = parsed(r#"<div bx:slot="x"/>"#);
		BsxDirectives::default()
			.rewrite(&mut nodes)
			.unwrap_err()
			.to_string()
			.xpect_contains("`bx:slot` on `<div>`")
			.xpect_contains("`bx:on<event>`");
	}

	#[crate::test]
	fn a_key_without_its_for_refuses() {
		let mut nodes = parsed(r#"<ul bx:key="id"/>"#);
		BsxDirectives::default()
			.rewrite(&mut nodes)
			.unwrap_err()
			.to_string()
			.xpect_contains("there is none");
	}

	/// The build walk's own keys pass through untouched.
	#[crate::test]
	fn walk_keys_pass_through() {
		rewritten(r#"<div bx:ref="a" bx:cfg="feature:x"/>"#)
			.xpect_eq(parsed(r#"<div bx:ref="a" bx:cfg="feature:x"/>"#));
	}

	/// A directive installed last shadows a standard one for the keys it
	/// matches, and installing it creates the registry it lands in.
	#[crate::test]
	fn a_downstream_directive_shadows_a_standard_one() {
		#[derive(Clone)]
		struct Upper;
		impl BsxDirective for Upper {
			fn pattern(&self) -> SmolStr { "bx:scope".into() }
			fn matches(&self, key: &str) -> bool { key == "bx:scope" }
			fn rewrite(&self, key: &str, element: &mut BsxElement) -> Result {
				let path = element.take_str_attribute(key)?;
				element.attributes.push(BsxAttribute {
					key: "data-scope".into(),
					value: AttrValue::Str(path.to_uppercase()),
				});
				Ok(())
			}
		}
		impl Installer for Upper {
			fn install(self, app: &mut App) {
				BsxDirectives::register_on(app, self);
			}
		}
		let mut app = App::new();
		app.install(Upper);
		let mut nodes = parsed(r#"<div bx:scope="user" bx:onclick="go()"/>"#);
		DocParsers::of(app.world())
			.directives()
			.rewrite(&mut nodes)
			.unwrap();
		nodes.xpect_eq(parsed(
			r#"<div data-scope="USER"><script {OnEvent{event:"click"}}>go()</script></div>"#,
		));
	}
}
