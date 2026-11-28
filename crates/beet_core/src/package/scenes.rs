//! What a compiled scene carries and what its package record says of it:
//! [`Imports`] and [`Props`], the scene meta; [`Scenes`] and [`LoadSets`], the
//! package meta; and the use site's [`LoadRoot`] and [`Preload`].
use crate::prelude::*;
use alloc::collections::BTreeMap;

/// `org.beet.core#imports`, on scene meta: every path this document references
/// through a [`DocPath`] in any component, package absolute with its
/// extension, and how it reaches each. Which of them are documents is the
/// package's business ([`LoadSets`]), so the meta stays a function of the
/// document alone. Written by the compiler, which finds every reference by its
/// type.
#[derive(
	Debug,
	Default,
	Clone,
	PartialEq,
	Eq,
	Component,
	Reflect,
	LexiconDef,
	Serialize,
	Deserialize,
)]
#[reflect(Component, Default, LexiconDef, Serialize, Deserialize)]
pub struct Imports(pub BTreeMap<FwdPath, ImportEdge>);

impl Imports {
	/// Record a reference to `path`, keeping the eagerest edge reaching it.
	pub fn insert(&mut self, path: FwdPath, edge: ImportEdge) {
		let entry = self.0.entry(path).or_insert(edge);
		entry.kind = entry.kind.min(edge.kind);
		entry.instance |= edge.instance;
	}
}

/// How a document reaches one of its [`Imports`].
#[derive(
	Debug, Default, Clone, Copy, PartialEq, Eq, Reflect, Serialize, Deserialize,
)]
#[reflect(Default, Serialize, Deserialize)]
pub struct ImportEdge {
	/// The eagerest load edge reaching the path.
	pub kind: ImportKind,
	/// Whether an [`Instance`] names it, the edges a cycle is found through:
	/// activation is eager once a set has loaded, so a cycle of instances
	/// would never finish building.
	#[serde(default)]
	pub instance: bool,
}

/// How a use reaches its target, set by the components beside the
/// [`Instance`] on the use site; a reference held by any other component is
/// [`Eager`](Self::Eager). Ordered eagerest first.
#[derive(
	Debug,
	Default,
	Clone,
	Copy,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Reflect,
	Serialize,
	Deserialize,
)]
#[reflect(Default, Serialize, Deserialize)]
pub enum ImportKind {
	/// In this document's load set.
	#[default]
	Eager,
	/// [`LoadRoot`] with [`Preload`]: the target's own set, fetched in the
	/// parent's round trip.
	Preload,
	/// [`LoadRoot`]: the target's own set, fetched when the use activates.
	Lazy,
}

/// On a use site beside its [`Instance`]: this use is lazy, its target fetched
/// with its own load set when the use activates. An edge, never the target's
/// meta, so one document can be a route in one place and inline in another.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Component, Reflect)]
#[reflect(Component, Default)]
pub struct LoadRoot;

/// Beside [`LoadRoot`]: fetch the target's set in the parent's round trip.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Component, Reflect)]
#[reflect(Component, Default)]
pub struct Preload;

/// `org.beet.core#props`, on scene meta: the document's named arguments,
/// hoisted from its [`Prop`] entities, which a use's props are checked against
/// at compile.
#[derive(
	Debug,
	Default,
	Clone,
	PartialEq,
	Eq,
	Component,
	Reflect,
	LexiconDef,
	Serialize,
	Deserialize,
)]
#[reflect(Component, Default, LexiconDef, Serialize, Deserialize)]
pub struct Props(pub BTreeMap<SmolStr, PropSchema>);

/// One named argument as [`Props`] holds it.
#[derive(
	Debug, Default, Clone, PartialEq, Eq, Reflect, Serialize, Deserialize,
)]
#[reflect(Default, Serialize, Deserialize)]
pub struct PropSchema {
	/// The kind of value it takes, ie `string` ([`PropSchema::admits`]).
	pub kind: SmolStr,
	/// What a use passing none reads, required when absent.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub default: Option<Value>,
	/// What it is for.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub description: Option<SmolStr>,
}

