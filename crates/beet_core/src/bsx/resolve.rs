//! AST-to-world resolution: build a [`BsxNode`] tree into an entity through the
//! substrate, producing trees identical to what `rsx!` lowers to.
//!
//! [`BsxTemplate`] is the [`Template`] every BSX front-end produces. Its build
//! walks the syntax tree into `cx.entity`:
//!
//! - a lowercase tag becomes an [`Element`] with attribute child entities;
//! - an uppercase tag resolves by name to a component (reflect-patched) or a
//!   template (built with input props), incl `<path::to::X>` BSX templates;
//! - a bare spread inserts its resolved components/templates onto the entity;
//! - a `#`reference lowers to a [`FieldRef`] (with `=init`), a `$`reference to a
//!   `bx:ref`-named entity through the one entity model;
//! - `bx:scope`/`bx:for`+`bx:key`/`<Slot>`/`bx:slot`/`bx:click` lower to their
//!   document-system and slot-marker components.
//!
//! Entity references resolve through `cx.entity_references` with a two-pass walk
//! (collect `bx:ref` names, then resolve `$name`), so `$name` may point forward.

use super::ast::*;
use super::events::*;
use super::reflect::*;
use super::registry::*;
use crate::prelude::*;
use bevy::ecs::template::SceneEntityReference;
use bevy::ecs::template::Template;
use bevy::ecs::template::TemplateContext;
use bevy::reflect::TypeRegistry;

/// A parsed BSX tree as a build-subtree [`Template`].
///
/// Holds its root nodes and the BSX-template registry snapshot, so resolving a
/// `<path::to::X>` tag needs no world lookup mid-build. Built once into the
/// caller's entity by `spawn_template`/`insert_template`.
#[derive(Clone)]
pub struct BsxTemplate {
	/// The root nodes to build into the calling entity.
	pub nodes: Vec<BsxNode>,
	/// A snapshot of the BSX-template registry for `<path::to::X>` resolution.
	pub registry: BsxTemplateRegistry,
	/// When `true`, the calling entity is a container and every root node spawns
	/// as its own child (the parse-a-document convention, matching the HTML
	/// parser). When `false`, a single root builds into the entity (the nested
	/// `<path::to::X>` body convention, so it composes onto the caller).
	pub as_container: bool,
}

subtree_template!(BsxTemplate);

impl BsxTemplate {
	/// A nested-template body: a single root builds into the calling entity.
	pub fn new(nodes: Vec<BsxNode>, registry: BsxTemplateRegistry) -> Self {
		Self {
			nodes,
			registry,
			as_container: false,
		}
	}

	/// A parsed document: the calling entity is a container, roots become children.
	pub fn container(nodes: Vec<BsxNode>, registry: BsxTemplateRegistry) -> Self {
		Self {
			nodes,
			registry,
			as_container: true,
		}
	}
}

impl Template for BsxTemplate {
	type Output = ();
	fn build_template(&self, cx: &mut TemplateContext) -> Result<()> {
		// pass 1: collect every `bx:ref` name -> a pinned reference id, so a `$name`
		// forward reference resolves to the same placeholder entity.
		let mut refs = RefBindings::default();
		collect_refs(&self.nodes, &mut refs);
		if self.as_container {
			// every root node spawns as a child of the container entity.
			let root = cx.entity.id();
			for node in &self.nodes {
				spawn_child(node, root, &self.registry, &refs, cx)?;
			}
			Ok(())
		} else {
			build_root_nodes(&self.nodes, &self.registry, &refs, cx)
		}
	}
	fn clone_template(&self) -> Self { self.clone() }
}

/// Names declared by a `bx:ref` anywhere in the tree, each pinned to a stable
/// [`SceneEntityReference`] so a forward `$name` resolves identically.
#[derive(Default)]
struct RefBindings {
	names: HashMap<SmolStr, SceneEntityReference>,
}

impl RefBindings {
	/// The pinned reference for `name`, allocating a stable one on first use.
	fn reference(&mut self, name: &str) -> SceneEntityReference {
		let next = self.names.len();
		*self
			.names
			.entry(name.into())
			.or_insert_with(|| SceneEntityReference::new(("bsx_ref", 0, 0), next))
	}

