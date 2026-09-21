//! The registered set of declarations that act before the entry builds.
//!
//! Four declarations must act before the tree exists: `<RepoRoot>` widens the
//! store before it is built, `<TemplateDir>` registers before the entry parses,
//! `<RequireCfg>` fires even when the tree cannot build, and `<Secrets>` sets
//! the environment before any constructor reads it. None has a meaning after
//! the build that differs, so the set is a registry of TYPES rather than a
//! per-element annotation an author could forget: a registered type at the
//! entry's top level, with string attributes and no `bx:cfg`, acts before the
//! build, and nothing else does.

use beet_core::prelude::*;
use beet_net::prelude::*;
use bevy::reflect::FromReflect;
use bevy::reflect::GetTypeRegistration;
use bevy::reflect::Typed;
use core::any::TypeId;
use std::sync::Arc;

/// A declaration that acts before its entry builds.
///
/// [`EntryPrescan`] constructs one from the raw element by the same
/// string-attribute coercion a request param uses
/// ([`MultiMapReflectExt::apply_reflect`] over the type's default), so a
/// declaration authors here exactly as in the build: `<Secrets label=".."
/// path=".."/>`. An element carrying a spread is not collected (the spread
/// names something the build resolves, ie another store), a `bx:cfg` subtree
/// is left to the build, and a non-literal attribute is skipped.
///
/// Registered with [`AppRegisterPrescanExt::register_prescan`], in the order
/// its [`preload`](Self::preload) should run.
pub trait Prescan: Default + Clone + FromReflect + Typed {
	/// One line of what the declaration does before the build, listed by
	/// `--help` under "before the build".
	fn describe() -> &'static str;

	/// The pre-build effect, run against the rebased repo store for every
	/// collected declaration, types in registration order and declarations
	/// in document order. A declaration resolution or the build reads for
	/// itself (`<RepoRoot>`, `<TemplateDir>`, `<RequireCfg>`) leaves the
	/// default no-op; an error fails the launch, so a declaration whose
	/// failure is tolerable (`<Secrets>` on a box with no identity) warns and
	/// answers `Ok`.
	fn preload(&self, repo_store: &BlobStore) -> SendBoxedFuture<Result> {
		let _ = repo_store;
		Box::pin(async { Ok(()) })
	}
}

/// The registered [`Prescan`] set, in registration order, which is the order
/// their preloads run in.
#[derive(Default, Clone, Resource)]
pub struct PrescanRegistry {
	registrations: Vec<PrescanRegistration>,
}

impl PrescanRegistry {
	/// Register `T`, appending it to the set; a type already registered keeps
	/// its position.
	pub fn register<T: Prescan>(&mut self) -> &mut Self {
		if !self.contains::<T>() {
			self.registrations.push(PrescanRegistration::new::<T>());
		}
		self
	}

	/// Whether `T` is registered.
	pub fn contains<T: Prescan>(&self) -> bool {
		self.registrations
			.iter()
			.any(|registration| registration.type_id == TypeId::of::<T>())
	}

	/// The registration authored as `tag`, ie `Secrets` for `<Secrets/>`.
	fn by_tag(&self, tag: &str) -> Option<&PrescanRegistration> {
		self.registrations
			.iter()
			.find(|registration| registration.tag == tag)
	}

	/// Every registered type as `(tag, description)`, in registration order,
	/// for `--help`.
	pub fn describe(&self) -> Vec<(&'static str, &'static str)> {
		self.registrations
			.iter()
			.map(|registration| (registration.tag, registration.describe))
			.collect()
	}

	/// Run every collected declaration's [`Prescan::preload`] against
	/// `repo_store`: types in registration order, declarations in document
	/// order, the first error ending the run.
	pub async fn preload(
		&self,
		prescan: &EntryPrescan,
		repo_store: &BlobStore,
	) -> Result {
		for registration in &self.registrations {
			for declaration in prescan.declarations_of(registration.type_id) {
				(registration.preload)(declaration.as_ref(), repo_store)
					.await
					.map_err(|err| {
						bevyhow!("<{}> preload: {err}", registration.tag)
					})?;
			}
		}
		Ok(())
	}
}

