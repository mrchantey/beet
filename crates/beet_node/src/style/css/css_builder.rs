use crate::prelude::*;
use crate::style::*;
use beet_core::prelude::*;
use bevy::reflect::Typed;

pub fn default_token_map() -> CssTokenMap {
	common_props::token_map().merge(material::token_map())
}


pub struct CssPlugin;

impl Plugin for CssPlugin {
	fn build(&self, app: &mut App) { app.insert_resource(default_token_map()); }
}

#[derive(Get)]
pub struct CssBuilder<'a, 'w, 's> {
	minify: bool,
	css_token_map: &'a CssTokenMap,
	style_query: &'a StyleQuery<'w, 's>,
}


impl CssBuilder<'_, '_, '_> {
	pub fn build(&self, entity: Entity) -> Result<String> {
		let rules = self
			.style_query
			.collect_rules(entity)
			.into_iter()
			.xtry_map(|rule| self.build_rule(rule))?;

		match self.minify {
			true => Ok(rules.join(" ")),
			false => Ok(rules.join("\n\n")),
		}
	}

	fn build_rule(&self, rule: &Rule) -> Result<String> {
		let rules = self.rules_to_css(&rule.rules());

		let properties = rule
			.declarations()
			.iter()
			.xtry_map(|(key, value)| -> Result<_> {
				self.css_token_map.resolve(self, key, value)
			})?
			.into_iter()
			.flatten()
			.map(|(key, value)| format!("{key}: {value}"));


		match self.minify {
			true => {
				format!(
					"{} {{ {} }}",
					rules,
					properties.collect::<Vec<_>>().join("; ")
				)
			}
			false => {
				format!(
					"{} {{\n{}\n}}",
					rules,
					properties
						.into_iter()
						.map(|prop| format!("    {prop};"))
						.collect::<Vec<_>>()
						.join("\n")
				)
			}
		}
		.xok()
	}


	fn rules_to_css(&self, rules: &[Selector]) -> String {
		if rules.is_empty() {
			return "*".to_string();
		} else {
			rules
				.iter()
				.map(|rule| self.rule_to_css(rule))
				.collect::<Vec<_>>()
				.join(" ")
		}
	}

	fn rule_to_css(&self, rule: &Selector) -> String {
		match rule {
			Selector::Root => ":root".to_string(),
			Selector::Tag(tag) => tag.to_string(),
			Selector::Class(class) => format!(".{}", class),
			Selector::State(ElementState::Hovered) => ":hover".to_string(),
			Selector::State(ElementState::Focused) => ":focus".to_string(),
			Selector::State(ElementState::Pressed) => ":active".to_string(),
			Selector::State(ElementState::Selected) => {
				"[aria-selected=\"true\"]".to_string()
			}
			Selector::State(ElementState::Dragged) => {
				"[data-dragging=\"true\"]".to_string()
			}
			Selector::State(ElementState::Disabled) => ":disabled".to_string(),
			Selector::State(ElementState::Custom(val)) => {
				// TODO needs design work
				format!("[data-state-{}]", val)
			}
			Selector::Attribute { key, value } => match value {
				Some(value) => format!("[{}=\"{}\"]", key, value),
				None => format!("[{}]", key),
			},
			Selector::Not(inner) => {
				format!(":not({})", self.rules_to_css(inner.as_ref()))
			}
		}
	}

	pub fn props_value_to_css<
		V: 'static
			+ Send
			+ Sync
			+ FromReflect
			+ Typed
			+ TypedTokenKey
			+ AsCssValues,
	>(
		&self,
		keys: Vec<CssKey>,
		value: &TokenValue,
	) -> Result<Vec<(CssKey, CssValue)>> {
		let values = CssValue::from_token_value::<V>(value)?;
		if keys.len() != values.len() {
			bevybail!(
				"Property count mismatch:\nkeys: {keys:#?}\nvalues:{values:#?}",
			);
		}
		keys.into_iter().zip(values).collect::<Vec<_>>().xok()
	}


	/// Used in CssToken declarations section.
	/// The value type will be checked for multiple properties, and
	/// appended to the key ident if found.
	pub fn key_value_to_css<
		K: TypedTokenKey,
		V: 'static
			+ Send
			+ Sync
			+ FromReflect
			+ Typed
			+ TypedTokenKey
			+ AsCssValues,
	>(
		&self,
		value: &TokenValue,
	) -> Result<Vec<(CssKey, CssValue)>> {
		let key = CssVariable::from_token_key(&K::token_key());
		let values = CssValue::from_token_value::<V>(value)?;
		let props = V::properties();
		if props.len() <= 1 {
			// no need for suffix for zero or one props
			key.xinto::<CssKey>().xvec()
		} else {
			if props.len() != values.len() {
				bevybail!(
					"Property count mismatch:\nkeys: {props:#?}\nvalues:{values:#?}",
				);
			}
			props
				.into_iter()
				.map(|prop| key.with_suffix(prop.to_string()).xinto::<CssKey>())
				.collect::<Vec<_>>()
		}
		.into_iter()
		.zip(values)
		.collect::<Vec<_>>()
		.xok()
	}
}

