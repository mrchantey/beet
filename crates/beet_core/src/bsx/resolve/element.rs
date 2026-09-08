//! Element building: the tag dispatch, the neutral content every element
//! carries (directives, attributes, children), and the slot machinery.

use super::binding::*;
use super::directives::*;
use super::entity_refs::*;
use super::events::*;
use super::literal::*;
use super::resolve::*;
use super::spread::*;
use super::uppercase::*;
use crate::prelude::*;
use bevy::ecs::template::TemplateContext;

/// Build an element: dispatch on its tag kind, then directives, attributes, and
/// children.
pub(super) fn build_element(
	el: &BsxElement,
	registry: &BsxTemplateRegistry,
	refs: &RefBindings,
	cx: &mut TemplateContext,
) -> Result<()> {
	if el.tag == "Slot" {
		return build_slot(el, cx.entity);
	}
	// a `<Fragment>` is a neutral host: it carries spreads, directives and slot
	// children but inserts no `Element`, so a bundle can wrap content without
	// emitting a DOM node (the markup twin of returning `impl Bundle`).
	if el.tag == "Fragment" {
		return build_fragment(el, registry, refs, cx);
	}
	if is_uppercase_tag(&el.tag) {
		return build_uppercase(el, registry, refs, cx);
	}
	build_html_element(el, registry, refs, cx)
}

/// Whether a tag resolves by name (a component or template) rather than as an
/// HTML element: a capitalized tag, or a `path::to::X` module path whose final
/// segment is capitalized (a `<path::to::X>` BSX template).
fn is_uppercase_tag(tag: &str) -> bool {
	tag.rsplit("::")
		.next()
		.unwrap_or(tag)
		.starts_with(|ch: char| ch.is_uppercase())
}

/// Build a lowercase HTML element: an `Element` host plus the neutral
/// [`build_fragment`] content (directives, attributes, children).
fn build_html_element(
	el: &BsxElement,
	registry: &BsxTemplateRegistry,
	refs: &RefBindings,
	cx: &mut TemplateContext,
) -> Result<()> {
	cx.entity.insert(Element::new(el.tag.clone()));
	build_fragment(el, registry, refs, cx)
}

/// Build an element's neutral content onto its host: directive components,
/// attribute child entities and child node entities, with no `Element` of its
/// own. Shared by [`build_html_element`] and the reserved `<Fragment>` host.
fn build_fragment(
	el: &BsxElement,
	registry: &BsxTemplateRegistry,
	refs: &RefBindings,
	cx: &mut TemplateContext,
) -> Result<()> {
	apply_common_directives(el, refs, cx)?;
	// pre-resolve `$name` refs (forward-aware) so spreads have a plain lookup.
	let entity_refs = resolve_entity_refs(el, refs, cx);
	apply_attributes(el, cx.entity, &entity_refs)?;
	build_children(el, registry, refs, cx)
}

/// Build a `<Slot>` placeholder into `entity` as a [`SlotTarget`] marker, with
/// its children as fallback content. A named slot targets by name; a transfer
/// `<Slot name bx:slot=..>` also carries a [`SlotChild`].
fn build_slot(el: &BsxElement, entity: &mut EntityWorldMut) -> Result<()> {
	let name = string_attr(el, "name");
	match &name {
		Some(name) => entity.insert(SlotTarget::named(name.clone())),
		None => entity.insert(SlotTarget::new()),
	};
	// a transfer: route this slot's content into a parent slot.
	if let Some(slot) = slot_routing(el) {
		entity.insert(slot);
	}
	// fallback children spawn beneath the target.
	let id = entity.id();
	if !el.children.is_empty() {
		entity.world_scope(|world| -> Result<()> {
			let mut references =
				bevy::ecs::template::SceneEntityReferences::default();
			let mut entity_mut = world.entity_mut(id);
			let mut cx = TemplateContext::new(&mut entity_mut, &mut references);
			let refs = RefBindings::default();
			let registry = BsxTemplateRegistry::default();
			for child in &el.children {
				spawn_child(child, id, &registry, &refs, &mut cx)?;
			}
			Ok(())
		})?;
	}
	Ok(())
}

