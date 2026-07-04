//! The `<Rule>` markup tag: a named style [`Rule`] declared at runtime.
//!
//! `<Rule>` is the no-code analogue of a typed `Rule` registered in Rust. A site
//! declares its named rules in markup (eg a `styles.bsx` it includes) and they
//! land in the live [`RuleSet`] at build time, resolving on both the web and
//! terminal targets like any other rule. The tag renders nothing: it is a
//! build-time effect (a [`BsxTagResolvers`] handler), the style analogue of how
//! `<PackageConfig>`/`<Theme>` patch a resource.
//!
//! ```html
//! <Rule class="design-row"
//!       display=Flex flex-wrap=Wrap align-items=Center
//!       column-gap=Rem(1.0) row-gap=Rem(1.0) />
//! <Rule class={["color-box", "primary"]}
//!       background-color="@token:Primary" color="@token:OnPrimary" />
//! ```
//!
//! The selector comes from `class`/`tag`/`state`/`media`; every other attribute
//! is a declaration whose key is a kebab property name
//! ([`prop_name_map`](crate::style::prop_name_map)) and whose value is the BSX
//! enum form (`Flex`, `Rem(1.0)`), parsed identically to the typed
//! `Rule::with_value`. A `"@token:Role"` value is a token-to-token binding
//! (`Rule::with_token`), the markup form of `color_scheme_rules`.
use crate::prelude::*;
use crate::style::color_role_map;
use crate::style::prop_name_map;
use beet_core::prelude::*;

/// Register the `<Rule>` custom-tag handler into the [`BsxTagResolvers`] seam, so
/// a `<Rule>` element declares a named rule into the [`RuleSet`] at build time.
pub fn register_rule_tag(world: &mut World) {
	world.get_resource_or_init::<BsxTagResolvers>().insert(
		"Rule",
		|el, entity| {
			let rule = parse_rule(el)?;
			entity.world_scope(|world| {
				world.get_resource_or_init::<RuleSet>().insert_rule(rule);
			});
			Ok(())
		},
	);
}

/// Build a [`Rule`] from a `<Rule>` element's attributes: the selector from
/// `class`/`tag`/`state`/`media`, one declaration per remaining attribute.
fn parse_rule(el: &BsxElement) -> Result<Rule> {
	if !el.children.is_empty() {
		bevybail!("`<Rule>` declares a style rule and cannot have children");
	}
	let mut rule = Rule::new().with_selector(parse_selector(el)?);
	if let Some(media) = parse_media(el)? {
		rule = rule.with_media(media);
	}
	let declarations = el
		.attributes
		.iter()
		.filter(|attr| !is_selector_attr(&attr.key))
		.map(|attr| (attr.key.as_str(), &attr.value));
	apply_declarations(rule, "<Rule>", declarations)
}

/// Apply a sequence of `(kebab-prop-name, value)` declarations onto `rule`, the
/// shared body of both [`parse_rule`] (the `<Rule>` tag) and the `bx:style`
/// directive. Each prop name resolves through [`prop_name_map`] and its value is
/// either a `"@token:Role"` token-to-token binding ([`Rule::with_token`]) or a
/// literal parsed against the property's value type ([`Rule::insert`]), keeping
/// the two markup surfaces in lockstep with the typed `Rule::with_value` API.
///
/// `context` names the caller for error messages, eg `<Rule>` or `bx:style`.
pub fn apply_declarations<'a>(
	mut rule: Rule,
	context: &str,
	declarations: impl IntoIterator<Item = (&'a str, &'a AttrValue)>,
) -> Result<Rule> {
	let prop_names = prop_name_map();
	let role_names = color_role_map();
	for (key, value) in declarations {
		let resolver = prop_names.get(key).ok_or_else(|| {
			bevyhow!("`{context}`: `{key}` is not a known style property")
		})?;
		rule = match token_binding(value) {
			// `"@token:Role"`: a token-to-token binding, not a literal value.
			Some(role) => {
				let token = role_names.get(role).cloned().ok_or_else(|| {
					bevyhow!(
						"`{context}`: `@token:{role}` is not a known colour role"
					)
				})?;
				rule.with_token(resolver.token.clone(), token)?
			}
			// otherwise a literal value parsed against the property's value type.
			None => {
				let value =
					resolver.parse(&attr_literal(context, key, value)?)?;
				rule.insert(resolver.token.clone(), value)?;
				rule
			}
		};
	}
	Ok(rule)
}

/// The selector attributes, which name the rule's target rather than a
/// declaration.
fn is_selector_attr(key: &str) -> bool {
	matches!(key, "class" | "tag" | "state" | "media")
}

