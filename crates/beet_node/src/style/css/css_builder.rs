use crate::prelude::*;
use crate::style::*;
use beet_core::prelude::*;
use bevy::reflect::Typed;


#[derive(Default)]
pub struct CssPlugin;

impl Plugin for CssPlugin {
	fn build(&self, app: &mut App) {
		app.world_mut()
			.get_resource_or_init::<CssTokenMap>()
			.extend(common_props::token_map());
	}
}

#[derive(Get, SetWith)]
pub struct CssBuilder {
	minify: bool,
	max_iterations: usize,
	format_variables: FormatVariables,
}
impl Default for CssBuilder {
	fn default() -> Self {
		cfg_if! {
			if #[cfg(debug_assertions)] {
				Self {
					minify: false,
					max_iterations: 8,
					format_variables: FormatVariables::Full,
				}
			} else {
				Self {
					minify: true,
					max_iterations: 8,
					format_variables: FormatVariables::Hash { min_len: 4 },
				}
			}
		}
	}
}

impl CssBuilder {
	pub fn build(
		&self,
		css_map: &CssTokenMap,
		tokens: &[(&TokenKey, &TokenValue)],
	) -> Result<String> {
		let css_rules = tokens
			.xtry_map(|(key, value)| CssToken::resolve(&css_map, key, value))?;

		// iteration variables
		let mut declared = HashMap::default();
		let mut format_variables = self.format_variables;

		for i in 0..self.max_iterations {
			match css_rules.iter().xtry_map(|rule| {
				self.format_rule(format_variables, &mut declared, rule)
			}) {
				Ok(formatted) => {
					return if self.minify {
						formatted.join("")
					} else {
						String::from("\n").xtend(formatted.join("\n\n"))
					}
					.xok();
				}
				Err(CollisionFound {
					original,
					formatted,
				}) => {
					warn!(
						"Collision found:\n variable: `{original}`\nformated: `{formatted}`\nFormat rules:\n{format_variables:?}\niteration:{i}/{}",
						self.max_iterations
					);
					format_variables = format_variables.increment_widening();
					declared.clear();
				}
			}
		}
		bevybail!(
			"Max iterations reached, unable to resolve variable name collisions\nformatting: {:?}\nmax iterations: {}",
			self.format_variables,
			self.max_iterations
		)
	}
	fn format_rule(
		&self,
		format_variables: FormatVariables,
		declared: &mut HashMap<CssVariable, CssVariable>,
		css_token: &CssToken,
	) -> Result<String, CollisionFound> {
		let predicate = css_token.predicate_to_css();
		let mut declarations =
			css_token.declarations.iter().xtry_map(|(key, value)| {
				Self::format_declaration(format_variables, declared, key, value)
			})?;

		declarations.sort();

		if self.minify {
			format!("{} {{ {} }}", predicate, declarations.join(" "))
		} else {
			format!(
				"{} {{\n{}\n}}",
				predicate,
				declarations
					.into_iter()
					.map(|dec| format!("  {dec}"))
					.collect::<Vec<_>>()
					.join("\n")
			)
		}
		.xok()
	}

	fn format_declaration(
		format_variables: FormatVariables,
		declared: &mut HashMap<CssVariable, CssVariable>,
		key: &CssKey,
		value: &CssValue,
	) -> Result<String, CollisionFound> {
		let key = match key {
			CssKey::Variable(var) => {
				Self::format_var(format_variables, declared, var)?.as_css_key()
			}
			CssKey::Property(prop) => prop.to_string(),
		};
		let value = match value {
			CssValue::Variable(var) => {
				Self::format_var(format_variables, declared, var)?
					.as_css_value()
			}
			CssValue::Expression(expr) => expr.clone(),
		};

		format!("{}: {};", key, value).xok()
	}

	fn format_var(
		format_variables: FormatVariables,
		declared: &mut HashMap<CssVariable, CssVariable>,
		original: &CssVariable,
	) -> Result<CssVariable, CollisionFound> {
		let formatted = format_variables.format(original);
		if let Some(prev_original) = declared.get(&formatted) {
			if prev_original != original {
				// collision found
				return Err(CollisionFound {
					original: original.clone(),
					formatted,
				});
			} else {
				// already declared with the same original, return formatted
				return Ok(formatted);
			}
		} else {
			declared.insert(formatted.clone(), original.clone());
			return Ok(formatted);
		}
	}
}


struct CollisionFound {
	original: CssVariable,
	formatted: CssVariable,
}


#[derive(Debug, Default, Copy, Clone)]
pub enum FormatVariables {
	#[default]
	Full,
	/// Splits the name by dashes, removing the first
	/// n parts
	Short {
		/// The number of parts to remove
		skip_parts: usize,
	},
	Hash {
		/// Specify the minimum hash length,
		/// this will be extended in the case of collisions
		min_len: usize,
	},
}


impl FormatVariables {
	/// Formats variables shortening the full name by five parts:
	/// - `--io-crates-beet-node-style-material-colors-primary`
	/// becomes
	/// - `--material-colors-primary`
	pub fn short() -> Self { Self::Short { skip_parts: 5 } }