	/// The pinned reference for `name`, if declared by a `bx:ref`.
	fn get(&self, name: &str) -> Option<SceneEntityReference> {
		self.names.get(name).copied()
	}
}

/// Walk the tree collecting every `bx:ref` name into stable references.
fn collect_refs(nodes: &[BsxNode], refs: &mut RefBindings) {
	for node in nodes {
		if let BsxNode::Element(el) = node {
			for attr in &el.attributes {
				if attr.key == "bx:ref" {
					if let AttrValue::Str(name) = &attr.value {
						refs.reference(name);
					}
				}
			}
			collect_refs(&el.children, refs);
		}
	}
}

/// Build the root nodes into the context entity. The first node builds into the
/// root; the rest spawn as children, matching the `rsx!` fragment lowering.
fn build_root_nodes(
	nodes: &[BsxNode],
	registry: &BsxTemplateRegistry,
	refs: &RefBindings,
	cx: &mut TemplateContext,
) -> Result<()> {
	match nodes {
		[] => Ok(()),
		[single] => build_node_into(single, registry, refs, cx),
		many => {
			// multiple roots: each spawns its own child entity under the root.
			let root = cx.entity.id();
			for node in many {
				spawn_child(node, root, registry, refs, cx)?;
			}
			Ok(())
		}
	}
}

/// Build a single node directly into `cx.entity` (the root case).
fn build_node_into(
	node: &BsxNode,
	registry: &BsxTemplateRegistry,
	refs: &RefBindings,
	cx: &mut TemplateContext,
) -> Result<()> {
	match node {
		BsxNode::Element(el) => build_element(el, registry, refs, cx),
		_ => {
			apply_leaf(node, cx.entity)?;
			Ok(())
		}
	}
}

/// Spawn `node` as a child of `parent`, returning the spawned entity.
fn spawn_child(
	node: &BsxNode,
	parent: Entity,
	registry: &BsxTemplateRegistry,
	refs: &RefBindings,
	cx: &mut TemplateContext,
) -> Result<Entity> {
	// a `bx:ref`-named element reuses its pinned reference entity (which a forward
	// `$name` may already have spawned as a placeholder), so the reference and the
	// built node are the same entity.
	let child = match node_ref_name(node).and_then(|name| refs.get(name)) {
		Some(reference) => {
			// SAFETY: only used to spawn-or-fetch the mapped placeholder entity.
			let world = unsafe { cx.entity.world_mut() };
			let child = cx.entity_references.get(reference, world);
			world.entity_mut(child).insert(ChildOf(parent));
			child
		}
		None => {
			// SAFETY: only used to spawn a child entity.
			let world = unsafe { cx.entity.world_mut() };
			world.spawn(ChildOf(parent)).id()
		}
	};
	build_node_at(node, child, registry, refs, cx)?;
	Ok(child)
}

/// The `bx:ref="name"` declared by an element node, if any.
fn node_ref_name(node: &BsxNode) -> Option<&str> {
	let BsxNode::Element(el) = node else {
		return None;
	};
	el.attributes.iter().find_map(|attr| {
		if attr.key != "bx:ref" {
			return None;
		}
		match &attr.value {
			AttrValue::Str(name) => Some(name.as_str()),
			_ => None,
		}
	})
}

/// Build `node` onto the already-spawned `entity`.
fn build_node_at(
	node: &BsxNode,
	entity: Entity,
	registry: &BsxTemplateRegistry,
	refs: &RefBindings,
	cx: &mut TemplateContext,
) -> Result<()> {
	// SAFETY: scope a build into the target entity, sharing the reference map.
	let world = unsafe { cx.entity.world_mut() };
	let mut entity_mut = world.entity_mut(entity);
	let mut scoped = TemplateContext::new(&mut entity_mut, cx.entity_references);
	match node {
		BsxNode::Element(el) => build_element(el, registry, refs, &mut scoped),
		_ => apply_leaf(node, scoped.entity),
	}
}

/// Apply a text/expr/comment/doctype leaf onto `entity`.
fn apply_leaf(node: &BsxNode, entity: &mut EntityWorldMut) -> Result<()> {
	match node {
		BsxNode::Text(text) => {
			entity.insert(Value::Str(text.into()));
		}
		BsxNode::Expr(expr) => apply_value_expr(expr, entity)?,
		BsxNode::Comment(content) => {
			entity.insert(Comment::new(content.clone()));
		}
		BsxNode::Doctype(value) => {
			entity.insert(Doctype::new(value.clone()));
		}
		BsxNode::Element(_) => unreachable!("handled before apply_leaf"),
	}
	Ok(())
}