/// One registered type: how a raw element constructs it and what it does.
#[derive(Clone)]
struct PrescanRegistration {
	type_id: TypeId,
	/// The tag it is authored as, its short type path.
	tag: &'static str,
	describe: &'static str,
	/// Construct the declaration from an element's string attributes.
	collect: fn(&BsxElement) -> Result<Arc<dyn Reflect>>,
	/// The erased [`Prescan::preload`], over a value `collect` produced.
	preload: fn(&dyn Reflect, &BlobStore) -> SendBoxedFuture<Result>,
}

impl PrescanRegistration {
	fn new<T: Prescan>() -> Self {
		Self {
			type_id: TypeId::of::<T>(),
			tag: T::type_info().type_path_table().short_path(),
			describe: T::describe(),
			collect: Self::collect::<T>,
			preload: Self::preload::<T>,
		}
	}

	/// `T` over its default, every string attribute applied to the field it
	/// names and a bare flag (`<Warm eager/>`) as a `bool`'s `true`, exactly
	/// as a request param; a non-literal attribute and one naming no field
	/// are skipped.
	fn collect<T: Prescan>(element: &BsxElement) -> Result<Arc<dyn Reflect>> {
		let mut params = MultiMap::<SmolStr, SmolStr>::default();
		for attr in &element.attributes {
			let key = SmolStr::from(attr.key.as_str());
			match &attr.value {
				AttrValue::Str(text) => {
					params.insert(key, text.as_str().into())
				}
				AttrValue::Flag => params.insert_key(key),
				_ => {}
			}
		}
		let mut value = T::default();
		params.apply_reflect(&mut value).map_err(|err| {
			bevyhow!(
				"<{}> is not authorable from its attributes: {err}",
				element.tag
			)
		})?;
		Ok(Arc::new(value))
	}

	fn preload<T: Prescan>(
		value: &dyn Reflect,
		repo_store: &BlobStore,
	) -> SendBoxedFuture<Result> {
		match value.as_any().downcast_ref::<T>() {
			Some(value) => value.preload(repo_store),
			None => Box::pin(async { Ok(()) }),
		}
	}
}

/// Registers a [`Prescan`] type on an [`App`].
#[extend::ext(name=AppRegisterPrescanExt)]
pub impl App {
	/// Register `T` as acting before the entry builds, and as a reflect type
	/// so the built tree resolves the same tag. Call order is preload order.
	fn register_prescan<T: Prescan + GetTypeRegistration>(
		&mut self,
	) -> &mut Self {
		self.register_type::<T>();
		self.world_mut()
			.get_resource_or_init::<PrescanRegistry>()
			.register::<T>();
		self
	}
}

/// Everything entry resolution reads out of a raw entry document, from one
/// registry-free walk: every registered [`Prescan`] declaration and every
/// `<Template src>` include.
///
/// One parse per bootstrap, versus the several the same bytes used to get. The
/// declarations remain the authoring vocabulary; this is only the extraction,
/// against the [`PrescanRegistry`] the launch's plugins registered, so a
/// downstream crate's declaration is collected by the same walk.
///
/// A subtree carrying a `bx:cfg` is NOT scanned, whatever its condition says.
/// This walk runs before the world that would answer the condition is reachable,
/// and the two possible mistakes are not symmetric: skipping a met branch only
/// defers work the build does anyway (a template dir registers via its observer
/// instead, an include is one the watcher picks up on rebuild), while scanning an
/// EXCLUDED branch would fire a `<RequireCfg>` this build was never meant to
/// satisfy and fail the load over a requirement that does not apply. So the
/// conditional case is left to the build, which is the stage that can answer it.
/// Entry-level declarations belong at the entry's top level regardless.
///
/// What the entry *instantiates* is not read here: the build resolves every tag
/// and marks each registry template it expands
/// ([`TemplateInstance`](beet_core::prelude::TemplateInstance)), which is where
/// the watch path reads the structural templates from.
#[derive(Debug, Default, Clone)]
pub struct EntryPrescan {
	/// Every registered declaration found, in document order.
	declarations: Vec<Arc<dyn Reflect>>,
	/// Every local `<Template src>` include. Remote includes are skipped: they are
	/// not local files a watcher sees, and an include is the watcher's business
	/// rather than a declaration, so it stays a plain path list.
	pub includes: Vec<RelPath>,
}