/// Apply the `bx:scope`/`bx:for`/`bx:key`/`bx:ref`/`bx:<event>`/`slot` directives
/// shared by every tag kind onto `cx.entity`.
pub(super) fn apply_common_directives(
	el: &BsxElement,
	refs: &RefBindings,
	cx: &mut TemplateContext,
) -> Result<()> {
	// `bx:ref` names this entity: bind the pinned reference to it.
	if let Some(name) = string_attr(el, "bx:ref") {
		if let Some(reference) = refs.get(&name) {
			let id = cx.entity.id();
			cx.entity_references.set(reference, id);
		}
	}
	// `bx:scope` -> a `DocumentScope` prefix.
	if let Some(scope) = string_attr(el, "bx:scope") {
		cx.entity.insert(DocumentScope {
			path: FieldPath::new([scope]),
			terminate: false,
		});
	}
	// a `slot`/`bx:slot` routes this whole node into a parent slot.
	if let Some(slot) = slot_routing(el) {
		cx.entity.insert(slot);
	}
	// `bx:style="prop=value .."` declares a one-off rule and attaches a unique,
	// span-derived class, the markup twin of `inline_class!`. The declaration
	// grammar lives in a higher crate, so it resolves through the `StyleResolver`
	// seam (a graceful no-op when no handler is registered).
	if let Some((source, span)) = bsx_style_attr(el) {
		let handler = cx
			.entity
			.world_scope(|world| world.get_resource::<StyleResolver>()?.get());
		if let Some(handler) = handler {
			handler(cx.entity, source, span)?;
		}
	}
	// `bx:<event>="script"` events. The event name is the directive suffix after
	// `bx:`; the script resolves through the core seam.
	for attr in &el.attributes {
		if !is_event_directive(&attr.key) {
			continue;
		}
		let AttrValue::Str(script) = &attr.value else {
			bevybail!(
				"`{key}` expects a script string, ie \
				 `{key}=\"await target.set_field('count', 1)\"`",
				key = attr.key
			);
		};
		let event = attr.key.strip_prefix("bx:").unwrap_or(&attr.key);
		install_event(cx.entity, &EventBinding::new(event, script.as_str()));
	}
	Ok(())
}

/// Build an element's child nodes. A `bx:for` element instead materializes a
/// reactive list, one child per item over the named array field.
pub(super) fn build_children(
	el: &BsxElement,
	registry: &BsxTemplateRegistry,
	refs: &RefBindings,
	cx: &mut TemplateContext,
) -> Result<()> {
	if let Some(field) = string_attr(el, "bx:for") {
		return build_reactive_children(el, &field, registry, refs, cx);
	}
	let parent = cx.entity.id();
	for child in &el.children {
		spawn_child(child, parent, registry, refs, cx)?;
	}
	Ok(())
}

/// `bx:for="items"` + optional `bx:key`: a [`ReactiveChildren`] over the `items`
/// array, spawning the element's child template per item, each in a terminating
/// index scope. With `bx:key="field"` it reconciles by that per-item key so an
/// append reuses existing rows instead of rebuilding them.
fn build_reactive_children(
	el: &BsxElement,
	field: &str,
	registry: &BsxTemplateRegistry,
	_refs: &RefBindings,
	cx: &mut TemplateContext,
) -> Result<()> {
	let template = el.children.clone();
	let registry = registry.clone();
	// each item child builds the element's body as its own template; the
	// terminating index scope `ReactiveChildren` adds resolves its fields.
	let build = move |_index: usize, _item: &Value| {
		let template = template.clone();
		let registry = registry.clone();
		OnSpawn::new(move |entity| {
			let nested =
				BsxTemplate::container(template.clone(), registry.clone());
			if let Err(error) = entity.build_template(&nested) {
				entity.insert(TemplateError::new(error));
			}
		})
	};
	// the field backing the list: its synced `Value` drives the rebuild.
	cx.entity.insert(FieldRef::new(field));
	match string_attr(el, "bx:key") {
		Some(key_path) => {
			let key_path = key_path.to_string();
			cx.entity.insert(ReactiveChildren::keyed(
				move |item| reactive_item_key(item, &key_path),
				build,
			));
		}
		None => {
			cx.entity.insert(ReactiveChildren::new(build));
		}
	}
	Ok(())
}

