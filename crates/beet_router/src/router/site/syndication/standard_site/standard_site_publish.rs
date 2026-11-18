//! `<StandardSitePublish/>`: the publication and its documents converged.
use super::publishing::*;
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_ui::prelude::*;

/// The first step of a publish sequence: converge the publication record,
/// then one standard site document per listed dated page in scope.
///
/// ```html
/// <Route path="publish" {(ExchangeSequence, PublicationRef($harvest))}>
/// 	<StandardSitePublish/>
/// 	<StandardSiteAnnounce/>
/// 	<StandardSiteIndexWrite/>
/// </Route>
/// ```
///
/// The publication is written from its declaration, the site's theme and
/// its icon (the `<IconSet>`'s 512, uploaded when its cid differs). The
/// documents publish on purpose, one page at a time, so a back catalogue
/// drips out rather than landing at once: every document already in the
/// repo is updated, a document is created only for a page `--only` names or
/// for every page under `--all`, and the rest are left alone. A page renamed
/// behind a `<Redirect>` updates the record it was published as.
///
/// Each page in scope renders once inside [`LivePage::scoped`] with
/// [`StandardSitePub::request`], its document rendered off the tree and,
/// when it differs from the record, the blobs it references uploaded before
/// the tree is released. Its contributors are its authors matched to the
/// declared accounts, and its announcement is the one the record already
/// carries. Every record is then converged at the rkey the repo holds it at,
/// or a freshly minted TID. A document whose page is gone or became a draft
/// is an orphan, reported, and deleted under `--prune`; one merely outside
/// `--only` never is.
///
/// The repo itself is the memory: every record carries its natural key, so
/// the step finds what an earlier run wrote by listing, and a run
/// interrupted anywhere resumes without writing anything twice.
#[action]
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn StandardSitePublish(
	publish: Params<StandardSiteWriteParams>,
	/// Delete the documents whose page is gone or became a draft, rather
	/// than only reporting them.
	#[param]
	prune: bool,
	/// Publish the page this slug names, ie `full-stack-bevy`, beside
	/// updating every document already published. Repeatable.
	#[param]
	only: Vec<String>,
	/// Publish every listed dated page.
	#[param]
	all: bool,
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let dry_run = publish.0.dry_run;
	let world = cx.world();
	let publishing = Publishing::resolve(&cx.caller).await?;
	let records = publishing.records().await?;
	// the named pages first, so a draft or a typo refuses the whole run
	let named = only
		.iter()
		.map(|slug| publishing.page(slug).map(|page| page.route))
		.collect::<Result<Vec<_>>>()?;
	let site = StandardSitePublish::publication(
		&world,
		&publishing,
		&records,
		dry_run,
	)
	.await?;
	let mut wanted = Vec::new();
	for page in &publishing.pages {
		let published = publishing.published(&records, page)?;
		if !(all || named.contains(&page.route) || published.is_some()) {
			continue;
		}
		wanted.push(
			StandardSitePublish::document(
				&world,
				&publishing,
				&site,
				page,
				published,
				dry_run,
			)
			.await?,
		);
	}
	let scope = site.clone();
	publishing
		.pds
		.converge(
			wanted,
			&Converge::default()
				.with_dry_run(dry_run)
				.with_prune(prune)
				.with_scope(move |document: &StandardSiteDocument| {
					document.site.at_uri().as_ref() == Some(&scope)
				}),
		)
		.await?
		.xref()
		.xmap(Publishing::log);
	Ok(Pass(cx.input))
}

impl StandardSitePublish {
	/// Converge the publication record, answering the address it lives at.
	async fn publication(
		world: &AsyncWorld,
		publishing: &Publishing,
		records: &StandardSiteRecords,
		dry_run: bool,
	) -> Result<AtUri> {
		let existing = records.publication.as_ref();
		let rkey = existing
			.map(|entry| entry.rkey().clone())
			.unwrap_or_else(|| Tid::now().into());
		let icon = publishing.icon().await?;
		if let Some(icon) = &icon
			&& existing.and_then(|entry| entry.value.icon.as_ref())
				!= Some(&icon.blob_ref)
		{
			icon.upload(&publishing.pds, dry_run).await?;
		}
		let theme = world
			.with(|world: &mut World| {
				world.get_resource::<Theme>().cloned().unwrap_or_default()
			})
			.await;
		let record = StandardSitePublication::from_declaration(
			&publishing.declaration,
			&publishing.package,
			&theme,
			icon.map(|icon| icon.blob_ref),
		)?;
		let url = publishing.url.clone();
		publishing
			.pds
			.converge(
				[Rkeyed::new(rkey.clone(), record)],
				&Converge::default().with_dry_run(dry_run).with_scope(
					move |publication: &StandardSitePublication| {
						publication.url == url
					},
				),
			)
			.await?
			.xref()
			.xmap(Publishing::log);
		publishing
			.pds
			.uri(&StandardSitePublication::COLLECTION, &rkey)
			.xok()
	}

	/// The document the listed page `page` publishes as within the
	/// publication at `site`, at the rkey of the record it was `published`
	/// as, its referenced blobs uploaded when it differs from that record.
	async fn document(
		world: &AsyncWorld,
		publishing: &Publishing,
		site: &AtUri,
		page: &SyndicationPage,
		published: Option<&RecordEntry<StandardSiteDocument>>,
		dry_run: bool,
	) -> Result<Rkeyed<StandardSiteDocument>> {
		let rkey = published
			.map(|entry| entry.rkey().clone())
			.unwrap_or_else(|| Tid::now().into());
		let existing = published.map(|entry| entry.value.clone());
		let contributors = publishing.contributors(world, page).await?;
		let (publication, pds, site, page) = (
			publishing.declaration.clone(),
			publishing.pds.clone(),
			site.clone(),
			page.clone(),
		);
		LivePage::scoped(
			&world.entity(publishing.router),
			publication.request(&page.path),
			async move |live| {
				let rendered = StandardSiteDocument::render(
					&publication,
					site,
					&page.path,
					&page.meta,
					&live,
				)
				.await?;
				let document = StandardSiteDocument {
					contributors,
					bsky_post_ref: existing
						.as_ref()
						.and_then(|existing| existing.bsky_post_ref.clone()),
					..rendered
				};
				// an unchanged document references blobs the repo holds
				if !dry_run && existing.as_ref() != Some(&document) {
					let record = AtprotoValue::from_serde(&document)?
						.into_record(&StandardSiteDocument::COLLECTION)?;
					live.upload_blob_refs(&pds, &record).await?;
				}
				Rkeyed::new(rkey, document).xok()
			},
		)
		.await
	}
}
