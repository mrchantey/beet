//! Element building: the tag dispatch, the neutral content every element
//! carries (directives, attributes, children), and the slot machinery.

use super::binding::*;
use super::directives::*;
use super::entity_refs::*;
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
	if el.tag == "slot" {
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
/// [`build_fragment`] content (directives, attributes, children). A
/// `<template>` is a [`Prefab`], its subtree held as the item a
/// [`ReactiveChildren`] clones, as HTML's `<template>` holds inert content.
fn build_html_element(
	el: &BsxElement,
	registry: &BsxTemplateRegistry,
	refs: &RefBindings,
	cx: &mut TemplateContext,
) -> Result<()> {
	cx.entity.insert(Element::new(el.tag.clone()));
	if el.tag == "template" {
		cx.entity.insert(Prefab);
	}
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

/// Build a `<slot>` placeholder into `entity` as a [`SlotTarget`] marker, with
/// its children as fallback content. A named slot targets by name; a transfer
/// `<slot name slot=..>` also carries a [`SlotChild`].
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

/// Apply the `bx:ref` and `slot` directives shared by every tag kind onto
/// `cx.entity`, the ones the build walk still reads.
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
	// a `slot` routes this whole node into a parent slot.
	if let Some(slot) = slot_routing(el) {
		cx.entity.insert(slot);
	}
	Ok(())
}

/// Build an element's child nodes.
pub(super) fn build_children(
	el: &BsxElement,
	registry: &BsxTemplateRegistry,
	refs: &RefBindings,
	cx: &mut TemplateContext,
) -> Result<()> {
	let parent = cx.entity.id();
	for child in &el.children {
		spawn_child(child, parent, registry, refs, cx)?;
	}
	Ok(())
}

/// Build the caller content of an uppercase tag as slot children: each child its
/// own entity carrying a default [`SlotChild`] unless it routes itself. A
/// child that never renders, a `<script {OnEvent}>` a `bx:onclick` rewrote to,
/// is behaviour of the tag itself and stays on it, never slotted.
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
			let child = world.entity(entity);
			if !child.contains::<SlotChild>()
				&& !child.contains::<NotRendered>()
			{
				world.entity_mut(entity).insert(SlotChild::new());
			}
		}
	}
	Ok(())
}

/// Whether a child element carries its own `slot` routing.
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
	// directives and spreads are not HTML attributes; spreads apply below.
	for attr in el
		.attributes
		.iter()
		.filter(|attr| !is_directive(&attr.key) && !attr.key.is_empty())
	{
		apply_attribute(attr, entity, entity_refs)?;
	}
	apply_spreads(el, entity, entity_refs)
}

/// Spawn one HTML attribute of `entity` as an attribute entity, its value a
/// literal, a flag or a bound expression. Shared by a lowercase element and the
/// attributes a template tag forwards to its root element.
pub(super) fn apply_attribute(
	attr: &BsxAttribute,
	entity: &mut EntityWorldMut,
	entity_refs: &HashMap<SmolStr, Entity>,
) -> Result<()> {
	let parent = entity.id();
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
				let comp_target = attr_comp_target(expr, parent, entity_refs);
				apply_value_expr(expr, &mut attr_entity, comp_target)?;
			}
			// spreads apply in the spread pass
			AttrValue::Spread(_) => {}
		}
		Ok(())
	})
}
