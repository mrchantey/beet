//! The generic ARIA disclosure pattern for the charcell runtime: a control
//! carrying `aria-controls` toggles `aria-hidden` on the element it references,
//! and styling reacts through attribute selectors (eg
//! `.sidebar:not([aria-hidden="false"])`), so one rule drives both the browser
//! and this runtime. The web twin is the menu-button wiring in
//! `widgets/sidebar.js`; the breakpoint seeding that pairs with it lives in
//! `widgets::sync_sidebar_breakpoint`.
//!
//! Attribute values live on attribute entities (see [`AttributeOf`]), which the
//! cascade's change filters cannot see, so every mutation here also dirties the
//! element's [`ElementStateMap`] — the same convention as
//! `toggle_details_on_click`. Helpers take raw queries rather than
//! [`ElementQuery`]/`AttributeQuery` so callers can hold `&mut Value` without a
//! query conflict.
// the ungated `attr_entity` only needs the beet_core attribute types; the
// crate prelude (Portal, ElementStateMap, ..) serves the tui-gated observer
#[cfg(feature = "tui")]
use crate::prelude::*;
use beet_core::prelude::*;

/// Observer: clicking an element that carries `aria-controls` toggles
/// `aria-hidden` on the element it references by id — the ARIA disclosure
/// pattern, eg the header `MenuButton` collapsing the sidebar rail. The native
/// twin of the menu-button click in `widgets/sidebar.js`, with its exact
/// semantics: the target is hidden when the attribute is `"true"` (absent
/// counts as shown), and a click flips between the two.
///
/// Generic: any BSX control referencing a target id toggles it, no per-widget
/// system required. The id resolves within the clicked tree (up to its render
/// root, then down across [`Portal`] boundaries), so one surface's toggle never
/// reaches another session's tree. Keyboard activation rides along for free:
/// a focused button's Enter/Space synthesizes the same [`PointerUp`].
// registered by `CharcellTuiPlugin`, so gated like it (`attr_entity` below
// stays ungated: `decorate` reads it in every build).
#[cfg(feature = "tui")]
pub(crate) fn toggle_aria_controls_on_click(
	ev: On<PointerUp>,
	parents: Query<&ChildOf>,
	holders: Query<&PortalOf>,
	children: Query<&Children>,
	portals: Query<&Portal>,
	attributes: Query<&Attributes>,
	attr_keys: Query<&Attribute>,
	mut values: Query<&mut Value>,
	mut states: Query<&mut ElementStateMap>,
	mut commands: Commands,
) {
	let control = ev.event_target();
	let Some(id) = attr_string(
		&attributes,
		&attr_keys,
		&values,
		control,
		"aria-controls",
	) else {
		return;
	};
	let root = Portal::render_root(&parents, &holders, control);
	let Some(target) = find_by_id(
		&children, &portals, &attributes, &attr_keys, &values, root, &id,
	) else {
		return;
	};
	// hidden only when explicitly "true", exactly like the script's `isHidden`
	let hidden =
		attr_string(&attributes, &attr_keys, &values, target, "aria-hidden")
			.is_some_and(|value| value == "true");
	set_attr_str(
		&mut commands,
		&mut values,
		&mut states,
		&attributes,
		&attr_keys,
		target,
		"aria-hidden",
		if hidden { "false" } else { "true" },
	);
}

/// The attribute entity with `key` on `entity`, if present.
pub(crate) fn attr_entity(
	attributes: &Query<&Attributes>,
	attr_keys: &Query<&Attribute>,
	entity: Entity,
	key: &str,
) -> Option<Entity> {
	attributes.get(entity).ok().and_then(|attrs| {
		attrs.iter().find(|&attr| {
			attr_keys
				.get(attr)
				.is_ok_and(|attr_key| attr_key.as_str() == key)
		})
	})
}

/// The string value of the `key` attribute on `entity`, if present.
#[cfg(feature = "tui")]
pub(crate) fn attr_string(
	attributes: &Query<&Attributes>,
	attr_keys: &Query<&Attribute>,
	values: &Query<&mut Value>,
	entity: Entity,
	key: &str,
) -> Option<SmolStr> {
	attr_entity(attributes, attr_keys, entity, key)
		.and_then(|attr| values.get(attr).ok())
		.and_then(|value| value.as_str().ok().map(SmolStr::new))
}

