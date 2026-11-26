use crate::prelude::*;
use crate::style::*;
use beet_core::prelude::*;

#[derive(Default)]
pub struct CssPlugin;

impl Plugin for CssPlugin {
	fn build(&self, app: &mut App) {
		app.world_mut()
			.get_resource_or_init::<CssTokenMap>()
			.extend(common_props::token_map());
		// a one-off rule a reader computed, carried through a scene body
		app.register_type::<InlineRule>();
		// markup, and what it writes of a style: the `<Rule>` tag declares a
		// named rule into the `RuleSet` at build time, the no-code analogue of a
		// typed `Rule` in Rust, and `bx:style` rewrites to an `InlineStyle`, a
		// one-off rule and its inline class, that of the `inline_class!` macro.
		#[cfg(feature = "scene")]
		app.install_types::<beet_scene::prelude::ScenePlugin>()
			.register_type::<DeclaredRule>()
			.register_type::<InlineStyle>();
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
	/// The stylesheet of every rule in `rule_set`, its `:root` default first.
	pub fn build(
		&self,
		css_map: &CssTokenMap,
		rule_set: &RuleSet,
	) -> Result<String> {
		self.build_where(css_map, rule_set, |_| true)
	}

	/// [`build`](Self::build) emitting only the rules `keep` admits. Every rule
	/// is still formatted, so variable names resolve exactly as the whole set's
	/// do and a fragment's sheet names the variables its page's sheet declares.
	/// A rule `keep` refuses that has no CSS form is skipped rather than failing
	/// the build: the whole sheet would have failed on it, so there is no page
	/// naming to match.
	pub fn build_where(
		&self,
		css_map: &CssTokenMap,
		rule_set: &RuleSet,
		keep: impl Fn(&Rule) -> bool,
	) -> Result<String> {
		let (kept, css_rules): (Vec<bool>, Vec<CssRule>) = rule_set
			.iter()
			// rules gated on a query with no CSS condition (`Terminal`) never reach
			// the serialized stylesheet (the charcell cascade applies them instead),
			// nor do entity rules, which have no CSS form and are resolved at
			// runtime against the entity itself.
			.filter(|rule| {
				rule.media().is_none_or(|media| media.as_css().is_some())
					&& !rule.selector().has_entity()
			})
			.filter_map(|rule| {
				let kept = keep(rule);
				match CssRule::from_rule(css_map, rule) {
					Ok(css) => Some(Ok((kept, css))),
					Err(err) if kept => Some(Err(err)),
					Err(_) => None,
				}
			})
			.collect::<Result<Vec<_>>>()?
			.into_iter()
			.unzip();

		// iteration variables
		let mut declared = HashMap::default();
		let mut format_variables = self.format_variables;

		for i in 0..self.max_iterations {
			match css_rules.iter().xtry_map(|rule| {
				self.format_rule(format_variables, &mut declared, rule)
			}) {
				Ok(formatted) => {
					let formatted = formatted
						.into_iter()
						.zip(&kept)
						.filter_map(|(rule, kept)| kept.then_some(rule))
						.collect::<Vec<_>>();
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
		css_rule: &CssRule,
	) -> Result<String, CollisionFound> {
		let selector = css_rule.selector_to_css();
		// a direct look outranks every author rule, as a `style` attribute does
		let important = match css_rule.origin() {
			RuleOrigin::Direct => " !important",
			_ => "",
		};
		let mut declarations =
			css_rule.declarations().iter().xtry_map(|(key, value)| {
				Self::format_declaration(format_variables, declared, key, value)
					.map(|declaration| match important {
						"" => declaration,
						important => declaration
							.strip_suffix(';')
							.map(|declaration| {
								format!("{declaration}{important};")
							})
							.unwrap_or(declaration),
					})
			})?;

		declarations.sort();

		let rule = if self.minify {
			format!("{} {{ {} }}", selector, declarations.join(" "))
		} else {
			format!(
				"{} {{\n{}\n}}",
				selector,
				declarations
					.into_iter()
					.map(|dec| format!("  {dec}"))
					.collect::<Vec<_>>()
					.join("\n")
			)
		};

		// gate behind an `@media (…)` at-rule when the rule carries a media query
		// with a CSS equivalent (`Terminal` has none and is filtered out before
		// serialization, so `as_css` returning `None` falls through to the bare rule).
		let rule = match css_rule.media().and_then(|media| media.as_css()) {
			None => rule,
			Some(media) => self.wrap(&format!("@media {media}"), rule),
		};
		// a user agent default sits in a layer, which every unlayered author
		// rule outranks whatever its specificity, as a browser's own does
		match css_rule.origin() {
			RuleOrigin::UserAgent => self.wrap("@layer user-agent", rule),
			_ => rule,
		}
		.xok()
	}

	/// `rule` inside the block of the at-rule `prelude`, ie `@media print`.
	fn wrap(&self, prelude: &str, rule: String) -> String {
		match self.minify {
			true => format!("{prelude}{{{rule}}}"),
			false => {
				let indented = rule
					.lines()
					.map(|line| format!("  {line}"))
					.collect::<Vec<_>>()
					.join("\n");
				format!("{prelude} {{\n{indented}\n}}")
			}
		}
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

	#[beet_core::test]
	fn test_color() {
		let mut world = World::new();

		world.insert_resource(
			CssTokenMap::default()
				.insert(colors::OnPrimary)
				.insert(tones::Primary20)
				.insert(common_props::ForegroundColor),
		);

		world.insert_resource(
			RuleSet::default()
				.with_token(colors::OnPrimary, tones::Primary20)
				.unwrap()
				.with_value(tones::Primary20, Color::srgb(0., 1., 0.))
				.unwrap()
				.with_rule(
					Rule::new()
						.with_selector(Selector::class("color-primary"))
						.with_token(
							common_props::ForegroundColor,
							colors::OnPrimary,
						)
						.unwrap(),
				),
		);
		let _css = world
			.with_state::<StyleQuery, _>(|query| {
				query.build_css(&test_builder())
			})
			.xunwrap()
			.xpect_snapshot();
		// println!("{_css}");
	}
	#[beet_core::test]
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
			RuleSet::default()
				.with_token(colors::Primary, tones::Primary80)
				.unwrap()
				.with_token(colors::OnPrimary, tones::Primary20)
				.unwrap()
				.with_value(colors::PrimaryRole, ColorRole {
					background: colors::Primary.into(),
					foreground: colors::OnPrimary.into(),
				})
				.unwrap()
				.with_value(tones::Primary80, Color::srgb(0., 0.8, 0.))
				.unwrap()
				.with_value(tones::Primary20, Color::srgb(0., 0.2, 0.))
				.unwrap()
				.with_rule(
					Rule::new()
						.with_selector(Selector::class("primary-role"))
						.with_token(ColorRoleProps, colors::PrimaryRole)
						.unwrap(),
				),
		);
		world
			.with_state::<StyleQuery, _>(|query| {
				query.build_css(&test_builder())
			})
			.xunwrap()
			.xpect_snapshot();
	}

	/// A user agent rule serializes inside `@layer user-agent`, its `@media`
	/// within it, and a direct rule's declarations are `!important`.
	#[beet_core::test]
	fn layers_and_marks_origins() {
		let mut world = World::new();
		world.insert_resource(
			CssTokenMap::default().insert(common_props::ForegroundColor),
		);
		let color = |rule: Rule| {
			rule.with_value(common_props::ForegroundColor, Color::BLACK)
		};
		world.insert_resource(
			RuleSet::default().with_rules([
				color(Rule::tag("p").with_origin(RuleOrigin::UserAgent)),
				color(
					Rule::tag("em")
						.with_origin(RuleOrigin::UserAgent)
						.with_media(MediaQuery::Print),
				),
				color(Rule::class("look").with_origin(RuleOrigin::Direct)),
				color(Rule::class("theme")),
			]),
		);
		world
			.with_state::<StyleQuery, _>(|query| {
				query.build_css(&test_builder())
			})
			.unwrap()
			.xpect_contains("@layer user-agent {\n  p {\n    color: ")
			.xpect_contains("@layer user-agent {\n  @media print {\n    em {\n")
			.xpect_contains(" !important;\n}")
			.xpect_contains(".theme {\n  color: ")
			.xnot()
			.xpect_contains("@layer user-agent {\n  .theme");
	}

	/// A fragment's sheet carries only the rules selecting its classes, each
	/// formatted exactly as the whole sheet formats it, and skips a rule
	/// outside it with no CSS form, which fails the whole sheet.
	#[beet_core::test]
	fn selecting_matches_the_whole_sheet() {
		let mut world = World::new();
		world.insert_resource(
			CssTokenMap::default()
				.insert(colors::OnPrimary)
				.insert(tones::Primary20)
				.insert(common_props::ForegroundColor),
		);
		let rule = |class: &str| {
			Rule::new()
				.with_selector(Selector::class(class))
				.with_token(common_props::ForegroundColor, colors::OnPrimary)
				.unwrap()
		};
		world.insert_resource(
			RuleSet::default()
				.with_token(colors::OnPrimary, tones::Primary20)
				.unwrap()
				.with_value(tones::Primary20, Color::srgb(0., 1., 0.))
				.unwrap()
				.with_rule(rule("kept"))
				.with_rule(rule("other")),
		);
		let whole = |world: &mut World| {
			world.with_state::<StyleQuery, _>(|query| {
				query.build_css(&test_builder())
			})
		};
		let fragment = |world: &mut World| {
			world
				.with_state::<StyleQuery, _>(|query| {
					query.build_css_selecting(
						&test_builder(),
						&HashSet::from_iter([SmolStr::new("kept")]),
					)
				})
				.unwrap()
		};
		let kept = fragment(&mut world);
		kept.xref()
			.xpect_contains(".kept")
			.xnot()
			.xpect_contains(".other")
			.xnot()
			.xpect_contains(":root");
		whole(&mut world).unwrap().xpect_contains(kept.trim());
		world.resource_mut::<RuleSet>().insert_rule(
			Rule::new()
				.with_selector(Selector::class("broken"))
				.with_token(ColorRoleProps, colors::PrimaryRole)
				.unwrap(),
		);
		whole(&mut world).is_err().xpect_true();
		fragment(&mut world).xpect_eq(kept);
	}
}