struct CssRule {
	selectors: Vec<Selector>,
	declarations: HashMap<CssKey, CssValue>,
}

/// The right hand side of a css declaration
#[derive(Debug, Clone)]
pub enum CssKey {
	/// A variable, ie `--color-primary`
	Variable(CssVariable),
	/// A css property, ie `background-color`
	Property(SmolStr),
}

impl CssKey {
	pub fn static_property(name: &'static str) -> Self {
		Self::Property(SmolStr::new_static(name))
	}
}
impl From<CssVariable> for CssKey {
	fn from(var: CssVariable) -> Self { Self::Variable(var) }
}

impl std::fmt::Display for CssKey {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			CssKey::Variable(var) => var.as_css_key().fmt(f),
			CssKey::Property(prop) => prop.fmt(f),
		}
	}
}

/// The right hand side of a css declaration
#[derive(Debug, Clone)]
pub enum CssValue {
	/// A variable, ie `var(--color-primary)`
	Variable(CssVariable),
	/// A raw expression, ie `rgb(0,0,0)`
	Expression(String),
}

impl CssValue {
	pub fn expression(value: impl Into<String>) -> Self {
		Self::Expression(value.into())
	}

	pub fn from_token_value<
		V: 'static
			+ Send
			+ Sync
			+ FromReflect
			+ Typed
			+ TypedTokenKey
			+ AsCssValues,
	>(
		value: &TokenValue,
	) -> Result<Vec<Self>> {
		value.schema().assert_eq::<V>()?;
		match value {
			TokenValue::Value(value) => {
				value.value().into_reflect::<V>()?.as_css_values()
			}
			TokenValue::Token(token) => Self::from_token::<V>(token).xok(),
		}
	}
	/// Represent tokens as css values, appending the property names in
	/// the case there are multiple
	pub fn from_token<T: AsCssValues>(token: &Token) -> Vec<Self> {
		let var = CssVariable::from_token_key(token.key());
		let props = T::properties();
		if props.len() <= 1 {
			// no need for suffix for no declared props
			var.xinto::<Self>().xvec()
		} else {
			props
				.into_iter()
				.map(|prop| var.with_suffix(prop.to_string()).xinto::<Self>())
				.collect::<Vec<_>>()
		}
	}
}

impl std::fmt::Display for CssValue {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			CssValue::Variable(var) => var.as_css_value().fmt(f),
			CssValue::Expression(expr) => expr.fmt(f),
		}
	}
}

impl From<CssVariable> for CssValue {
	fn from(var: CssVariable) -> Self { Self::Variable(var) }
}

/// A css variable, the inner string
/// is stored without the leading `--`
#[derive(Debug, Clone)]
pub struct CssVariable(String);