/// Lower a text-position value expression onto `entity`: a literal becomes a
/// [`Value`], a `#`reference a [`FieldRef`], a `$`reference is rejected (an
/// entity reference is not a text value).
fn apply_value_expr(expr: &ValueExpr, entity: &mut EntityWorldMut) -> Result<()> {
	match expr {
		ValueExpr::Literal(literal) => {
			entity.insert(literal_to_value(literal)?);
		}
		ValueExpr::FieldRef { path, init } => {
			entity.insert(field_ref(path, init.as_ref())?);
		}
		ValueExpr::EntityRef(_) => {
			bevybail!("`$name` entity references are not valid in text position")
		}
	}
	Ok(())
}

/// Build an element: dispatch on its tag kind, then directives, attributes, and
/// children.
fn build_element(
	el: &BsxElement,
	registry: &BsxTemplateRegistry,
	refs: &RefBindings,
	cx: &mut TemplateContext,
) -> Result<()> {
	if el.tag == "Slot" {
		return build_slot(el, cx.entity);
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

/// Build a lowercase HTML element: `Element` + directive components + attribute
/// child entities + child node entities.
fn build_html_element(
	el: &BsxElement,
	registry: &BsxTemplateRegistry,
	refs: &RefBindings,
	cx: &mut TemplateContext,
) -> Result<()> {
	cx.entity.insert(Element::new(el.tag.clone()));
	apply_common_directives(el, refs, cx)?;
	// pre-resolve `$name` refs (forward-aware) so spreads have a plain lookup.
	let entity_refs = resolve_entity_refs(el, refs, cx);
	apply_attributes(el, cx.entity, &entity_refs)?;
	build_children(el, registry, refs, cx)
}

/// Resolve every `$name` referenced by `el`'s attribute literals to a real,
/// forward-mapped entity through the one entity model, keyed by name for a plain
/// lookup during reflect-patch building.
fn resolve_entity_refs(
	el: &BsxElement,
	refs: &RefBindings,
	cx: &mut TemplateContext,
) -> HashMap<SmolStr, Entity> {
	let mut out = HashMap::default();
	let mut names = Vec::new();
	for attr in &el.attributes {
		match &attr.value {
			AttrValue::Spread(spread) => collect_entity_ref_names(spread, &mut names),
			AttrValue::Expr(ValueExpr::Literal(literal)) => {
				collect_literal_entity_ref_names(literal, &mut names)
			}
			AttrValue::Expr(ValueExpr::EntityRef(name)) => names.push(name.clone()),
			_ => {}
		}
	}
	for name in names {
		let reference = refs
			.get(&name)
			.unwrap_or_else(|| stable_reference(&name));
		// SAFETY: only used to spawn-or-fetch the mapped placeholder entity.
		let world = unsafe { cx.entity.world_mut() };
		let entity = cx.entity_references.get(reference, world);
		out.insert(name, entity);
	}
	out
}

/// Collect every `$name` in a spread's literals.
fn collect_entity_ref_names(spread: &SpreadExpr, out: &mut Vec<SmolStr>) {
	let items = match spread {
		SpreadExpr::Named(named) => core::slice::from_ref(named),
		SpreadExpr::Tuple(items) => items.as_slice(),
	};
	for named in items {
		collect_literal_entity_ref_names(&DataLiteral::Enum(named.clone()), out);
	}
}

/// Collect every `$name` referenced anywhere inside a literal.
fn collect_literal_entity_ref_names(literal: &DataLiteral, out: &mut Vec<SmolStr>) {
	match literal {
		DataLiteral::EntityRef(name) => out.push(name.clone()),
		DataLiteral::List(items) => items
			.iter()
			.for_each(|item| collect_literal_entity_ref_names(item, out)),
		DataLiteral::Struct(fields) => fields
			.iter()
			.for_each(|(_, item)| collect_literal_entity_ref_names(item, out)),
		DataLiteral::Enum(named) => match &named.fields {
			NamedFields::Tuple(items) => items
				.iter()
				.for_each(|item| collect_literal_entity_ref_names(item, out)),
			NamedFields::Struct(fields) => fields
				.iter()
				.for_each(|(_, item)| collect_literal_entity_ref_names(item, out)),
			NamedFields::Unit => {}
		},
		DataLiteral::Scalar(_) => {}
	}
}

/// A stable [`SceneEntityReference`] for a `$name` with no declared `bx:ref`, so
/// a dangling reference resolves to a consistent placeholder rather than erroring.
fn stable_reference(name: &str) -> SceneEntityReference {
	let hash = name.bytes().fold(0u64, |acc, byte| {
		acc.wrapping_mul(31).wrapping_add(byte as u64)
	});
	SceneEntityReference::new(("bsx_ref", 0, 0), hash as usize)
}

/// Build a capitalized tag: a component (reflect-patched) or a template (built
/// with input props), incl `<path::to::X>` BSX templates.
fn build_uppercase(
	el: &BsxElement,
	registry: &BsxTemplateRegistry,
	refs: &RefBindings,
	cx: &mut TemplateContext,
) -> Result<()> {
	// `<Template src="..">` is the remote-template front-end: register a pending
	// fetch into the root's pending set, awaited by `LoadTemplate`. Remote
	// resolution needs the async runtime, so it is `bevy_async`-gated.
	if el.tag == "Template" {
		if let Some(_src) = string_attr(el, "src") {
			#[cfg(feature = "bevy_async")]
			register_remote_template(SmolStr::from(_src.as_str()), cx)?;
		}
		apply_common_directives(el, refs, cx)?;
		return Ok(());
	}

	// a `<path::to::X>` BSX template resolves from the registry first.
	if let Some(def) = registry.get(&el.tag) {
		// a remote schema resolves asynchronously, deferring `LoadTemplate`.
		if let Some(_url) = def.remote_schema.clone() {
			#[cfg(feature = "bevy_async")]
			register_remote_schema(SmolStr::from(el.tag.as_str()), _url, cx)?;
		}
		// verify props against the template's inline `bx:schema`, if it declared one.
		if let Some(schema) = def.schema.clone() {
			verify_props_against(el, &el.tag, &schema, cx)?;
		}
		let nested = BsxTemplate::new(def.nodes.clone(), registry.clone());
		// build the template's subtree into this entity, carrying its slot targets.
		cx.entity.build_template(&nested)?;
		apply_common_directives(el, refs, cx)?;
		// caller content becomes slot children on this entity.
		build_slot_children(el, registry, refs, cx)?;
		return Ok(());
	}

	let app_registry = cx
		.entity
		.world_scope(|world| world.get_resource::<AppTypeRegistry>().cloned())
		.ok_or_else(|| {
			bevyhow!(
				"resolving the `<{}>` tag requires an `AppTypeRegistry`",
				el.tag
			)
		})?;
	// pre-resolve every `$name` in this tag's attributes to a real (forward-mapped)
	// entity, so patch building has a plain name->entity lookup.
	let entity_refs = resolve_entity_refs(el, refs, cx);
	let registration_kind = {
		let registry = app_registry.read();
		registry
			.get_with_short_type_path(&el.tag)
			.map(|registration| {
				(
					registration.data::<ReflectTemplate>().is_some(),
					registration.type_info(),
				)
			})
			.map(|(is_template, info)| {
				(is_template, build_patch(el, info, &registry, &entity_refs))
			})
	};

	let Some((is_template, patch)) = registration_kind else {
		bevybail!("no component or template registered for tag `{}`", el.tag);
	};
	let patch = patch?;

	if is_template {
		// verify the props against the template's schema before building, so a
		// missing required field or a type mismatch is a graceful error.
		verify_props(el, &el.tag, &app_registry, cx)?;
		// build the registered template into this entity, then route caller content.
		build_template_by_name(&app_registry, &el.tag, patch.as_ref(), cx)?;
		apply_common_directives(el, refs, cx)?;
		build_slot_children(el, registry, refs, cx)?;
	} else {
		// a component: reflect-patch over default and insert.
		insert_component(cx.entity, patch.as_ref(), &app_registry)?;
		// a `<MyComponent value=#path>` field reference binds the document field to
		// the component field, both ways, via a reflect-field binding.
		apply_reflect_field_bindings(el, cx.entity)?;
		apply_common_directives(el, refs, cx)?;
		build_children(el, registry, refs, cx)?;
	}
	Ok(())
}

/// Insert a reflect-field binding for every `field=#path` attribute on a
/// component tag, so the document field syncs with the component field both ways.
///
/// The first such attribute owns the entity's [`FieldRef`] (one per entity) plus
/// a [`ReflectFieldRef`] naming the component and field, and a default [`Value`]
/// the bidirectional sync fills.
fn apply_reflect_field_bindings(
	el: &BsxElement,
	entity: &mut EntityWorldMut,
) -> Result<()> {
	for attr in &el.attributes {
		if is_directive(&attr.key) || attr.key.is_empty() {
			continue;
		}
		let AttrValue::Expr(ValueExpr::FieldRef { path, init }) = &attr.value else {
			continue;
		};
		entity.insert(field_ref(path, init.as_ref())?);
		// the `Value`<->reflect bridge needs `serde_json` (the `json` feature); an
		// embedded build without it keeps the `FieldRef` binding but loses the
		// bidirectional reflect-field write (Risk: documented, acceptable).
		#[cfg(feature = "json")]
		{
			let component = el.tag.rsplit("::").next().unwrap_or(&el.tag);
			entity.insert(ReflectFieldRef::new(component, attr.key.as_str()));
		}
		// one FieldRef per entity: the first field reference owns the binding.
		break;
	}
	Ok(())
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
			let mut references = bevy::ecs::template::SceneEntityReferences::default();
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

/// Apply the `bx:scope`/`bx:for`/`bx:key`/`bx:ref`/`bx:click`/`slot` directives
/// shared by every tag kind onto `cx.entity`.
fn apply_common_directives(
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
	// `bx:<event>=verb#field` events. The event name is the directive suffix
	// after `bx:`; the verb + field resolve through the core registries.
	for attr in &el.attributes {
		if let Some(event) = attr.key.strip_prefix("bx:")
			&& BSX_EVENTS.contains(&event)
		{
			let binding = parse_event_binding(event, &attr.value)?;
			install_event(cx.entity, &binding);
		}
	}
	Ok(())
}

/// Build an element's child nodes. A `bx:for` element instead materializes a
/// reactive list, one child per item over the named array field.
fn build_children(
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

/// `bx:for="items"` + `bx:key`: a [`ReactiveChildren`] over the `items` array,
/// spawning the element's child template per item, each in a terminating index
/// scope.
fn build_reactive_children(
	el: &BsxElement,
	field: &str,
	registry: &BsxTemplateRegistry,
	_refs: &RefBindings,
	cx: &mut TemplateContext,
) -> Result<()> {
	let template = el.children.clone();
	let registry = registry.clone();
	// the field backing the list: its synced `Value` drives the rebuild.
	cx.entity.insert(FieldRef::new(field));
	cx.entity.insert(ReactiveChildren::new(move |_index, _item| {
		let template = template.clone();
		let registry = registry.clone();
		OnSpawn::new(move |entity| {
			// each item child builds the element's body as its own template; the
			// terminating index scope `ReactiveChildren` adds resolves its fields.
			let nested = BsxTemplate::container(template.clone(), registry.clone());
			if let Err(error) = entity.build_template(&nested) {
				entity.insert(TemplateError::new(error));
			}
		})
	}));
	Ok(())
}

/// Build the caller content of an uppercase tag as slot children: each child its
/// own entity carrying a default [`SlotChild`] unless it routes itself.
fn build_slot_children(
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
					apply_value_expr(expr, &mut attr_entity)?;
				}
				AttrValue::Spread(_) => {}
			}
			Ok(())
		})?;
	}
	// bare-position spreads insert components/templates onto the element itself.
	// the registry is only needed when a spread is present, so a plain HTML/markdown
	// parse (no `AppTypeRegistry`) never touches it.
	let has_spread = el
		.attributes
		.iter()
		.any(|attr| matches!(attr.value, AttrValue::Spread(_)));
	if has_spread {
		let app_registry = entity
			.world_scope(|world| world.get_resource::<AppTypeRegistry>().cloned())
			.ok_or_else(|| {
				bevyhow!("a spread requires an `AppTypeRegistry` in the world")
			})?;
		for attr in &el.attributes {
			if let AttrValue::Spread(spread) = &attr.value {
				apply_spread(spread, entity, &app_registry, entity_refs)?;
			}
		}
	}
	Ok(())
}

