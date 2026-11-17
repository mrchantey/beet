//! `<StandardSiteAnnounce/>`: a published post announced on Bluesky.
use super::publishing::*;
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// The announcement sequence, run after `<StandardSitePublish/>`: for each
/// page `--announce` names whose document has no announcement, a Bluesky post
/// from the publication's account linking the page, then the document
/// updated to point back at it; then the reposts, converged for every
/// `auto_repost` account credited on an announced document.
///
/// The post's text is the page's description, its links, mentions and
/// hashtags made live. It embeds the page as a link card: the canonical url,
/// the title and description, the page's social card as its thumb (never
/// its raw image, so every preview wears the one frame), and strong refs to
/// the document and the publication, which Bluesky hydrates the card from.
/// The document's ref in the card goes stale when the document gains its
/// `bskyPostRef`, which is expected: a post is never edited, and Bluesky
/// resolves the refs by uri.
///
/// An announcement is written once and never updated. A run finds one again
/// by the url its card embeds, so a run interrupted after the post resumes by
/// pointing the document at it rather than posting twice. A repost is found
/// by its subject, and only reposts of this publication's announcements are
/// ever the run's, an author's own reposts staying theirs.
#[action]
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn StandardSiteAnnounce(
	publish: Params<StandardSiteWriteParams>,
	/// Announce the published page this slug names, ie `full-stack-bevy`.
	/// Repeatable; a page already announced is left as it is.
	#[param]
	announce: Vec<String>,
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let dry_run = publish.0.dry_run;
	let world = cx.world();
	let publishing = Publishing::resolve(&cx.caller).await?;
	let mut records = publishing.records().await?;
	// every post the account holds, listed once and only when needed
	let mut posts = None;
	for slug in &announce {
		let page = publishing.page(slug)?;
		let path = publishing.document_path(page)?;
		let Some(document) = records.document(&path).cloned() else {
			// a dry publish beside it wrote nothing for this run to find
			if dry_run {
				info!("`{slug}` is announced once its document is published");
				continue;
			}
			bevybail!(
				"`{slug}` has no document to announce: publish it first, \
				 `--only={slug}`"
			);
		};
		if let Some(post) = &document.value.bsky_post_ref {
			info!("{:<9} {}", "unchanged", post.uri);
			continue;
		}
		let canonical = publishing.canonical(&path);
		if posts.is_none() {
			posts = Some(publishing.pds.list::<FeedPost>().await?);
		}
		let found = posts.iter().flatten().find(|entry| {
			entry
				.value
				.external()
				.is_some_and(|card| card.external.uri == canonical)
		});
		let post = match found {
			Some(entry) => {
				info!("{:<9} {}", "unchanged", entry.uri);
				entry.strong_ref()
			}
			None => {
				StandardSiteAnnounce::post(
					&publishing,
					&records,
					page,
					&document,
					canonical,
					dry_run,
				)
				.await?
			}
		};
		let announced = StandardSiteDocument {
			bsky_post_ref: Some(post),
			..document.value.clone()
		};
		let written = match dry_run {
			true => StrongRef::new(
				document.uri.clone(),
				AtprotoValue::from_serde(&announced)?
					.into_record(&StandardSiteDocument::COLLECTION)?
					.cid(),
			),
			false => {
				publishing
					.pds
					.put(&Rkeyed::new(
						document.rkey().clone(),
						announced.clone(),
					))
					.await?
			}
		};
		info!("{:<9} {}", "updated", written.uri);
		// the reposts below read the document as it now stands
		if let Some(entry) = records
			.documents
			.iter_mut()
			.find(|entry| entry.uri == document.uri)
		{
			*entry = RecordEntry {
				uri: written.uri,
				cid: written.cid,
				value: announced,
			};
		}
	}
	StandardSiteAnnounce::reposts(&world, &publishing, &records, dry_run)
		.await?;
	Ok(Pass(cx.input))
}