	/// Widens the formatting rules,
	/// - Hash: increment min_len
	/// - Short: decrement skip_parts
	/// - Full: remains the same
	fn increment_widening(&self) -> Self {
		match self {
			Self::Full => Self::Full,
			Self::Short { skip_parts } => Self::Short {
				skip_parts: skip_parts.saturating_sub(1),
			},
			Self::Hash { min_len } => Self::Hash {
				min_len: *min_len + 1,
			},
		}
	}

	fn format(&self, name: &CssVariable) -> CssVariable {
		match self {
			Self::Full => name.clone(),
			Self::Short { skip_parts } => name
				.0
				.split('-')
				.skip(*skip_parts)
				.collect::<Vec<_>>()
				.join("-")
				.xmap(CssVariable),
			Self::Hash { min_len } => {
				use std::hash::Hash;
				use std::hash::Hasher;
				let mut hasher = FixedHasher::default().build_hasher();
				name.hash(&mut hasher);
				let hash = hasher.finish();
				format!("{:x}", hash)[..(*min_len).min(16)]
					.to_string()
					.xmap(CssVariable)
			}
		}
	}
}


#[derive(Default, Get, SetWith)]
pub struct CssToken {
	predicate: Predicate,
	declarations: HashMap<CssKey, CssValue>,
}

impl CssToken {
	pub fn from_rule(token_map: &CssTokenMap, rule: &Rule) -> Result<Self> {
		let mut this = Self::default().with_predicate(rule.predicate().clone());

		for (key, value) in rule.declarations().iter() {
			let css_rule = Self::resolve(token_map, key, value)?;
			this.merge_any(css_rule);
		}
		this.xok()
	}

	pub fn merge_any(&mut self, other: CssToken) {
		self.predicate = self.predicate.clone().merge_any(other.predicate);
		self.declarations.extend(other.declarations);
	}

	pub fn resolve(
		css_map: &CssTokenMap,
		key: &TokenKey,
		value: &TokenValue,
	) -> Result<Self> {
		css_map.get(key)?.as_css_token(&value)
	}

	/// Used in CssToken declarations section.
	/// The value type will be checked for multiple properties, and
	/// appended to the key ident if found.
	pub fn from_key_value<
		V: 'static
			+ Send
			+ Sync
			+ serde::de::DeserializeOwned
			+ Typed
			+ TypedTokenKey
			+ AsCssValues,
	>(
		key: &TokenKey,
		value: &TokenValue,
	) -> Result<Self> {
		let key = CssVariable::from_token_key(&key);
		let values = CssValue::from_token_value::<V>(value)?;
		let suffixes = V::suffixes();
		let declarations = if suffixes.len() <= 1 {
			// no need for suffix for zero or one props
			key.xinto::<CssKey>().xvec()
		} else {
			if suffixes.len() != values.len() {
				bevybail!(
					"Property count mismatch:\nkeys: {suffixes:#?}\nvalues:{values:#?}",
				);
			}
			suffixes
				.into_iter()
				.map(|suffix| {
					key.with_suffix(suffix.to_string()).xinto::<CssKey>()
				})
				.collect::<Vec<_>>()
		};
		Self::default()
			.with_declarations(declarations.into_iter().zip(values).collect())
			.xok()
	}
	pub fn from_props_value<
		V: 'static
			+ Send
			+ Sync
			+ serde::de::DeserializeOwned
			+ Typed
			+ TypedTokenKey
			+ AsCssValues,
	>(
		keys: Vec<CssKey>,
		value: &TokenValue,
	) -> Result<Self> {
		let values = CssValue::from_token_value::<V>(value)?;
		if keys.len() != values.len() {
			bevybail!(
				"Property count mismatch:\nkeys: {keys:#?}\nvalues:{values:#?}",
			);
		}
		Self::default()
			.with_declarations(keys.into_iter().zip(values).collect())
			.xok()
	}

	pub fn predicate_to_css(&self) -> String {
		Self::predicate_to_css_inner(&self.predicate)
	}

	fn predicate_to_css_inner(rule: &Predicate) -> String {
		match rule {
			Predicate::Any => "*".to_string(),
			Predicate::Root => ":root".to_string(),
			Predicate::AnyOf(rules) => rules
				.iter()
				.map(|rule| Self::predicate_to_css_inner(rule))
				.collect::<Vec<_>>()
				.join(", "),
			Predicate::AllOf(_rules) => {
				unimplemented!("how to do this properly?")
			}
			Predicate::Tag(tag) => tag.to_string(),
			Predicate::Class(class) => format!(".{}", class),
			Predicate::State(ElementState::Hovered) => ":hover".to_string(),
			Predicate::State(ElementState::Focused) => ":focus".to_string(),
			Predicate::State(ElementState::Pressed) => ":active".to_string(),
			Predicate::State(ElementState::Selected) => {
				"[aria-selected=\"true\"]".to_string()
			}
			Predicate::State(ElementState::Dragged) => {
				"[data-dragging=\"true\"]".to_string()
			}
			Predicate::State(ElementState::Disabled) => ":disabled".to_string(),
			Predicate::State(ElementState::Custom(val)) => {
				// TODO needs design work
				format!("[data-state-{}]", val)
			}
			Predicate::Attribute { key, value } => match value {
				Some(value) => format!("[{}=\"{}\"]", key, value),
				None => format!("[{}]", key),
			},
			Predicate::Not(inner) => {
				format!(":not({})", Self::predicate_to_css_inner(inner))
			}
		}
	}
}