/// Insert or build a spread's components/templates onto `entity`.
fn apply_spread(
	spread: &SpreadExpr,
	entity: &mut EntityWorldMut,
	app_registry: &AppTypeRegistry,
	entity_refs: &HashMap<SmolStr, Entity>,
) -> Result<()> {
	let items = match spread {
		SpreadExpr::Named(named) => core::slice::from_ref(named),
		SpreadExpr::Tuple(items) => items.as_slice(),
	};
	for named in items {
		let literal = DataLiteral::Enum(named.clone());
		let (is_template, patch) = {
			let registry = app_registry.read();
			let info = type_info_by_name(&registry, &named.name);
			let is_template = registry
				.get_with_short_type_path(&named.name)
				.map(|registration| registration.data::<ReflectTemplate>().is_some())
				.unwrap_or(false);
			let mut resolver = entity_ref_resolver(entity_refs);
			(
				is_template,
				literal_to_reflect(&literal, info, &registry, &mut resolver)?,
			)
		};
		if is_template {
			let id = entity.id();
			entity.world_scope(|world| -> Result<()> {
				let mut references =
					bevy::ecs::template::SceneEntityReferences::default();
				let mut entity_mut = world.entity_mut(id);
				let mut cx =
					TemplateContext::new(&mut entity_mut, &mut references);
				build_template_by_name(
					app_registry,
					&named.name,
					patch.as_ref(),
					&mut cx,
				)
			})?;
		} else {
			insert_component(entity, patch.as_ref(), app_registry)?;
		}
	}
	Ok(())
}

