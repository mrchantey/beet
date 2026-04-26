use std::sync::Arc;

use super::FontWeight as StyleFontWeight;
use crate::prelude::*;
use crate::style::*;
use beet_core::prelude::*;
use bevy::reflect::Typed;

/// Converts a value to its CSS string representation.
pub trait CssValue {
	fn to_css_value(&self, builder: &CssBuilder) -> Result<String>;
}

pub struct CssPlugin;

impl Plugin for CssPlugin {
	fn build(&self, app: &mut App) {
		app.insert_resource(default_func_map())
			.insert_resource(common_props::css_ident_map());
	}
}

pub struct CssBuilder<'a, 'w, 's> {
	minify: bool,
	ident_map: &'a CssProperties,
	func_map: &'a CssFuncMap,
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

		let properties = selector.tokens().iter().xtry_map(
			|(key, value)| -> Result<String> {
				let key = self.ident_to_css(key)?;

				let value = match value {
					TokenValue::Value(value) => {
						self.func_map.resolve(value, self)?
					}
					TokenValue::Token(token) => {
						self.ident_to_css(&token.key())?.as_css_value()
					}
				};

				Ok(format!("{key}: {value}"))
			},
		)?;


		match self.minify {
			true => {
				format!(
					"{} {{ {} }}",
					rules,
					properties.into_iter().collect::<Vec<_>>().join("; ")
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
		if let Some(ident) = self.ident_map.get(path) {
			return ident.clone().xok();
		}
		use heck::ToKebabCase;
		let path = path.to_string().to_kebab_case().replace("/", "--");
		// TODO hash in prod
		CssIdent::variable(path).xok()
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
}



/// Registers tokens that should represent properties
/// instead of other variables.
#[derive(Default, Deref, Resource)]
pub struct CssProperties(HashMap<TokenKey, CssIdent>);


impl CssProperties {
	pub fn with(mut self, path: TokenKey, ident: CssIdent) -> Self {
		self.0.insert(path, ident);
		self
	}
	pub fn with_property<T: TypedToken>(
		self,
		prop: impl Into<SmolStr>,
	) -> Self {
		self.with(T::key(), CssIdent::Property(prop.into()))
	}
	pub fn with_variable<T: TypedToken>(
		self,
		variable: impl Into<SmolStr>,
	) -> Self {
		self.with(T::key(), CssIdent::Variable(variable.into()))
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

	pub fn as_css_property(&self) -> String { self.to_string() }
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
pub fn default_func_map() -> CssFuncMap {
	CssFuncMap::default()
		.insert::<Color>()
		.insert::<f32>()
		.insert::<Length>()
		.insert::<Typeface>()
		.insert::<StyleFontWeight>()
		.insert::<Duration>()
		.insert::<Shape>()
		.insert::<Elevation>()
		// types with a custom Tokens struct must specify Self as the marker
		.insert::<Typography>()
		.insert::<Motion>()
}

/// Store methods for looking up a schema path and resolving a value
#[derive(Default, Deref, Resource)]
pub struct CssFuncMap(
	HashMap<
		TokenKey,
		Arc<
			dyn 'static
				+ Send
				+ Sync
				+ Fn(&Value, &CssBuilder) -> Result<String>,
		>,
	>,
);
impl CssFuncMap {
	/// Registers a CSS value resolver keyed on `T::Tokens`'s type path.
	///
	/// Stored [`TypedValue`]s carry the schema of their *tokens* struct
	/// (the type actually passed to `with_value`), not the output type,
	/// so the key must match that tokens type.
	pub fn insert<T: Typed + FromReflect + CssValue>(mut self) -> Self {
		self.0.insert(
			TokenKey::of::<T>(),
			Arc::new(|value, builder| {
				value.into_reflect::<T>()?.to_css_value(builder)
			}),
		);
		self
	}

	pub fn resolve(
		&self,
		value: &TypedValue,
		builder: &CssBuilder,
	) -> Result<String> {
		if let Some(func) = self.0.get(value.schema()) {
			func(value.value(), builder)
		} else {
			bevybail!(
				"No CSS function registered for this schema: {:#?}",
				value
			)
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
			.with_state::<(Res<CssProperties>, Res<CssFuncMap>, StyleQuery), _>(
				|entity, state| {
					CssBuilder {
						minify: false,
						ident_map: &state.0,
						func_map: &state.1,
						style_query: &state.2,
					}
					.build(entity)
					.xunwrap()
				},
			);
		println!("{css}");
	}
}