/// The right hand side of a css declaration
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
			+ serde::de::DeserializeOwned
			+ Typed
			+ TypedTokenKey
			+ AsCssValues,
	>(
		value: &TokenValue,
	) -> Result<Vec<Self>> {
		value.schema().assert_eq_ty::<V>()?;
		match value {
			TokenValue::Value(value) => {
				value.value().clone().into_serde::<V>()?.as_css_values()
			}
			TokenValue::Token(token) => Self::from_token::<V>(token).xok(),
		}
	}
	/// Represent tokens as css values, appending the property names in
	/// the case there are multiple
	pub fn from_token<T: AsCssValues>(token: &Token) -> Vec<Self> {
		let var = CssVariable::from_token_key(token.key());
		let suffixes = T::suffixes();
		if suffixes.len() <= 1 {
			// no need for suffix for no declared props
			var.xinto::<Self>().xvec()
		} else {
			suffixes
				.into_iter()
				.map(|suffix| {
					var.with_suffix(suffix.to_string()).xinto::<Self>()
				})
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
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

	fn test_builder() -> CssBuilder {
		CssBuilder {
			minify: false,
			max_iterations: 8,
			format_variables: FormatVariables::Full,
		}
	}
	#[test]
	fn mismatch() {
		todo!("make it nice to define rule tokens inline");
		World::new()
			.with_resource(
				CssTokenMap::default().insert(common_props::ForegroundColor),
			)
			.with_resource(
				RuleStore::default().with(
					Rule::new()
						.with_predicate(Predicate::class("color-primary"))
						.with_token::<common_props::ForegroundColor>(
							colors::PrimaryRole,
						),
				),
			)
			.spawn_empty()
			.with_state::<StyleQuery, _>(|entity, query| {
				query.build_css(&test_builder(), entity)
			})
			.unwrap_err()
			.to_string()
			.xpect_contains("Token Key Mismatch");
	}
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
				.with(
					Rule::new()
						.with_predicate(Predicate::class("color-primary"))
						.with_token::<common_props::ForegroundColor>(
							colors::OnPrimary,
						),
				)
				.with(
					Rule::new()
						.with_token::<colors::OnPrimary>(tones::Primary20),
				)
				.with(
					Rule::new()
						.with_value::<tones::Primary20>(Color::srgb(0., 1., 0.))
						.unwrap(),
				),
		);
		let css = world
			.spawn(rsx! {
				<div class="text-primary">hello world!</div>
			})
			.with_state::<StyleQuery, _>(|entity, query| {
				query.build_css(&test_builder(), entity)
			})
			.xunwrap();
		// println!("{css}");
		css
			.xpect_contains(".color-primary")
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
				.insert(ColorRoleProps),
		);

		world.insert_resource(
			RuleStore::default()
				.with(
					Rule::new()
						.with_predicate(Predicate::class("primary-role"))
						.with_token::<ColorRoleProps>(colors::PrimaryRole),
				)
				.with(
					Rule::new()
						.with_token::<colors::Primary>(tones::Primary80)
						.with_token::<colors::OnPrimary>(tones::Primary20)
						.with_value::<colors::PrimaryRole>(ColorRole {
							background: colors::Primary.into(),
							foreground: colors::OnPrimary.into(),
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
			.with_state::<StyleQuery, _>(|entity, query| {
				query.build_css(&test_builder(), entity)
			})
			.xunwrap();
		// println!("{css}");
		css
			.xpect_contains(".primary-role")
			.xpect_contains("background-color: var(--io-crates-beet-node-style-material-colors-primary-role-bg)")
			.xpect_contains("color: var(--io-crates-beet-node-style-material-colors-primary-role-fg)")
			.xpect_contains(":root")
			.xpect_contains("--io-crates-beet-node-style-material-colors-primary: var(--io-crates-beet-node-style-material-tones-primary80)")
			.xpect_contains("--io-crates-beet-node-style-material-tones-primary20: rgb(0, 51, 0)")
			.xpect_contains("--io-crates-beet-node-style-material-colors-primary-role-bg: var(--io-crates-beet-node-style-material-colors-primary)")
			.xpect_contains("--io-crates-beet-node-style-material-colors-primary-role-fg: var(--io-crates-beet-node-style-material-colors-on-primary)")
			.xpect_contains("--io-crates-beet-node-style-material-tones-primary80: rgb(0, 204, 0)")
			.xpect_contains("--io-crates-beet-node-style-material-colors-on-primary: var(--io-crates-beet-node-style-material-tones-primary20)");
	}
}