impl StandardSiteAnnounce {
	/// Post the announcement of `document`, published off the listed page
	/// `page` at `canonical`, answering the post written.
	async fn post(
		publishing: &Publishing,
		records: &StandardSiteRecords,
		page: &SyndicationPage,
		document: &RecordEntry<StandardSiteDocument>,
		canonical: SmolStr,
		dry_run: bool,
	) -> Result<StrongRef> {
		let publication = records.publication.as_ref().ok_or_else(|| {
			bevyhow!("the repo holds no publication at `{}`", publishing.url)
		})?;
		let description =
			document.value.description.clone().ok_or_else(|| {
				bevyhow!(
					"page '{}' has no description, which its announcement's \
					 text is",
					page.path
				)
			})?;
		let thumb = publishing.card(&page.path).await?;
		if let Some(thumb) = &thumb {
			thumb.upload(&publishing.pds, dry_run).await?;
		}
		let card = EmbedExternal::new(External {
			uri: canonical,
			title: document.value.title.clone(),
			description: description.clone(),
			thumb: thumb.map(|thumb| thumb.blob_ref),
			associated_refs: vec![
				document.strong_ref(),
				publication.strong_ref(),
			],
		});
		let text = RichText::resolve(description, &default()).await?;
		let post = FeedPost {
			embed: Some(card.into_union()?),
			..FeedPost::new(text, Timestamp::now())
		};
		Publishing::create(
			&publishing.pds,
			Rkeyed::new(Tid::now().into(), post),
			dry_run,
		)
		.await
	}

	/// Converge the reposts of every announced document by each
	/// `auto_repost` account it credits, the announcing account aside.
	async fn reposts(
		world: &AsyncWorld,
		publishing: &Publishing,
		records: &StandardSiteRecords,
		dry_run: bool,
	) -> Result {
		let announced = records
			.documents
			.iter()
			.filter_map(|entry| {
				entry
					.value
					.bsky_post_ref
					.clone()
					.map(|post| (post, entry.value.contributors.clone()))
			})
			.collect::<Vec<_>>();
		let announcing = publishing.pds.did().clone();
		let reposters = world
			.with_state::<Query<(Entity, &AtprotoAccount)>, _>(
				move |accounts| {
					accounts
						.iter()
						.filter(|(_, account)| {
							account.auto_repost()
								&& account.did() != &announcing
						})
						.map(|(entity, account)| {
							(entity, account.did().clone())
						})
						.collect::<Vec<_>>()
				},
			)
			.await;
		let announcements = announced
			.iter()
			.map(|(post, _)| post.uri.clone())
			.collect::<HashSet<_>>();
		for (account, did) in reposters {
			let subjects = announced
				.iter()
				.filter(|(_, contributors)| {
					contributors
						.iter()
						.any(|contributor| contributor.did == did)
				})
				.map(|(post, _)| post.clone())
				.collect::<Vec<_>>();
			if subjects.is_empty() {
				continue;
			}
			let pds = world
				.with(move |world: &mut World| {
					PdsQuery::resolve(world, account)
				})
				.await?;
			// an existing repost is kept as it is, since its time is its own
			let existing = pds.list::<FeedRepost>().await?;
			let wanted = subjects.into_iter().map(|subject| {
				existing
					.iter()
					.find(|entry| entry.value.subject.uri == subject.uri)
					.map(|entry| entry.clone().into_rkeyed())
					.unwrap_or_else(|| {
						Rkeyed::new(
							Tid::now().into(),
							FeedRepost::new(subject, Timestamp::now()),
						)
					})
			});
			let scope = announcements.clone();
			pds.converge(
				wanted.collect::<Vec<_>>(),
				&Converge::default().with_dry_run(dry_run).with_scope(
					move |repost: &FeedRepost| {
						scope.contains(&repost.subject.uri)
					},
				),
			)
			.await?
			.xref()
			.xmap(Publishing::log);
		}
		Ok(())
	}
}
