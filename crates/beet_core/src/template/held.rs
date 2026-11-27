//! Templates held unbuilt: content a component carries and builds later, as
//! many times as it likes.
//!
//! A tag's children normally build where the tag sits. A component that
//! decides when its content builds (a server island resolving per request, a
//! fallback shown only in one render) holds it as a [`HeldTemplate`]
//! instead. In markup it implements [`HoldChildren`] and registers
//! `#[reflect(HoldChildren)]`, and its children lower to a scene body in the
//! field their slot names, so a scene holding it is data; in `rsx!` the
//! content is a closure, `content={|| rsx!{ .. }}`.
use crate::prelude::*;
use alloc::sync::Arc;
use bevy::ecs::template::Template;
#[cfg(feature = "serde")]
use bevy::ecs::template::TemplateContext;
use bevy::reflect::CreateTypeData;

/// The erased build a [`HeldTemplate`] holds.
type BuildFn = Arc<dyn Fn(&mut EntityWorldMut) -> Result + Send + Sync>;

/// A template held unbuilt, built into an entity on demand as many times as its
/// holder likes. The default holds nothing and builds nothing.
///
/// Held as a scene body it serializes as one, and a document's reference in it
/// resolves from the document it was written in. Held as code it is derived
/// ui, which no scene can hold, and writes nothing.
///
/// ```
/// # use beet_core::prelude::*;
/// let content = HeldTemplate::new(|| Snippet::from_bundle(Element::new("p")));
/// let mut world = World::new();
/// let entity = world.spawn_empty().id();
/// content.build(&mut world.entity_mut(entity)).unwrap();
/// world.entity(entity).contains::<Element>().xpect_true();
/// ```
#[derive(Clone, Default, Reflect)]
#[reflect(opaque, Default)]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
pub struct HeldTemplate(HeldKind);

/// What is held.
#[derive(Clone, Default)]
enum HeldKind {
	/// Nothing.
	#[default]
	Empty,
	/// Code, from Rust.
	Build(BuildFn),
	/// A scene body, from a document, and the package path it was read from.
	#[cfg(feature = "serde")]
	Body(Arc<SceneBody>, Option<FwdPath>),
}

impl HeldTemplate {
	/// Build what `func` returns each time this builds, ie
	/// `HeldTemplate::new(|| rsx!{ <VisitCount/> })`.
	pub fn new<T: Template<Output = ()>>(
		func: impl 'static + Send + Sync + Fn() -> T,
	) -> Self {
		Self(HeldKind::Build(Arc::new(
			move |entity: &mut EntityWorldMut| {
				entity.insert_template(func()).map(|_| ())
			},
		)))
	}

	/// Build `body` each time this builds.
	#[cfg(feature = "serde")]
	pub fn from_body(body: SceneBody) -> Self {
		Self(match body.entities.is_empty() {
			true => HeldKind::Empty,
			false => HeldKind::Body(Arc::new(body), None),
		})
	}

	/// Resolve the references of a held body from the document at `document`.
	#[cfg(feature = "serde")]
	pub fn set_document(&mut self, document: &FwdPath) {
		if let HeldKind::Body(_, path) = &mut self.0 {
			*path = Some(document.clone());
		}
	}

	/// The documents a held body instantiates, package absolute once its own
	/// document is known.
	///
	/// # Errors
	/// Errors when a reference climbs above the package.
	#[cfg(feature = "serde")]
	pub fn instances(&self) -> Result<Vec<FwdPath>> {
		let HeldKind::Body(body, path) = &self.0 else {
			return Vec::new().xok();
		};
		body.instances()
			.map(|document| match path {
				Some(path) => document.resolve(path),
				None => document.fwd_path(),
			})
			.collect()
	}

	/// Whether this holds nothing, so building it builds nothing.
	pub fn is_empty(&self) -> bool { matches!(self.0, HeldKind::Empty) }

