//! [`TemplateSource`], what a component naming a document or a rust template
//! names, and [`LoadedTemplate`], what it builds per request.
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_scene::prelude::*;
use std::sync::Arc;

/// A document or a registered rust template, as a component building either
/// names one, ie a [`Layout`](crate::prelude::Layout)'s chrome:
///
/// ```bsx
/// <Router {Layout(Document("templates/Layout.bsx"))}/>
/// <Router {Layout(Template("AppShell"))}/>
/// ```
#[derive(Debug, Clone, PartialEq, Reflect)]
pub enum TemplateSource {
	/// A document of the package the declaration is in, ie
	/// `templates/Layout.bsx`.
	Document(DocPath),
	/// A rust `#[template]` by its short or full type path, ie `SiteLayout`,
	/// registered with `app.install_types::<SiteLayout>()`.
	Template(SmolStr),
}

impl TemplateSource {
	/// The rust template `T`, named by its short type path, the name
	/// `app.install_types::<T>()` registers it under.
	pub fn of<T: BuildTemplate>() -> Self {
		Self::template(TypeName::of::<T>().short())
	}

	/// The rust template named `template`. Prefer [`of`](Self::of), which
	/// derives the name from the type.
	pub fn template(template: impl Into<SmolStr>) -> Self {
		Self::Template(template.into())
	}

	/// The document at `document` of the declaration's package.
	pub fn document(document: impl Into<RelPath>) -> Self {
		Self::Document(document.into().into())
	}

	/// Load what this names for `entity`, a document resolved in `entity`'s
	/// package.
	///
	/// ## Errors
	/// Errors when the document does not load.
	pub async fn load(self, entity: &AsyncEntity) -> Result<LoadedTemplate> {
		match self {
			Self::Document(document) => {
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
				LoadedTemplate::Document(package, prefab).xok()
			}
			Self::Template(template) => {
				LoadedTemplate::Template(template).xok()
			}
		}
	}
}

/// A [`TemplateSource`] loaded off the world, then built as many times as its
/// holder likes.
#[derive(Clone)]
pub enum LoadedTemplate {
	/// A document and the store of the package it is of.
	Document(BlobStore, Arc<InstancePrefab>),
	/// A rust template by its short or full type path.
	Template(SmolStr),
}

impl LoadedTemplate {
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
			Self::Document(package, prefab) => entity
				.insert(PackageStore::new(package.clone()))
				.insert_template(InstanceTemplate::new(prefab.clone()))?,
			Self::Template(name) => {
				let template = SceneBody::invoking(entity.world(), name)?
					.reflect(entity.world())?;
				entity.insert_template(template)?
			}
		};
		Ok(())
	}
}