/// Build the rule's [`Selector`] from `class`/`tag`/`state`. Multiple of these
/// compound into an [`Selector::AllOf`] (eg `class="color-box" + class child`),
/// and a `class=["a", "b"]` list is itself an `AllOf` of class selectors.
fn parse_selector(el: &BsxElement) -> Result<Selector> {
	let mut parts = Vec::new();
	if let Some(value) = attr(el, "class") {
		parts.extend(class_selectors(value)?);
	}
	if let Some(value) = attr(el, "tag") {
		parts.push(Selector::tag(string_value(value, "tag")?));
	}
	if let Some(value) = attr(el, "state") {
		parts.push(Selector::state(parse_state(&string_value(
			value, "state",
		)?)?));
	}
	match parts.len() {
		0 => bevybail!(
			"`<Rule>` needs a selector, ie `class=`, `tag=`, or `state=`"
		),
		1 => Ok(parts.remove(0)),
		_ => Ok(Selector::AllOf(parts)),
	}
}

/// The class selectors named by a `class` attribute: a single class, or every
/// class of a `["a", "b"]` list.
fn class_selectors(value: &AttrValue) -> Result<Vec<Selector>> {
	match value {
		AttrValue::Str(class) => {
			Ok(vec![Selector::class(ClassName::string(class.as_str()))])
		}
		AttrValue::Expr(ValueExpr::Literal(DataLiteral::Scalar(
			Value::Str(class),
		))) => Ok(vec![Selector::class(ClassName::string(class.as_str()))]),
		AttrValue::Expr(ValueExpr::Literal(DataLiteral::List(items))) => items
			.iter()
			.map(|item| match item {
				DataLiteral::Scalar(Value::Str(class)) => {
					Ok(Selector::class(ClassName::string(class.as_str())))
				}
				_ => bevybail!("`<Rule>` `class` list items must be strings"),
			})
			.collect(),
		_ => {
			bevybail!("`<Rule>` `class` must be a string or a list of strings")
		}
	}
}

/// Parse the `state` attribute value to an [`ElementState`], the markup form of
/// a `:hover`/`:focus`/… pseudo-class selector.
fn parse_state(state: &str) -> Result<ElementState> {
	match state {
		"hover" | "hovered" => Ok(ElementState::Hovered),
		"focus" | "focused" => Ok(ElementState::Focused),
		"active" | "pressed" => Ok(ElementState::Pressed),
		"dragged" => Ok(ElementState::Dragged),
		"disabled" => Ok(ElementState::Disabled),
		"selected" => Ok(ElementState::Selected),
		other => Ok(ElementState::Custom(other.into())),
	}
}

/// Parse the optional `media` attribute to a [`MediaQuery`] gate: one of the
/// keyword queries, or a width gate like `max-width: 1024px` (evaluated by the
/// browser via CSS and by the charcell cascade against its surface).
fn parse_media(el: &BsxElement) -> Result<Option<MediaQuery>> {
	let Some(value) = attr(el, "media") else {
		return Ok(None);
	};
	let media = string_value(value, "media")?;
	match media.as_str() {
		"terminal" => Ok(Some(MediaQuery::Terminal)),
		"screen" => Ok(Some(MediaQuery::Screen)),
		"print" => Ok(Some(MediaQuery::Print)),
		"reduced-motion" => Ok(Some(MediaQuery::ReducedMotion)),
		other => match parse_max_width(other) {
			Some(media) => Ok(Some(media)),
			None => bevybail!(
				"`<Rule>` `media=\"{other}\"` is not a known media query (terminal/screen/print/reduced-motion, or `max-width: 1024px`)"
			),
		},
	}
}

/// Parse a width-gated media value, `max-width: 1024px`, to
/// [`MediaQuery::MaxWidth`]. CSS-style parens and the `px` suffix are optional.
fn parse_max_width(value: &str) -> Option<MediaQuery> {
	let value = value.trim().trim_start_matches('(').trim_end_matches(')');
	let (key, px) = value.split_once(':')?;
	if key.trim() != "max-width" {
		return None;
	}
	px.trim()
		.trim_end_matches("px")
		.trim()
		.parse::<u32>()
		.ok()
		.map(MediaQuery::MaxWidth)
}

/// The `@token:Role` role name carried by an attribute value, if it is a quoted
/// `"@token:Role"` token-binding form.
fn token_binding(value: &AttrValue) -> Option<&str> {
	let text = match value {
		AttrValue::Str(text) => text.as_str(),
		AttrValue::Expr(ValueExpr::Literal(DataLiteral::Scalar(
			Value::Str(text),
		))) => text.as_str(),
		_ => return None,
	};
	text.strip_prefix("@token:")
}

