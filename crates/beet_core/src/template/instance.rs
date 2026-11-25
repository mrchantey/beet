//! [`Instance`]: another document's scene, instantiated at a use site, and the
//! [`Prop`]s a document takes.
use crate::prelude::*;

/// Instantiate another document's scene here: the target is loaded through its
/// parser and built under this entity, the use site, each use with its own
/// entities, state and references.
///
/// The use site is the props document the target's `@prop:` bindings read
/// ([`PropsDocument`]), and the children authored under it are its slot
/// children, moved into the target's `<slot>`s. Activation lives where
/// documents are read (`beet_scene`), so a binary without it holds the
/// reference as the data it is.
///
/// ```html
/// <Instance document="./Card.bsx" props={{title:"Intro", count:@doc:count}}>
/// 	<p>slotted</p>
/// </Instance>
/// ```
///
/// An instance is a builder: activation consumes it, so a snapshot holds what
/// it built and one not yet activated stays the reference it is.
#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[reflect(Component)]
#[component(on_insert = hook_ext::component_hook(|instance: &Instance| {
	let props = instance.props.clone();
	move |entity: &mut EntityCommands| {
		entity.insert((Document::new(Value::Map(props)), PropsDocument));
	}
}))]
pub struct Instance {
	/// The document to instantiate.
	pub document: DocPath,
	/// The target's named arguments.
	#[reflect(default)]
	pub props: Map,
}

impl Instance {
	/// An instance of the document at `document` taking no props.
	pub fn new(document: impl Into<DocPath>) -> Self {
		Self {
			document: document.into(),
			props: default(),
		}
	}

	/// This instance with `value` passed as the prop `name`.
	pub fn with_prop(
		mut self,
		name: impl Into<SmolStr>,
		value: impl Into<Value>,
	) -> Self {
		self.props.insert(name.into(), value.into());
		self
	}
}

/// One named argument a document takes, an entity of its own in the document
/// declaring it. The body reads it with `@prop:name`, which resolves to the use
/// site, and a use passing no such prop reads its default.
///
/// ```html
/// <Prop name="title" kind="string" description="The card's heading"/>
/// <Prop name="count" kind="u32" default={3}/>
/// ```
#[derive(Debug, Default, Clone, PartialEq, Component, Reflect)]
#[reflect(Component, Default)]
pub struct Prop {
	/// The name a use passes it by, and the body binds it by.
	pub name: SmolStr,
	/// The kind of value it takes, in the schema vocabulary, ie `string`.
	pub kind: SmolStr,
	/// What a use passing none reads, null when absent.
	pub default: Option<Value>,
	/// What it is for.
	pub description: Option<SmolStr>,
}
