use crate::as_beet::*;
use bevy::prelude::*;

pub fn extract_rsx_directives_plugin(app: &mut App) {
	app.add_plugins(extract_directive_plugin::<SlotChild>)
		.add_systems(
			Update,
			slot_target_directive.in_set(ExtractDirectivesSet),
		);
}

/// Directive indicating a node should be moved to the slot with the given name.
/// All nodes without this directive are moved to the default slot.
#[derive(Debug, Default, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub enum SlotChild {
	/// Default slot indicated by the `slot="default"`, or `slot` attribute without a value.
	#[default]
	Default,
	/// Named slot indicated by the `slot="name"` attribute.
	Named(String),
}

impl TemplateDirective for SlotChild {
	fn try_from_attribute(
		key: &str,
		value: Option<&AttributeLit>,
	) -> Option<Self> {
		match (key, value) {
			("slot", Some(AttributeLit::String(value)))
				if value.as_str() == "default" =>
			{
				Some(Self::Default)
			}
			("slot", Some(AttributeLit::String(value))) => {
				Some(Self::Named(value.to_string()))
			}
			("slot", None) => Some(Self::Default),
			_ => None,
		}
	}
}



/// The target for slots, defined with a tag `<slot>` or <slot name="foo">`.
/// This directive is unique as its defined by the tag name, not an attribute.
#[derive(Debug, Default, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component)]
#[require(FragmentNode)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub enum SlotTarget {
	#[default]
	Default,
	Named(String),
}

impl SlotTarget {
	/// Returns the name of the slot target, or `None` if it's the default slot.
	pub fn name(&self) -> Option<&str> {
		match self {
			SlotTarget::Default => None,
			SlotTarget::Named(name) => Some(name),
		}
	}
}


// convert all nodes with the `slot` tag into a `SlotTarget`
fn slot_target_directive(
	mut commands: Commands,
	attributes: Query<&Attributes>,
	query: Populated<(Entity, &NodeTag), With<ElementNode>>,
	attributes_query: Query<(Entity, &AttributeKey, Option<&AttributeLit>)>,
) {
	for (node_ent, node_tag) in query.iter() {
		if **node_tag != "slot" {
			continue;
		}
		let target = attributes
			.iter_descendants(node_ent)
			.filter_map(|a| attributes_query.get(a).ok())
			.find(|(_, key, _)| ***key == "name")
			.map(|(entity, _, value)| {
				commands.entity(entity).despawn();
				if let Some(AttributeLit::String(value)) = value.as_ref() {
					SlotTarget::Named(value.clone())
				} else {
					SlotTarget::Default
				}
			})
			.unwrap_or(SlotTarget::Default);

		commands
			.entity(node_ent)
			.remove::<NodeTag>()
			.remove::<ElementNode>()
			// requires fragment
			.insert(target);
	}
}



#[cfg(test)]
mod test {
	use super::*;
	use crate::prelude::*;
	use bevy::ecs::system::RunSystemOnce;
	use sweet::prelude::*;

	#[test]
	fn default_slot_target() {
		let mut app = App::new();
		let entity = app
			.world_mut()
			.spawn((ElementNode::self_closing(), NodeTag("slot".to_string())))
			.id();
		app.world_mut()
			.run_system_once(slot_target_directive)
			.unwrap();

		app.world_mut()
			.entity(entity)
			.get::<SlotTarget>()
			.xpect()
			.to_be(Some(&SlotTarget::default()));
	}
	#[test]
	fn named_slot_target() {
		let mut app = App::new();
		let entity = app
			.world_mut()
			.spawn((
				ElementNode::self_closing(),
				NodeTag("slot".to_string()),
				related!(
					Attributes[(
						AttributeKey::new("name"),
						"foo".into_attribute_bundle()
					)]
				),
			))
			.id();
		app.world_mut()
			.run_system_once(slot_target_directive)
			.unwrap();

		app.world_mut()
			.entity(entity)
			.get::<SlotTarget>()
			.xpect()
			.to_be(Some(&SlotTarget::Named("foo".to_string())));
	}
}