/// Extract a reconciliation key from a `bx:for` item by a dotted `bx:key` path,
/// eg `bx:key="id"` or `bx:key="user.id"`. Falls back to a debug repr for
/// non-string keys (eg numeric ids), which is stable within a session.
fn reactive_item_key(item: &Value, key_path: &str) -> String {
	let mut current = item;
	for segment in key_path.split('.') {
		match current.get(segment) {
			Some(next) => current = next,
			None => return String::new(),
		}
	}
	current
		.as_str()
		.map(str::to_string)
		.unwrap_or_else(|_| format!("{current:?}"))
}

/// Build the caller content of an uppercase tag as slot children: each child its
/// own entity carrying a default [`SlotChild`] unless it routes itself.
pub(super) fn build_slot_children(
	el: &BsxElement,
	registry: &BsxTemplateRegistry,
	refs: &RefBindings,
	cx: &mut TemplateContext,
) -> Result<()> {
	let parent = cx.entity.id();
	for child in &el.children {
		let entity = spawn_child(child, parent, registry, refs, cx)?;
		// an unrouted child goes to the default slot.
		if !child_routes_itself(child) {
			// SAFETY: only used to mark the spawned slot child.
			let world = unsafe { cx.entity.world_mut() };
			if !world.entity(entity).contains::<SlotChild>() {
				world.entity_mut(entity).insert(SlotChild::new());
			}
		}
	}
	Ok(())
}

/// Whether a child element carries its own `slot`/`bx:slot` routing.
fn child_routes_itself(node: &BsxNode) -> bool {
	matches!(node, BsxNode::Element(el) if slot_routing(el).is_some())
}

/// Apply an element's attributes as attribute child entities, the
/// `Element` + `related!(Attributes[..])` shape `rsx!` produces.
fn apply_attributes(
	el: &BsxElement,
	entity: &mut EntityWorldMut,
	entity_refs: &HashMap<SmolStr, Entity>,
) -> Result<()> {
	let parent = entity.id();
	for attr in &el.attributes {
		// directives and spreads are not HTML attributes.
		if is_directive(&attr.key) || attr.key.is_empty() {
			if let AttrValue::Spread(_) = &attr.value {
				// spreads handled separately below.
			}
			continue;
		}
		// `on*` event handlers are not implemented as data attributes here.
		entity.world_scope(|world| -> Result<()> {
			let mut attr_entity = world.spawn((
				AttributeOf::new(parent),
				Attribute::new(attr.key.clone()),
			));
			match &attr.value {
				AttrValue::Flag => {}
				AttrValue::Str(string) => {
					attr_entity.insert(Value::Str(string.into()));
				}
				AttrValue::Expr(expr) => {
					// attribute position: an `@comp` binds the element unless
					// `$ref` retargets.
					let comp_target =
						attr_comp_target(expr, parent, entity_refs);
					apply_value_expr(expr, &mut attr_entity, comp_target)?;
				}
				// spreads and `bx:style` are handled elsewhere (the
				// directives pass / spread pass).
				AttrValue::Spread(_) | AttrValue::Style { .. } => {}
			}
			Ok(())
		})?;
	}
	apply_spreads(el, entity, entity_refs)
}
