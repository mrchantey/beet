//! `<StandardSitePub/>`: the declaration a publication is built from.
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// A route subtree published as a standard.site publication: what the
/// publication record says about itself, where its documents come from, the
/// account whose public repo holds them, and the derived index remembering where
/// each record went.
///
/// ```html
/// <StandardSitePub {AccountRef($beet_org)} path="blog"
/// 	name="The Full Moon Harvest"/>
/// ```
///
/// Declared at the root of its [`Router`]'s url space, beside `<Sitemap/>`,
/// since the well-known route it serves roots where it sits. The documents
/// are the listed dated pages beneath [`path`](Self::path), the set an
/// `<RssFeed/>` declared there carries, so the declaration names no posts.
/// The account is the [`Pds`] the entity resolves through [`PdsQuery`]: an
/// `{AccountRef($account)}` spread, or an `<AtprotoAccount/>` ancestor.
///
/// The declaration is its own producing action: its [`Derivation`] is the
/// [`StandardSiteIndex`] at [`index`](Self::index), written by `publish`
/// alone, which this action rebuilds by listing the account's public repo. The served
/// side reads that file and never the public repo: see [`StandardSiteIndex`] for
/// what it serves.
///
/// Registered in every build that carries the data model, so a declaration
/// naming a `content` def this build has no renderer for (the
/// `standard_site_extra` feature's Leaflet is off in a served binary) still
/// builds, the def resolving where the publish step renders it.
///
/// [`Router`]: crate::prelude::Router
#[action]
#[derive(Debug, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn StandardSitePub(
	/// The route path the publication roots at, ie `blog`, joined to the
	/// site's homepage to form the publication's url. Empty for a whole site.
	#[field(no_clone)]
	path: FwdPath,
	/// The publication's name.
	#[field(no_clone)]
	name: SmolStr,
	/// What the publication is about.
	#[field(no_clone)]
	description: Option<String>,
	/// The self labels the publication carries, normally none.
	#[field(no_clone)]
	labels: Vec<SmolStr>,
	/// Whether the publication may appear in discovery feeds, the lexicon's
	/// own default.
	#[field(no_clone, default = true)]
	show_in_discover: bool,
	/// Whether each document carries the post as plain text, its
	/// `textContent`.
	#[field(no_clone, default = true)]
	text_content: bool,
	/// The lexicon def each document's `content` carries the post as:
	/// `org.beet.scene#body` by default, the post itself as a beet scene, else
	/// ie `pub.leaflet.content` with the `standard_site_extra` feature.
	/// Rendered through the [`LexiconRenderers`] where the publish step
	/// renders, never resolved at build.
	///
	/// [`LexiconRenderers`]: beet_ui::prelude::LexiconRenderers
	#[field(no_clone, default = StandardSitePub::default_content())]
	content: Option<Nsid>,
	/// The standard site media ingest policy the `content` def's embedded
	/// media is resolved under.
	#[field(no_clone)]
	media_ingest: MediaIngestPolicy,
	/// The derived index's path in the store the declaration resolves by
	/// ancestry, the package store's `derived/` in a site.
	#[field(no_clone, default = StandardSitePub::INDEX)]
	index: FwdPath,
	cx: ActionContext<BlobStore>,
) -> Result<EntityBody> {
	let entity = cx.id();
	let (publication, pds, package) = cx
		.world()
		.with(move |world: &mut World| -> Result<_> {
			(
				world.get::<StandardSitePub>(entity).cloned().ok_or_else(
					|| bevyhow!("entity {entity} has no `StandardSitePub`"),
				)?,
				PdsQuery::resolve(world, entity)?,
				world
					.get_resource::<PackageConfig>()
					.cloned()
					.unwrap_or_default(),
			)
				.xok()
		})
		.await?;
	let index =
		StandardSiteIndex::from_repo(&pds, &publication.url(&package)?).await?;
	// no sources: the index is a copy of the public repo, which the probe compares
	// against, see `StandardSiteIndex`
	EntityBody::default()
		.with_typed(&index)?
		.with_typed(&Provenance::new([]))
}

