//! `@` bindings: each to the sync components of its source.
use super::literal::*;
use super::lowering::*;
use crate::body::KeySentinels;
use beet_core::prelude::*;

/// The reserved selector names, targeting well-known entities rather than a
/// `bx:ref`, which may not shadow them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ReservedRef {
	/// The document's root.
	BuildRoot,
	/// The document's root, a document being lowered alone.
	SnippetRoot,
	/// The nearest self-or-ancestor entity carrying a `PageRoot` component,
	/// resolved lazily each sync pass: the render tree may not exist or be
	/// attached when the body spawns (layouts build detached, per request).
	PageRoot,
	/// The nearest self-or-ancestor entity carrying a `Router` component,
	/// resolved lazily each sync pass.
	Router,
}

impl ReservedRef {
	/// Every reserved selector name.
	pub const NAMES: &[&str] =
		&["BuildRoot", "SnippetRoot", "PageRoot", "Router"];

	/// Classify a selector name, `None` for a user `bx:ref` name.
	pub fn parse(name: &str) -> Option<Self> {
		match name {
			"BuildRoot" => Some(Self::BuildRoot),
			"SnippetRoot" => Some(Self::SnippetRoot),
			"PageRoot" => Some(Self::PageRoot),
			"Router" => Some(Self::Router),
			_ => None,
		}
	}
}

/// What an `@comp` binding binds, an entity of the body by key.
#[derive(Debug, Clone, PartialEq)]
enum CompTarget {
	This,
	Entity(EntityKey),
	Reserved(SmolStr),
}

impl Lowering<'_> {
	/// Lower `binding` onto the entity keyed `key`, an `@comp` with no
	/// selector binding `element` where given and the entity itself
	/// otherwise.
	pub fn binding(
		&mut self,
		binding: &BindingExpr,
		key: &EntityKey,
		element: Option<&EntityKey>,
	) -> Result {
		match binding.source {
			BindingSource::Doc => {
				let mut field = FieldRef::new(binding.field_path.clone());
				if let Some(init) = &binding.init {
					field = field.with_init(literal_to_value(init)?);
				}
				self.insert(key, &field)
			}
			BindingSource::Prop => self.insert(
				key,
				&FieldRef::new(binding.field_path.clone())
					.with_document(DocumentPath::Props),
			),
			BindingSource::Res => {
				self.seed_value(key)?;
				self.insert(key, &Self::resource_ref(binding))
			}
			BindingSource::Comp => {
				self.seed_value(key)?;
				let target = match (&binding.selector, element) {
					(Some(name), _) => self.selector(name)?,
					(None, Some(element)) => {
						CompTarget::Entity(element.clone())
					}
					(None, None) => CompTarget::This,
				};
				self.insert_linked(key, |keys| {
					Self::component_ref(binding, target, keys)
				})
			}
		}
	}

	/// Lower a binding valued prop of the use site keyed `site`: an entity
	/// related to it mirroring the source into the props document's field
	/// `prop`, so it never renders and despawns with the use.
	pub fn prop_binding(
		&mut self,
		site: &EntityKey,
		prop: &str,
		binding: &BindingExpr,
	) -> Result {
		let key = self.attribute(site, prop, default(), &default())?;
		self.seed_value(&key)?;
		self.insert_linked(&key, |keys| {
			FieldRef::new(prop)
				.with_document(DocumentPath::Entity(keys.entity(site.clone())))
		})?;
		// the source resolves from the use site, the binding entity being
		// outside the hierarchy; an `@comp` is a component beside the instance
		match binding.source {
			BindingSource::Doc | BindingSource::Prop => {
				let document = match binding.source {
					BindingSource::Prop => DocumentPath::Props,
					_ => DocumentPath::Ancestor,
				};
				self.insert_linked(&key, |keys| {
					SourceFieldRef::new(binding.field_path.clone())
						.with_document(document)
						.with_subject(keys.entity(site.clone()))
				})
			}
			BindingSource::Res => {
				self.insert(&key, &Self::resource_ref(binding))
			}
			BindingSource::Comp => {
				let target = match &binding.selector {
					Some(name) => self.selector(name)?,
					None => CompTarget::Entity(site.clone()),
				};
				self.insert_linked(&key, |keys| {
					Self::component_ref(binding, target, keys)
				})
			}
		}
	}

	/// The resource field an `@res` binding names.
	fn resource_ref(binding: &BindingExpr) -> ResourceFieldRef {
		ResourceFieldRef::new(
			binding.type_path.clone().unwrap_or_default(),
			binding.field_path.to_string(),
		)
	}

	/// The component field an `@comp` binding names, on `target`.
	fn component_ref(
		binding: &BindingExpr,
		target: CompTarget,
		keys: &mut KeySentinels,
	) -> ReflectFieldRef {
		let mut field = ReflectFieldRef::new(
			binding.type_path.clone().unwrap_or_default(),
			binding.field_path.to_string(),
		);
		field.target = match target {
			CompTarget::This => BindingTarget::This,
			CompTarget::Entity(key) => BindingTarget::Entity(keys.entity(key)),
			CompTarget::Reserved(name) => BindingTarget::Reserved(name),
		};
		field
	}

	/// What an `@entity:name::` selector targets: the entity `name` keys, or
	/// a reserved one the sync pass resolves.
	fn selector(&self, name: &SmolStr) -> Result<CompTarget> {
		match ReservedRef::parse(name) {
			Some(ReservedRef::PageRoot | ReservedRef::Router) => {
				CompTarget::Reserved(name.clone()).xok()
			}
			_ => self.reference(name).map(CompTarget::Entity),
		}
	}

	/// Seed a default [`Value`] for a binding's sync to fill, keeping one
	/// already there.
	fn seed_value(&mut self, key: &EntityKey) -> Result {
		match self.entity(key).contains_typed::<Value>() {
			true => Ok(()),
			false => self.insert(key, &Value::default()),
		}
	}
}