impl EntryPrescan {
	/// Pre-scan an entry document against `registry`. A non-markup (serde)
	/// entry declares none of these, so it yields the default; a markup entry
	/// that fails to parse is an error, since every later stage would fail on
	/// it too.
	pub fn parse(
		entry: &MediaBytes,
		registry: &PrescanRegistry,
	) -> Result<Self> {
		if !matches!(entry.media_type(), MediaType::Bsx | MediaType::Html) {
			return Ok(Self::default());
		}
		let nodes =
			BsxNode::parse_document(entry.as_utf8()?, &BsxParseConfig::bsx())?;
		let mut prescan = Self::default();
		prescan.collect(&nodes, registry)?;
		prescan.xok()
	}

	/// Pre-scan an entry document, yielding the default rather than erroring on
	/// unreadable or unparseable content. The watch path's transitive include
	/// walk uses it, so a broken include never blocks watch startup.
	pub fn parse_lossy(entry: &MediaBytes, registry: &PrescanRegistry) -> Self {
		Self::parse(entry, registry).unwrap_or_default()
	}

	/// Every collected `T`, in document order.
	pub fn iter<T: Prescan>(&self) -> impl Iterator<Item = &T> {
		self.declarations
			.iter()
			.filter_map(|declaration| Self::downcast::<T>(declaration))
	}

	/// The `T` an erased declaration holds, through the `Arc` (which is itself
	/// an opaque `Reflect`, so a bare `as_any` would name the `Arc`).
	fn downcast<T: Prescan>(declaration: &Arc<dyn Reflect>) -> Option<&T> {
		declaration.as_ref().as_any().downcast_ref::<T>()
	}

	/// The first collected `T`, for a declaration of which the first wins.
	pub fn first<T: Prescan>(&self) -> Option<&T> { self.iter::<T>().next() }

	/// Whether the document declared nothing this walk reads.
	pub fn is_empty(&self) -> bool {
		self.declarations.is_empty() && self.includes.is_empty()
	}

	/// The erased declarations of one registered type, in document order.
	fn declarations_of(
		&self,
		type_id: TypeId,
	) -> impl Iterator<Item = &Arc<dyn Reflect>> {
		self.declarations.iter().filter(move |declaration| {
			declaration.as_ref().as_any().type_id() == type_id
		})
	}

	/// Recursively collect every declaration from `nodes`.
	fn collect(
		&mut self,
		nodes: &[BsxNode],
		registry: &PrescanRegistry,
	) -> Result {
		for node in nodes {
			let BsxNode::Element(element) = node else {
				continue;
			};
			// a conditional subtree is the build's to answer, see the type docs
			if element.attributes.iter().any(|attr| attr.key == "bx:cfg") {
				continue;
			}
			if element.tag == "Template" {
				self.includes.extend(
					Self::str_attr(element, "src")
						.filter(|src| !Self::is_remote(src))
						.map(RelPath::new),
				);
			} else if let Some(registration) = registry.by_tag(&element.tag)
				&& !Self::has_spread(element)
			{
				self.declarations.push((registration.collect)(element)?);
			}
			self.collect(&element.children, registry)?;
		}
		Ok(())
	}

	/// The value of a string attribute, if present. A non-literal (block) value
	/// is skipped: it cannot resolve without the registry this scan runs before.
	fn str_attr(element: &BsxElement, key: &str) -> Option<SmolStr> {
		element.attributes.iter().find_map(|attr| {
			match (attr.key == key, &attr.value) {
				(true, AttrValue::Str(value)) => {
					Some(SmolStr::from(value.as_str()))
				}
				_ => None,
			}
		})
	}

	/// Whether the element carries a bare-position spread (`<el {..}>`).
	fn has_spread(element: &BsxElement) -> bool {
		element
			.attributes
			.iter()
			.any(|attr| matches!(attr.value, AttrValue::Spread(_)))
	}