/// The literal of a declaration attribute, for parsing against the property's
/// value type. A bare flag, spread, or `@`/`$` binding is not a style value.
fn attr_literal(
	context: &str,
	key: &str,
	value: &AttrValue,
) -> Result<DataLiteral> {
	match value {
		AttrValue::Str(string) => {
			Ok(DataLiteral::Scalar(Value::Str(string.into())))
		}
		AttrValue::Expr(ValueExpr::Literal(literal)) => Ok(literal.clone()),
		_ => bevybail!(
			"`{context}`: `{key}` must be a literal style value, ie `Flex` or `Rem(1.0)`"
		),
	}
}

/// The first attribute named `key`, if present.
fn attr<'a>(el: &'a BsxElement, key: &str) -> Option<&'a AttrValue> {
	el.attributes
		.iter()
		.find(|attr| attr.key == key)
		.map(|attr| &attr.value)
}

/// The string value of an attribute expected to be a plain string.
fn string_value(value: &AttrValue, key: &str) -> Result<SmolStr> {
	match value {
		AttrValue::Str(string) => Ok(string.into()),
		AttrValue::Expr(ValueExpr::Literal(DataLiteral::Scalar(
			Value::Str(string),
		))) => Ok(string.clone()),
		_ => bevybail!("`<Rule>` `{key}` must be a string"),
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::style::common_props;
	use crate::style::material::MaterialStylePlugin;

	/// Build a `<Rule>` markup string and return the single rule it inserted into
	/// the live [`RuleSet`] of a Material world (`register_rule_tag` runs via
	/// `BsxDefaultsPlugin`, pulled in by `MaterialStylePlugin`).
	fn rule_from(markup: &str) -> Rule {
		let mut world = MaterialStylePlugin::world();
		let before = world.resource::<RuleSet>().rules().count();
		let nodes = parse_document(markup, &BsxParseConfig::bsx()).unwrap();
		world
			.spawn_template(BsxTemplate::container(
				nodes,
				BsxTemplateRegistry::default(),
			))
			.unwrap();
		world
			.resource::<RuleSet>()
			.rules()
			.nth(before)
			.cloned()
			.unwrap()
	}

	#[beet_core::test]
	fn design_row_matches_typed() {
		let rule = rule_from(
			r#"<Rule class="design-row" display=Flex flex-wrap=Wrap align-items=Center column-gap=Rem(1.0) row-gap=Rem(1.0)/>"#,
		);
		// same selector as `design_row_rule`
		rule.selector()
			.clone()
			.xpect_eq(Selector::class("design-row"));
		// representative declarations equal the typed `with_value` forms
		rule.get_typed::<style::Display>(&common_props::DisplayProp.into())
			.unwrap()
			.xpect_eq(style::Display::Flex);
		rule.get_typed::<style::Length>(&common_props::ColumnGapProp.into())
			.unwrap()
			.xpect_eq(style::Length::Rem(1.0));
		rule.get_typed::<style::Length>(&common_props::RowGapProp.into())
			.unwrap()
			.xpect_eq(style::Length::Rem(1.0));
	}

	// the width forms all resolve to `MaxWidth`; keywords stay themselves
	#[beet_core::test]
	fn media_parses_max_width() {
		rule_from(r#"<Rule class="a" media="max-width: 1024px" display=None/>"#)
			.media()
			.xpect_eq(Some(MediaQuery::MaxWidth(1024)));
		rule_from(r#"<Rule class="b" media="(max-width: 800px)" display=None/>"#)
			.media()
			.xpect_eq(Some(MediaQuery::MaxWidth(800)));
		rule_from(r#"<Rule class="c" media="max-width:640" display=None/>"#)
			.media()
			.xpect_eq(Some(MediaQuery::MaxWidth(640)));
		rule_from(r#"<Rule class="d" media="terminal" display=None/>"#)
			.media()
			.xpect_eq(Some(MediaQuery::Terminal));
	}

	#[beet_core::test]
	fn token_binding_resolves() {
		let rule = rule_from(
			r#"<Rule class={["color-box", "primary"]} background-color="@token:Primary"/>"#,
		);
		rule.selector().clone().xpect_eq(Selector::AllOf(vec![
			Selector::class("color-box"),
			Selector::class("primary"),
		]));
		// the declaration is a token-to-token binding to the Primary role
		matches!(
			rule.get(&common_props::BackgroundColor.into()).unwrap(),
			TokenValue::Token(_)
		)
		.xpect_true();
	}
}
