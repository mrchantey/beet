use std::sync::Arc;

use crate::prelude::*;
use crate::style::*;
use beet_core::prelude::*;

/// Converts a value to its CSS string representation.
pub trait CssValue {
	fn to_css_value(&self) -> String;
}

pub struct CssPlugin;

impl Plugin for CssPlugin {
	fn build(&self, app: &mut App) {
		app.insert_resource(default_func_map())
			.insert_resource(common_props::css_ident_map());
	}
}

pub fn default_func_map() -> CssFuncMap {
	CssFuncMap::default()
		.insert::<Color, _>()
		// types with an actual Tokens impl must specify Self
		.insert::<Typography, Typography>()
		.insert::<Motion, Motion>()
}

/// Store methods for looking up a schema path and resolving a value
#[derive(Default, Deref, Resource)]
pub struct CssFuncMap(
	HashMap<
		TokenPath,
		Arc<
			dyn 'static
				+ Send
				+ Sync
				+ Fn(&Value, Entity, &DocumentQuery) -> Result<String>,
		>,
	>,
);
impl CssFuncMap {
	pub fn insert<T: TypePath + FromTokens<M> + CssValue, M>(mut self) -> Self {
		self.0.insert(
			TokenPath::of::<T>(),
			Arc::new(|value, entity, query| {
				T::from_value(value, entity, query)?.to_css_value().xok()
			}),
		);
		self
	}

	pub fn resolve(
		&self,
		value: &TypedValue,
		entity: Entity,
		query: &DocumentQuery,
	) -> Result<String> {
		if let Some(func) = self.0.get(value.schema()) {
			func(value.value(), entity, query)
		} else {
			bevybail!(
				"No CSS function registered for this schema: {:#?}",
				value
			)
		}
	}
}



/// Map a token path to a css key,
/// Multiple tokens may point to the same key,
/// but usually dont when defined in the same crate.
#[derive(Default, Deref, Resource)]
pub struct CssIdentMap(HashMap<TokenPath, CssIdent>);


impl CssIdentMap {
	pub fn with(mut self, path: TokenPath, ident: CssIdent) -> Self {
		self.0.insert(path, ident);
		self
	}
	pub fn with_property<T: TypedToken>(
		self,
		prop: impl Into<SmolStr>,
	) -> Self {
		self.with(T::path(), CssIdent::Property(prop.into()))
	}
	pub fn with_variable<T: TypedToken>(
		self,
		variable: impl Into<SmolStr>,
	) -> Self {
		self.with(T::path(), CssIdent::Variable(variable.into()))
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
	fn to_css_value(&self) -> String {
		let this = self.to_srgba();
		format!(
			"rgba({}, {}, {}, {})",
			(this.red * 255.0).round() as u8,
			(this.green * 255.0).round() as u8,
			(this.blue * 255.0).round() as u8,
			this.alpha
		)
	}
}
#[derive(Default)]
pub struct CssBuilder {
	minify: bool,
}


impl CssBuilder {
	pub fn build(
		&self,
		entity: Entity,
		ident_map: &CssIdentMap,
		func_map: &CssFuncMap,
		style_query: &StyleQuery,
		document_query: &DocumentQuery,
	) -> Result<String> {
		let selectors = style_query
			.collect_selectors(entity)
			.into_iter()
			.xtry_map(|selector| {
				self.build_selector(
					selector,
					entity,
					document_query,
					func_map,
					ident_map,
				)
			})?;

		match self.minify {
			true => Ok(selectors.join(" ")),
			false => Ok(selectors.join("\n\n")),
		}
	}

	fn build_selector(
		&self,
		selector: &Selector,
		entity: Entity,
		document_query: &DocumentQuery,
		func_map: &CssFuncMap,
		ident_map: &CssIdentMap,
	) -> Result<String> {
		let rules = self.rules_to_css(&selector.rules());

		let properties = selector.tokens().iter().xtry_map(
			|(key, value)| -> Result<String> {
				let key = self.ident_to_css(key, ident_map)?;

				let value = match value {
					ValueOrToken::Value(value) => {
						func_map.resolve(value, entity, document_query)?
					}
					ValueOrToken::Token(token) => self
						.ident_to_css(&token.path(), ident_map)?
						.as_css_value(),
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
					"{} {{\n    {}\n}}",
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
	fn ident_to_css(
		&self,
		path: &TokenPath,
		ident_map: &CssIdentMap,
	) -> Result<CssIdent> {
		if let Some(ident) = ident_map.get(path) {
			return ident.clone().xok();
		}
		use heck::ToKebabCase;
		let path = path.to_string().to_kebab_case().replace("/", "--");
		// TODO full path instead?
		CssIdent::variable(path).xok()
	}

	fn rules_to_css(&self, rules: &[Rule]) -> String {
		rules
			.iter()
			.map(|rule| self.rule_to_css(rule))
			.collect::<Vec<_>>()
			.join(" ")
	}

	fn rule_to_css(&self, rule: &Rule) -> String {
		match rule {
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


#[cfg(test)]
mod tests {
	use super::*;
	use crate::style::material;

	#[test]
	fn test() {
		let mut world =
			(material::MaterialStylePlugin::default(), CssPlugin).into_world();


		let css = world.spawn(rsx! {
			<div class="text-primary">hello world!</div>
		}).with_state::<(Res<CssIdentMap>,
		Res<CssFuncMap>,
		StyleQuery,
		DocumentQuery),_>(|entity,state|{
			CssBuilder::default().build(entity,&state.0,&state.1,&state.2,&state.3).unwrap()
		});
		println!("{css}");
	}
}