	/// Build the held template into `entity`, which becomes its template root,
	/// so a deferred dependency it parks lands on `entity`'s
	/// [`TemplatePending`]. Holding nothing builds nothing.
	pub fn build(&self, entity: &mut EntityWorldMut) -> Result {
		match &self.0 {
			HeldKind::Empty => Ok(()),
			HeldKind::Build(func) => func(entity),
			#[cfg(feature = "serde")]
			HeldKind::Body(body, path) => entity
				.insert_template(HeldBody {
					body: body.clone(),
					path: path.clone(),
				})
				.map(|_| ()),
		}
	}
}

/// A held body as the document it was written in.
#[cfg(feature = "serde")]
#[derive(Clone)]
struct HeldBody {
	body: Arc<SceneBody>,
	path: Option<FwdPath>,
}

#[cfg(feature = "serde")]
impl Template for HeldBody {
	type Output = ();

	fn build_template(&self, cx: &mut TemplateContext) -> Result {
		SceneBodyBuilder::build(&self.body, self.path.as_ref(), cx)
	}

	fn clone_template(&self) -> Self { self.clone() }
}

#[cfg(feature = "serde")]
subtree_template!(HeldBody);

impl core::fmt::Debug for HeldTemplate {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self.0 {
			HeldKind::Empty => f.write_str("HeldTemplate(empty)"),
			_ => f.write_str("HeldTemplate(..)"),
		}
	}
}

/// A held body as the body it is; code is not data, so writes an empty one.
#[cfg(feature = "serde")]
impl Serialize for HeldTemplate {
	fn serialize<S: serde::Serializer>(
		&self,
		serializer: S,
	) -> core::result::Result<S::Ok, S::Error> {
		match &self.0 {
			HeldKind::Body(body, _) => body.serialize(serializer),
			_ => SceneBody::default().serialize(serializer),
		}
	}
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for HeldTemplate {
	fn deserialize<D: serde::Deserializer<'de>>(
		deserializer: D,
	) -> core::result::Result<Self, D::Error> {
		SceneBody::deserialize(deserializer).map(Self::from_body)
	}
}

/// An `rsx!` prop takes a closure where its field is a [`HeldTemplate`],
/// ie `<ServerIsland name="visits" content={|| rsx!{ <VisitCount/> }}/>`.
impl<F, T> IntoProp<HeldTemplate> for F
where
	F: 'static + Send + Sync + Fn() -> T,
	T: Template<Output = ()>,
{
	fn into_prop(self) -> HeldTemplate { HeldTemplate::new(self) }
}

/// A component holding its authored children unbuilt rather than building
/// them where it sits. Register `#[reflect(HoldChildren)]` and each child
/// lowers into the [`HeldTemplate`] field its `slot` names.
pub trait HoldChildren {
	/// The slots it takes, each with the field holding its children: the
	/// default slot `None`, ie `[(None, "content"), (Some("fallback"), "fallback")]`.
	const SLOTS: &'static [(Option<&'static str>, &'static str)];
}

/// Reflect type data for a [`HoldChildren`] component, so a lowering hands
/// children to a tag known only by name.
#[derive(Clone)]
pub struct ReflectHoldChildren {
	slots: &'static [(Option<&'static str>, &'static str)],
}

impl ReflectHoldChildren {
	/// The field holding the children routed to `slot`, `None` being those
	/// routed nowhere.
	pub fn field(&self, slot: Option<&str>) -> Option<&'static str> {
		self.slots
			.iter()
			.find(|(name, _)| *name == slot)
			.map(|(_, field)| *field)
	}

	/// The slots it takes, for the error naming a slot it does not.
	pub fn describe(&self) -> String {
		self.slots
			.iter()
			.map(|(slot, field)| match slot {
				Some(slot) => {
					format!("a `slot=\"{slot}\"` child is its {field}")
				}
				None => format!("its children are its {field}"),
			})
			.collect::<Vec<_>>()
			.join(", ")
	}
}

impl<T: HoldChildren> CreateTypeData<T> for ReflectHoldChildren {
	fn create_type_data(_input: ()) -> Self { Self { slots: T::SLOTS } }
}