/// Build a reflect patch for an uppercase tag's attributes against its type
/// info, so each value coerces to the field's concrete type.
fn build_patch(
	el: &BsxElement,
	type_info: &'static bevy::reflect::TypeInfo,
	registry: &TypeRegistry,
	entity_refs: &HashMap<SmolStr, Entity>,
) -> Result<Box<dyn bevy::reflect::PartialReflect>> {
	use bevy::reflect::structs::DynamicStruct;
	let struct_info = match type_info {
		bevy::reflect::TypeInfo::Struct(info) => Some(info),
		_ => None,
	};
	let mut patch = DynamicStruct::default();
	for attr in &el.attributes {
		if is_directive(&attr.key) || attr.key.is_empty() {
			continue;
		}
		// a `#field` reference becomes a reflect-field binding, not a patch field.
		if matches!(
			&attr.value,
			AttrValue::Expr(ValueExpr::FieldRef { .. })
		) {
			continue;
		}
		let field_info = struct_info
			.and_then(|info| info.field(&attr.key))
			.and_then(|field| field.type_info());
		let literal = attr_to_literal(&attr.value)?;
		let mut resolver = entity_ref_resolver(entity_refs);
		// a `#[template]` stores an optional/required prop as `PropOpt<T>`; a markup
		// value must wrap into `PropOpt(Some(value))` to apply over the default.
		let reflected = match prop_opt_inner_info(field_info) {
			Some(inner_info) => prop_opt_value(
				&literal,
				field_info,
				inner_info,
				registry,
				&mut resolver,
			)?,
			None => {
				literal_to_reflect(&literal, field_info, registry, &mut resolver)?
			}
		};
		patch.insert_boxed(&attr.key, reflected);
	}
	patch.set_represented_type(Some(type_info));
	Ok(Box::new(patch))
}

