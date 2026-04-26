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
		T: 'static
			+ Send
			+ Sync
			+ FromReflect
			+ Typed
			+ TypedTokenKey
			+ AsCssValues,
	>(
		&self,
		value: &TokenValue,
	) -> Result<Vec<String>> {
		match value {
			TokenValue::Value(value) => {
				value.schema().assert_eq::<T>()?;
				value.value().into_reflect::<T>()?.as_css_values(&self)
			}
			TokenValue::Token(token) => token.as_css_values(&self),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
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
			SelectorStore::default()
				.with(selectors::hero_heading())
				.with(
					Selector::root()
						.with_token::<colors::OnPrimary, tones::Primary20>(),
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
	#[test]
	fn test_color_role() {
		let mut world = World::new();

		world.insert_resource(
			CssTokenMap::default()
				// .insert(colors::Primary)
				// .insert(colors::OnPrimary)
				// .insert(tones::Primary80)
				// .insert(tones::Primary20)
				.insert(colors::PrimaryRole)
				.insert(ColorRoleProps),
			// .insert(common_props::ForegroundColor),
		);

		world.insert_resource(
			SelectorStore::default().with(
				Selector::new()
					.with_rule(Rule::class("primary-role"))
					.with_token::<style::ColorRoleProps, colors::PrimaryRole>(),
			), // .with(
			   // 	Selector::root()
			   // 		.with_typed::<colors::Primary, tones::Primary80>()
			   // 		.with_typed::<colors::OnPrimary, tones::Primary20>(),
			   // )
			   // .with(
			   // 	Selector::root()
			   // 		.with_value::<tones::Primary80>(Color::srgb(
			   // 			0., 0.8, 0.,
			   // 		))
			   // 		.unwrap()
			   // 		.with_value::<tones::Primary20>(Color::srgb(
			   // 			0., 0.2, 0.,
			   // 		))
			   // 		.unwrap(),
			   // ),
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
