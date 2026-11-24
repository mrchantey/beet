//! [`TemplateSource`], what a component naming a document or a rust template
//! builds per request.
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_scene::prelude::*;
use std::sync::Arc;

/// What a component naming a document or a registered rust template builds,
/// ie a [`Layout`](crate::prelude::Layout)'s chrome: loaded off the world, then built as many times as
/// its holder likes.
#[derive(Clone)]
pub enum TemplateSource {
	/// A document and the package it is of.
	Document(PackageStore, Arc<InstancePrefab>),
	/// A rust template by its short or full type path.
	Template(SmolStr),
}

impl TemplateSource {
	/// Load what `document` or `template`, one of the two, names for `entity`,
	/// the document resolved in `entity`'s package.
	///
	/// ## Errors
	/// Errors when neither or both are named, or the document does not load.
	pub async fn load(
		entity: &AsyncEntity,
		document: Option<DocPath>,
		template: Option<SmolStr>,
	) -> Result<Self> {
		match (document, template) {
			(Some(document), None) => {
				let package = entity
					.with_state::<PackageQuery, _>(|entity, packages| {
						packages.get(entity)
					})
					.await??;
				let prefab = PrefabCache::load(
					&entity.world(),
					&package,
					&document.rel_path(),
				)
				.await?;
				Self::Document(package, prefab).xok()
			}
			(None, Some(template)) => Self::Template(template).xok(),
			_ => bevybail!(
				"name a `document`, ie `templates/Layout.bsx`, or a registered \
				 rust `template`, one of the two"
			),
		}
	}

	/// The name it was declared by.
	pub fn name(&self) -> &str {
		match self {
			Self::Document(_, prefab) => prefab.path().as_str(),
			Self::Template(name) => name,
		}
	}

	/// Build under `entity`, which becomes its build root: the document's or
	/// the template's own root is always its child.
	pub fn build(&self, entity: &mut EntityWorldMut) -> Result {
		match self {
			Self::Document(package, prefab) => {
				entity
					.insert(package.clone())
					.insert_template(InstanceTemplate::new(prefab.clone()))?
			}
			Self::Template(name) => {
				let template = SceneBody::invoking(entity.world(), name)?
					.reflect(entity.world())?;
				entity.insert_template(template)?
			}
		};
		Ok(())
	}
}