/// If `field_info` is a `PropOpt<T>` tuple struct, the inner `Option<T>`'s
/// [`TypeInfo`], else `None`. A `#[template]`'s optional/required props store as
/// `PropOpt<T>`, so a markup value targeting one must wrap into the option.
fn prop_opt_inner_info(
	field_info: Option<&'static bevy::reflect::TypeInfo>,
) -> Option<&'static bevy::reflect::TypeInfo> {
	let bevy::reflect::TypeInfo::TupleStruct(info) = field_info? else {
		return None;
	};
	if !info.type_path().contains("PropOpt<") {
		return None;
	}
	info.field_at(0).and_then(|field| field.type_info())
}

/// Build a `PropOpt(Some(value))` reflected value for a `PropOpt<T>` field, so a
/// markup prop value reaches a `#[template]`'s optional/required prop.
fn prop_opt_value(
	literal: &DataLiteral,
	field_info: Option<&'static bevy::reflect::TypeInfo>,
	option_info: &'static bevy::reflect::TypeInfo,
	registry: &TypeRegistry,
	resolver: &mut dyn FnMut(&str) -> Entity,
) -> Result<Box<dyn bevy::reflect::PartialReflect>> {
	use bevy::reflect::enums::DynamicEnum;
	use bevy::reflect::enums::DynamicVariant;
	use bevy::reflect::enums::VariantInfo;
	use bevy::reflect::tuple::DynamicTuple;
	use bevy::reflect::tuple_struct::DynamicTupleStruct;
	// the `Option<T>` carried by `PropOpt<T>(Option<T>)`; resolve the inner `T`.
	let inner_info = match option_info {
		bevy::reflect::TypeInfo::Enum(enum_info) => enum_info
			.variant("Some")
			.and_then(|variant| match variant {
				VariantInfo::Tuple(tuple) => tuple.field_at(0),
				_ => None,
			})
			.and_then(|field| field.type_info()),
		_ => None,
	};
	let inner = literal_to_reflect(literal, inner_info, registry, resolver)?;
	// `Some(inner)`
	let mut some = DynamicTuple::default();
	some.insert_boxed(inner);
	let mut option = DynamicEnum::new("Some", DynamicVariant::Tuple(some));
	option.set_represented_type(Some(option_info));
	// `PropOpt(Some(inner))`
	let mut prop_opt = DynamicTupleStruct::default();
	prop_opt.insert_boxed(Box::new(option));
	prop_opt.set_represented_type(field_info);
	Ok(Box::new(prop_opt))
}