impl PropSchema {
	/// Whether `value` is of this prop's kind: `string`, `bool`, an integer
	/// (`u32`, `i64`, `int`, ..), a number (`f32`, `float`, ..), `list` or
	/// `map`, while an empty kind, `any` or one this does not know admits
	/// anything, its check the type's own when the value reflects.
	pub fn admits(&self, value: &Value) -> bool {
		let kind = self.kind.as_str();
		// a kind this vocabulary does not name is checked by its type
		if !Self::is_known(kind) {
			return true;
		}
		match value {
			Value::Null => true,
			Value::Str(_) => matches!(kind, "string" | "str"),
			Value::Bool(_) => matches!(kind, "bool" | "boolean"),
			Value::Int(int) => {
				(Self::is_integer(kind)
					&& (*int >= 0 || !kind.starts_with('u')))
					|| Self::is_number(kind)
			}
			Value::Uint(_) => Self::is_integer(kind) || Self::is_number(kind),
			Value::Float(_) => Self::is_number(kind),
			Value::List(_) => matches!(kind, "list" | "array"),
			Value::Map(_) => matches!(kind, "map" | "object"),
			Value::Bytes(_) => kind == "bytes",
		}
	}

	fn is_integer(kind: &str) -> bool {
		matches!(
			kind,
			"u8" | "u16"
				| "u32" | "u64"
				| "usize" | "i8"
				| "i16" | "i32"
				| "i64" | "isize"
				| "int" | "integer"
		)
	}

	fn is_number(kind: &str) -> bool {
		matches!(kind, "f32" | "f64" | "float" | "number")
	}

	fn is_known(kind: &str) -> bool {
		Self::is_integer(kind)
			|| Self::is_number(kind)
			|| matches!(
				kind,
				"string"
					| "str" | "bool"
					| "boolean" | "list"
					| "array" | "map"
					| "object" | "bytes"
			)
	}
}

impl SceneMeta for Prop {
	fn apply(cx: &mut SceneMetaContext) -> Result {
		let prop = cx.get::<Self>()?.clone();
		let error =
			cx.error(format!("the prop `{}` is declared twice", prop.name));
		let schema = PropSchema {
			kind: prop.kind,
			default: prop.default,
			description: prop.description,
		};
		match cx.meta_mut::<Props>().0.insert(prop.name, schema) {
			Some(_) => Err(error),
			None => Ok(()),
		}
	}
}

/// `org.beet.core#scenes`, on a package record: every document of the
/// package, sub packages excluded, keyed by its path in `Files`.
#[derive(
	Debug,
	Default,
	Clone,
	PartialEq,
	Eq,
	Component,
	Reflect,
	LexiconDef,
	Serialize,
	Deserialize,
)]
#[reflect(Component, Default, LexiconDef, Serialize, Deserialize)]
pub struct Scenes(pub BTreeMap<FwdPath, SceneRef>);

/// A document of a package record or a release: its content-keyed scene
/// record, and the heads of the Automerge document it was compiled from,
/// empty when it was not one. Provenance lives here, never on the scene
/// record, which is pure content shared by any number of branches and
/// releases.
#[derive(
	Debug, Clone, PartialEq, Eq, Reflect, Serialize, Deserialize,
)]
#[reflect(Serialize, Deserialize)]
pub struct SceneRef {
	/// The scene record as it was, what a loader verifies a fetch against.
	pub record: StrongRef,
	/// The Automerge heads it was compiled at.
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub heads: Vec<ChangeHash>,
}

/// `org.beet.core#loadSets`, on a package record: per document, everything
/// one round trip fetches when that document is a load root, its closure
/// through eager and preload [`Imports`] stopping at a lazy one, whose target
/// fetches its own set when the use activates.
#[derive(
	Debug,
	Default,
	Clone,
	PartialEq,
	Eq,
	Component,
	Reflect,
	LexiconDef,
	Serialize,
	Deserialize,
)]
#[reflect(Component, Default, LexiconDef, Serialize, Deserialize)]
pub struct LoadSets(pub BTreeMap<FwdPath, LoadSet>);

/// One document's load set.
#[derive(
	Debug, Default, Clone, PartialEq, Eq, Reflect, Serialize, Deserialize,
)]
#[reflect(Default, Serialize, Deserialize)]
pub struct LoadSet {
	/// The scene body blobs, the document's own first.
	pub scenes: Vec<Cid>,
	/// The plain files reached.
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub blobs: Vec<Cid>,
	/// The union of the members' [`RequireCfg`], checked against a runtime's
	/// registration before any body is fetched.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub requires: Option<SmolStr>,
}
