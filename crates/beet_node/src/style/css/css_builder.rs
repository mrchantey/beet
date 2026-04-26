use crate::prelude::*;
use crate::style::*;
use beet_core::prelude::*;
use bevy::reflect::Typed;
use std::sync::Arc;

/// Converts a value to its CSS string representation.
pub trait CssValue {
	fn to_css_value(&self, builder: &CssBuilder) -> Result<String>;
}

pub fn default_token_map() -> CssTokenMap {
	common_props::token_map().merge(material::token_map())
}


pub struct CssPlugin;

impl Plugin for CssPlugin {
	fn build(&self, app: &mut App) { app.insert_resource(default_token_map()); }
}

pub struct CssBuilder<'a, 'w, 's> {
	minify: bool,
	css_token_map: &'a CssTokenMap,
	style_query: &'a StyleQuery<'w, 's>,
}


impl CssBuilder<'_, '_, '_> {
	pub fn build(&self, entity: Entity) -> Result<String> {
		let selectors = self
			.style_query
			.collect_selectors(entity)
			.into_iter()
			.xtry_map(|selector| self.build_selector(selector))?;

		match self.minify {
			true => Ok(selectors.join(" ")),
			false => Ok(selectors.join("\n\n")),
		}
	}

	fn build_selector(&self, selector: &Selector) -> Result<String> {
		let rules = self.rules_to_css(&selector.rules());

		let properties = selector
			.tokens()
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

	/// Returns the ident in css form, using the [`CssIdentMap`]
	/// if a mapping is found, otherwise the last part of
	/// the field path as a variable.
	/// Non-specified idents are assumed to be variables, not properties.
	pub fn ident_to_css(&self, path: &TokenKey) -> Result<CssIdent> {
		use heck::ToKebabCase;
		let path = path.to_string().to_kebab_case().replace("/", "--");
		// TODO hash in prod
		CssIdent::variable(path).xok()
	}

	pub fn css_key<T: TypedTokenKey>(&self) -> Result<String> {
		self.ident_to_css(&T::token_key())?.as_css_key().xok()
	}

	fn rules_to_css(&self, rules: &[Rule]) -> String {
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

	fn rule_to_css(&self, rule: &Rule) -> String {
		match rule {
			Rule::Root => ":root".to_string(),
			Rule::Tag(tag) => tag.to_string(),
			Rule::Class(class) => format!(".{}", class),
			Rule::State(ElementState::Hovered) => ":hover".to_string(),
			Rule::State(ElementState::Focused) => ":focus".to_string(),
			Rule::State(ElementState::Pressed) => ":active".to_string(),
			Rule::State(ElementState::Selected) => {
				"[aria-selected=\"true\"]".to_string()
			}
			Rule::State(ElementState::Dragged) => {
				"[data-dragging=\"true\"]".to_string()
			}
			Rule::State(ElementState::Disabled) => ":disabled".to_string(),
			Rule::State(ElementState::Custom(val)) => {
				// TODO needs design work
				format!("[data-state-{}]", val)
			}
			Rule::Attribute { key, value } => match value {
				Some(value) => format!("[{}=\"{}\"]", key, value),
				None => format!("[{}]", key),
			},
			Rule::Not(inner) => {
				format!(":not({})", self.rules_to_css(inner.as_ref()))
			}
		}
	}

	pub fn token_value_to_css<
		T: 'static + Send + Sync + FromReflect + Typed + TypedTokenKey + CssValue,
	>(
		&self,
		value: &TokenValue,
	) -> Result<String> {
		match value {
			TokenValue::Value(value) => {
				value.schema().assert_eq::<T>()?;
				value.value().into_reflect::<T>()?.to_css_value(&self)
			}
			TokenValue::Token(token) => token.key().to_css_value(&self),
		}
	}
}

#[derive(Debug, Clone)]
pub enum CssIdent {
	/// The variable name without a leading `--`
	Variable(SmolStr),
	Property(SmolStr),
}

impl CssIdent {
	pub fn variable(name: impl Into<SmolStr>) -> Self {
		Self::Variable(name.into())
	}
	pub fn property(name: impl Into<SmolStr>) -> Self {
		Self::Property(name.into())
	}

	pub fn as_css_key(&self) -> String { self.to_string() }
	pub fn as_css_value(&self) -> String {
		match self {
			CssIdent::Variable(var) => format!("var(--{})", var),
			CssIdent::Property(prop) => prop.to_string(),
		}
	}
}

impl std::fmt::Display for CssIdent {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			CssIdent::Variable(var) => write!(f, "--{}", var),
			CssIdent::Property(prop) => write!(f, "{}", prop),
		}
	}
}

/// A token key is always represented as a css variable
/// when in the css value position, ie:
/// `foo: var(--path-to-this-token)`
impl CssValue for TokenKey {
	fn to_css_value(&self, builder: &CssBuilder) -> Result<String> {
		builder.ident_to_css(self)?.as_css_value().xok()
	}
}

impl CssValue for Color {
	fn to_css_value(&self, _builder: &CssBuilder) -> Result<String> {
		let this = self.to_srgba();
		let alpha = this.alpha;
		// still undecided about this..
		// what if user wants to overwrite
		if alpha == 1.0 {
			format!(
				"rgb({}, {}, {})",
				(this.red * 255.0).round() as u8,
				(this.green * 255.0).round() as u8,
				(this.blue * 255.0).round() as u8,
			)
		} else {
			format!(
				"rgba({}, {}, {}, {})",
				(this.red * 255.0).round() as u8,
				(this.green * 255.0).round() as u8,
				(this.blue * 255.0).round() as u8,
				alpha
			)
		}
		.xok()
	}
}

/// Store methods for looking up a schema path and resolving a value
#[derive(Default, Deref, Resource)]
pub struct CssTokenMap(
	HashMap<TokenKey, Arc<dyn 'static + Send + Sync + CssToken>>,
);
impl CssTokenMap {
	/// Registers a CSS value resolver keyed on `T::Tokens`'s type path.
	///
	/// Stored [`TypedValue`]s carry the schema of their *tokens* struct
	/// (the type actually passed to `with_value`), not the output type,
	/// so the key must match that tokens type.
	pub fn insert<T: 'static + Send + Sync + TypedTokenKey + CssToken>(
		mut self,
		token: T,
	) -> Self {
		self.0.insert(TokenKey::of::<T>(), Arc::new(token));
		self
	}

	pub fn merge(mut self, other: Self) -> Self {
		self.0.extend(other.0);
		self
	}

	pub fn resolve(
		&self,
		builder: &CssBuilder,
		key: &TokenKey,
		value: &TokenValue,
	) -> Result<Vec<(String, String)>> {
		if let Some(token) = self.0.get(key) {
			// if let Some(func) = self.0.get(value.schema()) {
			token.declarations(builder, value)
		} else {
			bevybail!("No CSS Token registered for this schema:\n{}", key)
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::style::material::*;

	#[test]
	fn test() {
		// let mut world =
		// 	(material::MaterialStylePlugin::default(), CssPlugin).into_world();
		let mut world = (CssPlugin).into_world();

		world.insert_resource(
			CssTokenMap::default()
				.insert(colors::OnPrimary)
				.insert(tones::Primary20)
				.insert(common_props::ForegroundColor),
		);

		world.insert_resource(
			SelectorStore::default()
				.with(selectors::hero_heading())
				.with(
					Selector::root()
						.with_typed::<colors::OnPrimary, tones::Primary20>(),
				)
				.with(
					Selector::root()
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
		println!("{css}");
	}
}