/// An [`EntityResolver`] over a pre-resolved name->entity map; an unknown name
/// falls back to a placeholder so a dangling `$name` never panics.
fn entity_ref_resolver(
	entity_refs: &HashMap<SmolStr, Entity>,
) -> impl FnMut(&str) -> Entity + '_ {
	move |name| entity_refs.get(name).copied().unwrap_or(Entity::PLACEHOLDER)
}

/// Insert a reflect-patched component over its default onto `entity`.
fn insert_component(
	entity: &mut EntityWorldMut,
	patch: &dyn bevy::reflect::PartialReflect,
	app_registry: &AppTypeRegistry,
) -> Result<()> {
	use bevy::ecs::reflect::ReflectComponent;
	let registry = app_registry.read();
	let type_info = patch
		.get_represented_type_info()
		.ok_or_else(|| bevyhow!("spread/component patch has no represented type"))?;
	let registration = registry.get(type_info.type_id()).ok_or_else(|| {
		bevyhow!("type `{}` is not registered", type_info.type_path())
	})?;
	let reflect_component =
		registration.data::<ReflectComponent>().ok_or_else(|| {
			bevyhow!(
				"type `{}` is not a registered component",
				type_info.type_path()
			)
		})?;
	// `from_reflect` the partial patch over default, then insert. A `DynamicStruct`
	// carrying only the provided fields fills the rest from the type's default.
	reflect_component.insert(entity, patch, &registry);
	Ok(())
}

// --- value/attribute lowering helpers ----------------------------------------

/// Lower a literal in text/attribute position to a concrete [`Value`].
fn literal_to_value(literal: &DataLiteral) -> Result<Value> {
	match literal {
		DataLiteral::Scalar(value) => Ok(value.clone()),
		DataLiteral::List(items) => items
			.iter()
			.map(literal_to_value)
			.collect::<Result<Vec<_>>>()
			.map(Value::List),
		DataLiteral::Struct(fields) => {
			let mut map = Map::default();
			for (key, value) in fields {
				map.insert(key.clone(), literal_to_value(value)?);
			}
			Ok(Value::Map(map))
		}
		DataLiteral::Enum(named) if matches!(named.fields, NamedFields::Unit) => {
			Ok(Value::Str(named.name.clone().into()))
		}
		DataLiteral::Enum(_) => {
			bevybail!("enum literal with fields is not a plain text value")
		}
		DataLiteral::EntityRef(_) => {
			bevybail!("`$name` entity references are not a plain text value")
		}
	}
}