impl StandardSitePub {
	/// The index's default path, beside the site's other derived files.
	pub const INDEX: FwdPath = fwd_path!("derived/standard-site.json");
	/// The verb that alone writes the index.
	pub const WRITTEN_BY: &str = "publish";
	/// The well-known path a publication at the root of a site serves its
	/// uri from, a publication at a path appending it.
	const WELL_KNOWN: FwdPath = fwd_path!(".well-known/site.standard.publication");

	/// The `content` def a declaration names unless it says otherwise:
	/// `org.beet.scene#body`, the post itself as a beet scene.
	fn default_content() -> Option<Nsid> { Some(SceneBody::NSID) }

	/// The publication's url: [`PackageConfig::homepage`] joined with the
	/// declaration's path, its trailing slash trimmed as the lexicon asks.
	///
	/// # Errors
	/// Errors when the homepage is unset or names no origin, see
	/// [`PackageConfig::absolute_url`].
	pub fn url(&self, package: &PackageConfig) -> Result<Uri> {
		package
			.absolute_url(self.path.as_str())?
			.to_string()
			.trim_end_matches('/')
			.xmap(Uri::parse)
	}

	/// The `path` a standard site document of the page at route `page` carries,
	/// ie `/full-stack-bevy` for `blog/full-stack-bevy`, `None` for a page
	/// outside the publication.
	pub fn document_path(&self, page: &FwdPath) -> Option<SmolStr> {
		page.strip_prefix(&self.path)
			.map(|relative| format!("/{relative}").into())
	}

	/// The route path of the well-known file naming the publication, ie
	/// `.well-known/site.standard.publication/blog`.
	pub fn well_known_path(&self) -> FwdPath {
		Self::WELL_KNOWN.join(&self.path)
	}

	/// The [`Derivation`] declaring the index, which `publish` alone writes.
	pub fn derivation(&self) -> Derivation {
		Derivation::new(self.index.clone()).with_written_by(Self::WRITTEN_BY)
	}
}

impl TypeInstaller for StandardSitePub {
	/// Registers the declaration, the reference naming it, and the observer
	/// declaring and loading each one's index.
	fn install(app: &mut App) {
		app.register_type::<Self>()
			.register_type::<PublicationRef>()
			.add_observer(declare_index);
	}
}

/// Names a [`StandardSitePub`] declared elsewhere in the document, so a
/// publish step outside the served router reaches it:
/// `{PublicationRef($harvest)}` beside a
/// `<StandardSitePub bx:ref="harvest" ../>`, on the step or an ancestor.
#[derive(
	Debug, Clone, Copy, PartialEq, Eq, Component, Reflect, MapEntities,
)]
#[reflect(Component, MapEntities)]
pub struct PublicationRef(#[entities] pub Entity);

/// Observer: land the [`Derivation`] a [`StandardSitePub`] declares, then load
/// its index from the store it resolves by ancestry, the build's [`Ready`]
/// waiting on the load so a request never meets a site still reading it.
fn declare_index(
	ev: On<Insert<StandardSitePub>>,
	publications: Query<&StandardSitePub>,
	build_root: Option<Res<TemplateBuildRoot>>,
	mut commands: Commands,
) -> Result {
	let entity = ev.entity;
	let derivation = publications.get(entity)?.derivation();
	let root = build_root.map(|root| **root);
	commands.entity(entity).insert(derivation);
	commands.queue(move |world: &mut World| {
		let guard = TemplatePending::park_on(
			world,
			root.unwrap_or(entity),
			PendingKind::Passive,
			"<StandardSitePub> index load",
		);
		let Ok(mut entity_mut) = world.get_entity_mut(entity) else {
			// despawned before the command ran: the dropped guard resolves
			// through the sweep
			return;
		};
		entity_mut.run_async_local(async move |publication: AsyncEntity| {
			let loaded = StandardSiteIndex::load(&publication).await;
			publication
				.world()
				.with(move |world: &mut World| guard.resolve(world))
				.await;
			// reported, never fatal: the site serves without the index
			loaded.map_err(|err| err.with_severity(Severity::Error))
		});
	});
	Ok(())
}
