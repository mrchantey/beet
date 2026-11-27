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
	/// A rust `#[template]` by its type path, a document writing it short, ie
	/// `SiteLayout`, registered with `app.install_types::<SiteLayout>()`.
	Template(ReflectName),
}

impl TemplateSource {
	/// The rust template `T`, by its type path.
	pub fn of<T: BuildTemplate + TypePath>() -> Self {
		Self::template(ReflectName::of::<T>())
	}

	/// The rust template named `template`. Prefer [`of`](Self::of), which
	/// derives the name from the type.
	pub fn template(template: impl Into<ReflectName>) -> Self {
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
					.with_state::<PackageFilesQuery, _>(|entity, packages| {
						packages.get(entity)
					})
					.await??;
				let prefab = PrefabCache::load(
					&entity.world(),
					&package.resolve(&document),
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
	/// A document and the package it is of.
	Document(PackageFiles, Arc<InstancePrefab>),
	/// A rust template by its type path.
	Template(ReflectName),
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
			Self::Document(package, prefab) => {
				entity
					.insert(package.clone())
					.insert_template(InstanceTemplate::new(prefab.clone()))?
			}
			Self::Template(name) => {
				let body = BsxParser::invoking(entity.world(), name)?;
				entity.insert_template(body)?
			}
		};
		Ok(())
	}
}