	/// Whether `src` names a remote endpoint rather than a local path.
	fn is_remote(src: &str) -> bool {
		src.starts_with("http://")
			|| src.starts_with("https://")
			|| src.starts_with("s3://")
	}
}

/// `<RequireCfg cfg="..">` fires even when the tree cannot build: the build
/// spawns every collected one ahead of the tree.
impl Prescan for RequireCfg {
	fn describe() -> &'static str {
		"refuses the load unless this binary satisfies its `cfg`, naming every unmet atom"
	}
}

/// `<Secrets path="..">` sets its `EnvVar` records into the process
/// environment before anything builds, so every declaration constructed in
/// the build walk and every verb dispatched on ready finds them set, and no
/// `main` knows any of this. A document that cannot be loaded (no identity, an
/// identity in none of its groups, a corrupt file) is one warning and nothing
/// set, never an error: a cloud box's repo store carries no identity, and a
/// contributor without one must still build and run. A declaration naming
/// another store (a `{StoreRef(..)}` spread) is an export target the verbs
/// read, which the spread rule already leaves uncollected.
#[cfg(feature = "vault")]
impl Prescan for Secrets {
	fn describe() -> &'static str {
		"loads the document's `EnvVar` records into the process environment (existing wins)"
	}

	fn preload(&self, repo_store: &BlobStore) -> SendBoxedFuture<Result> {
		let secrets = self.clone();
		let repo_store = repo_store.clone();
		Box::pin(async move {
			let loaded = async {
				SecretsHandle::in_store(repo_store, &secrets)?
					.load_env_vars()
					.await
			};
			if let Err(err) = loaded.await {
				warn!("secrets `{}`: {err}", secrets.label);
			}
			Ok(())
		})
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::prelude::*;

	/// The set the router registers, as the launch sees it.
	fn registry() -> PrescanRegistry {
		RouterPlugin
			.into_world()
			.resource::<PrescanRegistry>()
			.clone()
	}

	fn parse(markup: &str, registry: &PrescanRegistry) -> EntryPrescan {
		EntryPrescan::parse(&MediaBytes::new_bsx(markup), registry).unwrap()
	}

	/// One walk reads every declaration kind, at any depth, from one document.
	#[beet_core::test]
	fn parses_every_declaration() {
		let prescan = parse(
			r#"<Router>
				<RepoRoot src="../.."/>
				<TemplateDir src="templates"/>
				<RequireCfg cfg="feature:sockets && version:0.1.0"/>
				<Template src="header.bsx"/>
				<Template src="https://example.org/remote.bsx"/>
				<div>
					<TemplateDir src="more"/>
					<RequireCfg cfg="feature:ssh"/>
					<Template src="footer.bsx"/>
				</div>
			</Router>"#,
			&registry(),
		);
		prescan.first::<RepoRoot>().unwrap().src.xpect_eq("../..");
		prescan
			.iter::<TemplateDir>()
			.map(|dir| dir.src.clone())
			.collect::<Vec<_>>()
			.xpect_eq(vec![RelPath::from("templates"), RelPath::from("more")]);
		prescan
			.iter::<RequireCfg>()
			.cloned()
			.collect::<Vec<_>>()
			.xpect_eq(vec![
				RequireCfg::new("feature:sockets && version:0.1.0"),
				RequireCfg::new("feature:ssh"),
			]);
		// the remote include is skipped: it is not a local file a watcher sees
		prescan.includes.xpect_eq(vec![
			RelPath::from("header.bsx"),
			RelPath::from("footer.bsx"),
		]);
	}

	/// A `bx:cfg` subtree declares nothing: the build answers the condition.
	#[beet_core::test]
	fn conditional_subtree_declares_nothing() {
		parse(
			r#"<Router>
				<Fragment bx:cfg="feature:infra">
					<TemplateDir src="templates"/>
					<Template src="header.bsx"/>
				</Fragment>
			</Router>"#,
			&registry(),
		)
		.is_empty()
		.xpect_true();
	}

	/// The first `<RepoRoot>` wins, and a document declaring nothing yields the
	/// default.
	#[beet_core::test]
	fn first_repo_root_wins() {
		parse(
			r#"<Router><RepoRoot src="../.."/><RepoRoot src="nope"/></Router>"#,
			&registry(),
		)
		.first::<RepoRoot>()
		.unwrap()
		.src
		.xpect_eq("../..");
		parse("<Router/>", &registry()).is_empty().xpect_true();
	}

	/// The entry's own documents are read off the top level; one in another
	/// store (a spread naming it) and one under a `bx:cfg` are left to the
	/// build.
	#[cfg(feature = "vault")]
	#[beet_core::test]
	fn collects_the_entry_documents() {
		parse(
			r#"<Router>
				<Secrets/>
				<Secrets label="mail-prod" path="infra/secrets/mail--prod.toml"/>
				<Secrets label="mail-cold" path="secrets/export.toml" {StoreRef($cold)}/>
				<Secrets label="gated" bx:cfg="feature:vault"/>
			</Router>"#,
			&registry(),
		)
		.iter::<Secrets>()
		.cloned()
		.collect::<Vec<_>>()
		.xpect_eq(vec![
			Secrets::default(),
			Secrets::new("infra/secrets/mail--prod.toml")
				.with_label("mail-prod"),
		]);
	}

	/// A serde entry declares none of these, so it pre-scans to the default
	/// rather than erroring on non-markup bytes.
	#[beet_core::test]
	fn non_markup_yields_default() {
		EntryPrescan::parse(
			&MediaBytes::new(MediaType::Json, b"{}".to_vec()),
			&registry(),
		)
		.unwrap()
		.is_empty()
		.xpect_true();
	}

	/// A downstream type registers into the same walk, and its preload runs
	/// in registration order behind the types registered before it.
	#[beet_core::test]
	async fn a_registered_type_is_collected_and_preloaded_in_order() {
		#[derive(Debug, Default, Clone, PartialEq, Component, Reflect)]
		#[reflect(Component, Default)]
		struct Warm {
			cache: SmolStr,
			eager: bool,
		}
		#[derive(Debug, Default, Clone, PartialEq, Component, Reflect)]
		#[reflect(Component, Default)]
		struct Second;
		impl Prescan for Warm {
			fn describe() -> &'static str { "warms a cache" }
			fn preload(
				&self,
				repo_store: &BlobStore,
			) -> SendBoxedFuture<Result> {
				let (store, cache) = (repo_store.clone(), self.cache.clone());
				Box::pin(async move {
					store
						.insert(&RelPath::from("order.txt"), cache.to_string())
						.await
				})
			}
		}
		impl Prescan for Second {
			fn describe() -> &'static str { "runs second" }
			fn preload(
				&self,
				repo_store: &BlobStore,
			) -> SendBoxedFuture<Result> {
				let store = repo_store.clone();
				Box::pin(async move {
					let first = store
						.get_media(&RelPath::from("order.txt"))
						.await?
						.as_utf8()?
						.to_string();
					store
						.insert(
							&RelPath::from("order.txt"),
							format!("{first},second"),
						)
						.await
				})
			}
		}
		let mut app = App::new();
		app.register_prescan::<Warm>().register_prescan::<Second>();
		let registry = app.world().resource::<PrescanRegistry>().clone();
		registry.describe().xpect_eq(vec![
			("Warm", "warms a cache"),
			("Second", "runs second"),
		]);
		// document order puts `Second` first; registration order still runs
		// `Warm` first
		let prescan = parse(
			r#"<Router><Second/><Warm cache="hot" eager/></Router>"#,
			&registry,
		);
		prescan.first::<Warm>().cloned().unwrap().xpect_eq(Warm {
			cache: "hot".into(),
			eager: true,
		});
		let store = BlobStore::temp();
		registry.preload(&prescan, &store).await.unwrap();
		store
			.get_media(&RelPath::from("order.txt"))
			.await
			.unwrap()
			.as_utf8()
			.unwrap()
			.xpect_eq("hot,second");
	}
}
