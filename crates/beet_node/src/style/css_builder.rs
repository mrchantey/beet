#![allow(unused)]
use crate::prelude::*;
use crate::style::*;
use beet_core::prelude::*;

/// Converts a value to its CSS string representation.
pub trait CssValue {
	fn to_css_value(&self) -> String;
}

/// Map a token path to a css key,
/// Multiple tokens may point to the same key,
/// but usually dont when defined in the same crate.
#[derive(Default, Deref, Resource)]
pub struct CssIdentMap(HashMap<FieldPath, CssIdent>);


impl CssIdentMap {
	pub fn with(mut self, path: FieldPath, ident: CssIdent) -> Self {
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


pub struct CssRule {
	selectors: String,
	properties: HashMap<String, String>,
}
pub struct CssBuilder {
	minify: bool,
}


impl CssBuilder {
	pub fn build(
		&self,
		entity: Entity,
		style_query: &StyleQuery,
		document_query: &mut DocumentQuery,
	) -> Result<String> {
		let ident_map = style_query.css_key_map().as_deref();
		let mut get_value = |field: &FieldRef| -> Result<Value> {
			document_query.with_field(entity, &field, |val| val.clone())
		};

		let selectors = style_query
			.collect_selectors(entity)
			.into_iter()
			.xtry_map(|selector| self.build_selector(selector, ident_map))?;

		String::new().xok()
	}

	fn build_selector(
		&self,
		selector: &Selector,
		key_map: Option<&CssIdentMap>,
	) -> Result<String> {
		let rules = self.rules_to_css(&selector.rules());

		let properties = selector.tokens().iter().xtry_map(
			|(key, value)| -> Result<String> {
				let key = self.ident_to_css(key, key_map)?;

				let value = match value {
					ValueOrRef::Value(val) => self.value_to_css(val)?,
					ValueOrRef::Ref(field_ref) => self
						.ident_to_css(&field_ref.field_path, key_map)?
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

	fn value_to_css(&self, value: &Value) -> Result<String> {
		String::default().xok()
	}

	/// Returns the ident in css form, using the [`CssIdentMap`]
	/// if a mapping is found, otherwise the last part of
	/// the field path as a variable.
	/// Non-specified idents are assumed to be variables, not properties.
	fn ident_to_css(
		&self,
		path: &FieldPath,
		key_map: Option<&CssIdentMap>,
	) -> Result<CssIdent> {
		if let Some(ident) = key_map.and_then(|map| map.0.get(path)) {
			return ident.clone().xok();
		}
		let last = path.last().ok_or_else(|| {
			bevyhow!(
				"Path {} is empty and cannot be converted to a CSS key",
				path
			)
		})?;
		use heck::ToKebabCase;
		// TODO full path instead?
		CssIdent::variable(last.to_string().to_kebab_case()).xok()
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

	#[test]
	fn test_name() {}
}