/// Build a [`FieldRef`] from a `#field=init` reference.
fn field_ref(path: &FieldPath, init: Option<&DataLiteral>) -> Result<FieldRef> {
	let mut field = FieldRef::new(path.clone());
	if let Some(init) = init {
		field = field.with_init(literal_to_value(init)?);
	}
	Ok(field)
}

/// Lower an attribute value to a literal for reflect-patching an uppercase tag.
fn attr_to_literal(value: &AttrValue) -> Result<DataLiteral> {
	match value {
		AttrValue::Flag => Ok(DataLiteral::Scalar(Value::Bool(true))),
		AttrValue::Str(string) => {
			Ok(DataLiteral::Scalar(Value::Str(string.into())))
		}
		AttrValue::Expr(ValueExpr::Literal(literal)) => Ok(literal.clone()),
		// `field=$name` lowers to an entity-reference literal on the patched field.
		AttrValue::Expr(ValueExpr::EntityRef(name)) => {
			Ok(DataLiteral::EntityRef(name.clone()))
		}
		AttrValue::Expr(ValueExpr::FieldRef { .. }) => {
			bevybail!("a `#field` reference is not a component patch value")
		}
		AttrValue::Spread(_) => bevybail!("a spread is not an attribute value"),
	}
}

/// The `bx:<event>` directive names treated as event bindings (as opposed to the
/// structural directives `scope`/`for`/`key`/`slot`/`ref`/`schema`). Core stays
/// event-agnostic; this list only names which directives carry a `verb#field`,
/// not what the events mean (that lives in the [`EventRegistry`] installers).
const BSX_EVENTS: &[&str] = &["click"];

/// Parse a `bx:<event>=verb#field` event binding from its attribute value.
fn parse_event_binding(event: &str, value: &AttrValue) -> Result<EventBinding> {
	let raw = match value {
		AttrValue::Str(string) => string.clone(),
		// an unbraced `bx:click=increment#count` parses as a value expr fallback.
		AttrValue::Expr(ValueExpr::Literal(DataLiteral::Enum(named)))
			if matches!(named.fields, NamedFields::Unit) =>
		{
			named.name.to_string()
		}
		_ => bevybail!("`bx:{event}` expects a `verb#field` binding"),
	};
	let (verb, rest) = raw.split_once('#').ok_or_else(|| {
		bevyhow!("`bx:{event}` must name a field, ie `verb#field`")
	})?;
	let (field_str, init) = match rest.split_once('=') {
		Some((field, init)) => (field, Some(parse_init(init)?)),
		None => (rest, None),
	};
	Ok(EventBinding {
		event: event.into(),
		verb: verb.into(),
		field: FieldPath::new(field_str.split('.').collect::<Vec<_>>()),
		init,
	})
}

/// Parse an event field initializer literal `=init`.
fn parse_init(init: &str) -> Result<Value> {
	if let Ok(int) = init.parse::<i64>() {
		Ok(Value::Int(int))
	} else if let Ok(b) = init.parse::<bool>() {
		Ok(Value::Bool(b))
	} else {
		Ok(Value::Str(init.into()))
	}
}

// --- attribute lookup helpers ------------------------------------------------

/// The string value of a literal-string attribute, if present.
fn string_attr(el: &BsxElement, key: &str) -> Option<String> {
	el.attributes.iter().find_map(|attr| {
		if attr.key != key {
			return None;
		}
		match &attr.value {
			AttrValue::Str(string) => Some(string.clone()),
			AttrValue::Flag => Some(String::new()),
			_ => None,
		}
	})
}

/// The [`SlotChild`] routing marker for an element's `slot`/`bx:slot`, if any.
fn slot_routing(el: &BsxElement) -> Option<SlotChild> {
	el.attributes.iter().find_map(|attr| {
		if attr.key != "slot" && attr.key != "bx:slot" {
			return None;
		}
		match &attr.value {
			AttrValue::Str(name) => Some(SlotChild::named(name.clone())),
			_ => Some(SlotChild::new()),
		}
	})
}

/// Whether a key is a `bx:`/slot directive rather than an HTML attribute.
pub(super) fn is_directive(key: &str) -> bool {
	key.starts_with("bx:") || key == "slot"
}