/// Set the string attribute `key` on `entity`, upserting the attribute entity,
/// and dirty the element's [`ElementStateMap`] so the cascade re-resolves its
/// subtree the same frame (attribute values live on attribute entities, which
/// the cascade's change filters cannot see).
#[cfg(feature = "tui")]
pub(crate) fn set_attr_str(
	commands: &mut Commands,
	values: &mut Query<&mut Value>,
	states: &mut Query<&mut ElementStateMap>,
	attributes: &Query<&Attributes>,
	attr_keys: &Query<&Attribute>,
	entity: Entity,
	key: &str,
	value: &str,
) {
	match attr_entity(attributes, attr_keys, entity, key) {
		Some(attr) => {
			if let Ok(mut current) = values.get_mut(attr) {
				current.set_if_neq(Value::str(value));
			}
		}
		None => {
			commands.spawn((
				AttributeOf::new(entity),
				Attribute::new(key),
				Value::str(value),
			));
		}
	}
	if let Ok(mut map) = states.get_mut(entity) {
		map.set_changed();
	} else {
		commands.entity(entity).insert(ElementStateMap::default());
	}
}

/// The entity under `root` (inclusive) carrying an `id` attribute equal to
/// `id`, in depth-first order, following [`Portal`] references into transcluded
/// content so a control can reference a target across a transclusion boundary.
#[cfg(feature = "tui")]
pub(crate) fn find_by_id(
	children: &Query<&Children>,
	portals: &Query<&Portal>,
	attributes: &Query<&Attributes>,
	attr_keys: &Query<&Attribute>,
	values: &Query<&mut Value>,
	root: Entity,
	id: &str,
) -> Option<Entity> {
	let mut stack = vec![root];
	while let Some(entity) = stack.pop() {
		if attr_string(attributes, attr_keys, values, entity, "id")
			.is_some_and(|value| value == id)
		{
			return Some(entity);
		}
		if let Ok(kids) = children.get(entity) {
			stack.extend(kids.iter());
		}
		if let Ok(portal) = portals.get(entity) {
			stack.push(portal.target());
		}
	}
	None
}

#[cfg(all(test, feature = "tui"))]
mod tests {
	use super::*;

	/// The string value of `key` on `entity`, via the raw attribute queries.
	fn attr(world: &mut World, entity: Entity, key: &'static str) -> Option<SmolStr> {
		world.with_state::<(
			Query<&Attributes>,
			Query<&Attribute>,
			Query<&mut Value>,
		), _>(move |(attributes, attr_keys, values)| {
			attr_string(&attributes, &attr_keys, &values, entity, key)
		})
	}

	/// The entity of the sole element with `tag`.
	fn tag_entity(world: &mut World, tag: &str) -> Entity {
		let mut query = world.query::<(Entity, &Element)>();
		query
			.iter(world)
			.find(|(_, element)| element.tag() == tag)
			.map(|(entity, _)| entity)
			.unwrap()
	}

	/// A world with the template substrate (which materializes rsx attributes
	/// as attribute entities) and the disclosure observer installed.
	fn world() -> World {
		let mut world = (TemplatePlugin, DocumentPlugin).into_world();
		world.add_observer(toggle_aria_controls_on_click);
		world
	}

	// mirrors sidebar.js's menu-button wiring: the first click on an unseeded
	// target hides it (absent != "true"), the next shows it, and so on.
	#[beet_core::test]
	fn toggles_aria_hidden_on_target() {
		let mut world = world();
		world
			.spawn_template(rsx! {
				<div>
					<button aria-controls="sidebar">"三"</button>
					<nav id="sidebar">"nav"</nav>
				</div>
			})
			.unwrap();
		let (button, nav) =
			(tag_entity(&mut world, "button"), tag_entity(&mut world, "nav"));
		world.entity_mut(button).trigger(PointerUp::new(button));
		world.flush();
		attr(&mut world, nav, "aria-hidden").unwrap().xpect_eq("true");
		world.entity_mut(button).trigger(PointerUp::new(button));
		world.flush();
		attr(&mut world, nav, "aria-hidden").unwrap().xpect_eq("false");
	}

	// the id resolves within the clicked tree only, so a control in one
	// surface's tree never toggles a same-id target in another's.
	#[beet_core::test]
	fn scoped_to_the_clicked_tree() {
		let mut world = world();
		world
			.spawn_template(rsx! {
				<div>
					<button aria-controls="sidebar">"三"</button>
				</div>
			})
			.unwrap();
		world
			.spawn_template(rsx! { <nav id="sidebar">"nav"</nav> })
			.unwrap();
		let (button, nav) =
			(tag_entity(&mut world, "button"), tag_entity(&mut world, "nav"));
		world.entity_mut(button).trigger(PointerUp::new(button));
		attr(&mut world, nav, "aria-hidden").xpect_eq(None);
	}
}