impl CssVariable {
	pub fn from_token_key(token_key: &TokenKey) -> Self {
		use heck::ToKebabCase;
		let token_key = token_key.to_string().to_kebab_case().replace("/", "-");
		Self(token_key)
	}
	pub fn as_css_key(&self) -> String { self.to_string() }
	pub fn as_css_value(&self) -> String { format!("var({})", self) }
	pub fn with_suffix(&self, suffix: impl Into<SmolStr>) -> Self {
		Self(format!("{}-{}", self.0, suffix.into()))
	}
}
impl std::fmt::Display for CssVariable {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "--{}", self.0)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::style::common_props;
	use crate::style::material::*;

	#[test]
	fn test_color() {
		let mut world = World::new();

		world.insert_resource(
			CssTokenMap::default()
				.insert(colors::OnPrimary)
				.insert(tones::Primary20)
				.insert(common_props::ForegroundColor),
		);

		world.insert_resource(
			RuleStore::default()
				.with(rules::hero_heading())
				.with(
					Rule::root()
						.with_token::<colors::OnPrimary, tones::Primary20>(),
				)
				.with(
					Rule::root()
						.with_value::<tones::Primary20>(Color::srgb(0., 1., 0.))
						.unwrap(),
				),
		);
		let css = world
			.spawn(rsx! {
				<div class="text-primary">hello world!</div>
			})
			.with_state::<(Res<CssTokenMap>, StyleQuery), _>(
				|entity, state| {
					CssBuilder {
						minify: false,
						css_token_map: &state.0,
						style_query: &state.1,
					}
					.build(entity)
					.xunwrap()
				},
			);
		// println!("{css}");
		css
			.xpect_contains(".hero-heading")
			.xpect_contains("color: var(--io-crates-beet-node-style-material-colors-on-primary)")
			.xpect_contains(":root")
			.xpect_contains("--io-crates-beet-node-style-material-colors-on-primary: var(--io-crates-beet-node-style-material-tones-primary20)")
			.xpect_contains("--io-crates-beet-node-style-material-tones-primary20: rgb(0, 255, 0)");
	}
	#[test]
	fn test_color_role() {
		let mut world = World::new();

		world.insert_resource(
			CssTokenMap::default()
				.insert(colors::Primary)
				.insert(colors::OnPrimary)
				.insert(tones::Primary80)
				.insert(tones::Primary20)
				.insert(colors::PrimaryRole)
				.insert(common_props::ColorRoleProps),
		);

		world.insert_resource(
			RuleStore::default()
				.with(
					Rule::new()
						.with_rule(Selector::class("primary-role"))
						.with_token::<common_props::ColorRoleProps, colors::PrimaryRole>(
					),
				)
				.with(
					Rule::root()
						.with_token::<colors::Primary, tones::Primary80>()
						.with_token::<colors::OnPrimary, tones::Primary20>()
						.with_value::<colors::PrimaryRole>(ColorRole {
							background: colors::Primary::token(),
							foreground: colors::OnPrimary::token(),
						})
						.unwrap()
						.with_value::<tones::Primary80>(Color::srgb(
							0., 0.8, 0.,
						))
						.unwrap()
						.with_value::<tones::Primary20>(Color::srgb(
							0., 0.2, 0.,
						))
						.unwrap(),
				),
		);
		let css = world
			.spawn(rsx! {
				<div class="text-primary">hello world!</div>
			})
			.with_state::<(Res<CssTokenMap>, StyleQuery), _>(
				|entity, state| {
					CssBuilder {
						minify: false,
						css_token_map: &state.0,
						style_query: &state.1,
					}
					.build(entity)
					.xunwrap()
				},
			);
		// println!("{css}");
		css
			.xpect_contains(".primary-role")
			.xpect_contains("background-color: var(--io-crates-beet-node-style-material-colors-primary-role-background-color)")
			.xpect_contains("color: var(--io-crates-beet-node-style-material-colors-primary-role-color)")
			.xpect_contains(":root")
			.xpect_contains("--io-crates-beet-node-style-material-colors-primary: var(--io-crates-beet-node-style-material-tones-primary80)")
			.xpect_contains("--io-crates-beet-node-style-material-tones-primary20: rgb(0, 51, 0)")
			.xpect_contains("--io-crates-beet-node-style-material-colors-primary-role-background-color: var(--io-crates-beet-node-style-material-colors-primary)")
			.xpect_contains("--io-crates-beet-node-style-material-colors-primary-role-color: var(--io-crates-beet-node-style-material-colors-on-primary)")
			.xpect_contains("--io-crates-beet-node-style-material-tones-primary80: rgb(0, 204, 0)")
			.xpect_contains("--io-crates-beet-node-style-material-colors-on-primary: var(--io-crates-beet-node-style-material-tones-primary20)");
	}
}
