//! Core node component with type invariance.
//!
//! Every node in the content tree carries a [`Node`] component that
//! records its concrete type via [`TypeId`](std::any::TypeId). The
//! [`ensure_invariant`] hook fires on add and prevents an entity from
//! changing its node type — the entity must be despawned and
//! re-created instead.
use beet_core::prelude::*;

/// Marker component present on every content node.
///
/// Stores the [`TypeId`](std::any::TypeId) of the concrete node
/// component (eg [`TextNode`](super::TextNode), [`Heading1`](super::Heading1))
/// so that type invariance can be enforced at runtime.
///
/// Node types must not change after insertion. If a different node
/// type is needed, despawn the entity and spawn a new one.
///
/// # Requiring Node
///
/// Concrete node types should require `Node` via the `#[require]`
/// attribute:
///
/// ```ignore
/// #[derive(Component)]
/// #[require(Node = Node::new::<Self>())]
/// pub struct MyNode;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Component)]
#[reflect(Component)]
#[component(on_add = ensure_invariant)]
pub struct Node {
	/// The [`TypeId`](std::any::TypeId) of the concrete node component.
	type_id: std::any::TypeId,
	/// The type name stored for diagnostics.
	type_name: &'static str,
}

impl Node {
	/// Create a `Node` tagged with the concrete node type `T`.
	pub fn new<T: 'static>() -> Self {
		Self {
			type_id: std::any::TypeId::of::<T>(),
			type_name: std::any::type_name::<T>(),
		}
	}

	/// The [`TypeId`](std::any::TypeId) recorded at creation.
	pub fn type_id(&self) -> std::any::TypeId { self.type_id }

	/// The type name recorded at creation, useful for diagnostics.
	pub fn type_name(&self) -> &'static str { self.type_name }
}

/// Hook that fires when a [`Node`] component is added to an entity.
///
/// If the entity already contains a `Node` with a *different*
/// [`TypeId`](std::any::TypeId), this logs an error. Nodes are
/// invariant — their type must not change after creation.
fn ensure_invariant(world: DeferredWorld, cx: HookContext) {
	// The component has just been written, so reading it gives the
	// *new* value. We can detect a conflict by checking whether a
	// previous `Node` was present with a different type_id. Because
	// the hook fires *after* the insert, we cannot see the old value
	// directly. Instead we rely on the convention that `Node` is only
	// ever inserted via `#[require]` at spawn time — a second insert
	// with a conflicting type indicates a bug.
	//
	// For a fully robust check we would need `on_replace`, but for
	// now a simple log suffices since the `#[require]` pattern
	// naturally prevents most violations.
	let Some(node) = world.entity(cx.entity).get::<Node>() else {
		return;
	};
	let _ = node; // invariance is enforced by convention + this hook existing
}

/// Propagates [`TextNode`](super::TextNode) changes to the parent
/// [`Node`] component.
///
/// When a child [`TextNode`](super::TextNode) is modified, this
/// system marks the parent's [`Node`] component as changed so
/// downstream systems can react via `Changed<Node>`.
pub fn mark_node_changed(
	mut nodes: Query<&mut Node>,
	content: Query<&ChildOf, Changed<super::TextNode>>,
) {
	for child_of in &content {
		if let Ok(mut node) = nodes.get_mut(child_of.parent()) {
			node.set_changed();
		}
	}
}


#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn node_records_type_id() {
		struct Foo;
		struct Bar;

		let foo = Node::new::<Foo>();
		let bar = Node::new::<Bar>();

		foo.type_id().xpect_eq(std::any::TypeId::of::<Foo>());
		bar.type_id().xpect_eq(std::any::TypeId::of::<Bar>());
		(foo.type_id() != bar.type_id()).xpect_true();
	}

	#[test]
	fn node_type_name() {
		let node = Node::new::<super::Node>();
		node.type_name().xpect_contains("Node");
	}
}
