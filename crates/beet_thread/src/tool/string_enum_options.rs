//! [`StringEnumOptions`]: constrain a tool input field to a runtime set of string
//! options, patching the entity's [`ToolDefinition`] schema so the model may only
//! pick one. The options are data (a blob-store listing, config, anything), not a
//! fixed Rust enum, so any action can offer choices computed at runtime.
use crate::prelude::*;
use beet_core::prelude::*;

/// Constrains one string field of this entity's tool schema to `options`, so the
/// model must pick one of them. Spawn it beside an action route (which carries the
/// [`ToolDefinition`]); the schema stays in sync as the options change or the
/// definition is rebuilt.
///
/// Empty `options` leaves the field an unconstrained string (an empty `enum` is not
/// valid strict-mode schema), so it is safe to author with no options and fill them
/// in later. See [`Schema::set_field_enum`].
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
pub struct StringEnumOptions {
	/// The input field to constrain, ie `"image"`.
	pub field: SmolStr,
	/// The allowed values the model may pick from.
	pub options: Vec<SmolStr>,
}

impl StringEnumOptions {
	/// Constrain `field` to `options`.
	pub fn new(
		field: impl Into<SmolStr>,
		options: impl IntoIterator<Item = impl Into<SmolStr>>,
	) -> Self {
		Self {
			field: field.into(),
			options: options.into_iter().map(Into::into).collect(),
		}
	}

	/// Inject the options as an `enum` on the definition's schema field, unless
	/// empty (an empty `enum` is invalid), leaving the field unconstrained.
	fn apply(&self, def: &mut ToolDefinition) {
		if self.options.is_empty() {
			return;
		}
		if let ToolDefinition::Function(func) = def {
			func.params_schema_mut()
				.set_field_enum(&self.field, self.options.iter().cloned());
		}
	}
}

/// Re-patch the tool schema whenever the options are inserted or changed.
pub(crate) fn sync_string_enum_options(
	mut tools: Query<
		(&StringEnumOptions, &mut ToolDefinition),
		Changed<StringEnumOptions>,
	>,
) {
	for (options, mut def) in &mut tools {
		options.apply(&mut def);
	}
}

/// Re-apply the options when the [`ToolDefinition`] is (re)built from the static
/// type (eg `insert_tool_definition`), so a scene reload does not drop the enum.
pub(crate) fn reapply_string_enum_options(
	ev: On<Insert, ToolDefinition>,
	mut tools: Query<(&StringEnumOptions, &mut ToolDefinition)>,
) {
	if let Ok((options, mut def)) = tools.get_mut(ev.entity) {
		options.apply(&mut def);
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// A tool route's schema gains, then updates, an `enum` constraint as the
	/// [`StringEnumOptions`] beside it change.
	#[beet_core::test]
	fn patches_tool_schema() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, ThreadPlugin::default()));
		// a minimal function tool + options constraining its `image` field.
		let tool = app
			.world_mut()
			.spawn((
				ToolDefinition::function(
					"show",
					"show an image",
					Schema::from_value(image_object_schema()),
				),
				StringEnumOptions::new("image", ["happy", "sad"]),
			))
			.id();
		app.update();
		field_enum(&app, tool).xpect_eq(Some(vec![
			SmolStr::new("happy"),
			SmolStr::new("sad"),
		]));

		// changing the options re-patches the schema.
		app.world_mut()
			.get_mut::<StringEnumOptions>(tool)
			.unwrap()
			.options = vec![SmolStr::new("car")];
		app.update();
		field_enum(&app, tool).xpect_eq(Some(vec![SmolStr::new("car")]));
	}

	/// A bare object schema with a single string `image` property.
	fn image_object_schema() -> Value {
		let mut image = Map::default();
		image.insert("type", "string");
		let mut props = Map::default();
		props.insert("image", Value::Map(image));
		let mut root = Map::default();
		root.insert("type", "object");
		root.insert("properties", Value::Map(props));
		Value::Map(root)
	}

	/// The `enum` values on the tool's `image` field, if any.
	fn field_enum(app: &App, tool: Entity) -> Option<Vec<SmolStr>> {
		let ToolDefinition::Function(func) =
			app.world().get::<ToolDefinition>(tool)?
		else {
			return None;
		};
		func.params_schema()
			.get("properties")
			.and_then(|props| props.get("image"))
			.and_then(|field| field.get("enum"))
			.and_then(|values| values.as_list().ok())
			.map(|values| {
				values
					.iter()
					.filter_map(|value| value.as_str().ok())
					.map(SmolStr::new)
					.collect()
			})
	}
}
