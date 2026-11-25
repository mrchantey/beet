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
/// hashtags made live, and the post is dated the day it is made: when the page
/// was written is its social card's to say, and the document's. A text over a
/// Bluesky post's 300 graphemes refuses the run before any write, dry run
/// included, and `beet check` flags such a description beforehand. It embeds
/// the page as a link card: the canonical url, the title and description, the
/// page's social card as its thumb (never its raw image, so every preview
/// wears the one frame), and strong refs to the document and the publication,
/// which Bluesky hydrates the card from.
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
	announce: Params<StandardSiteAnnounceParams>,
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let ActionContext { caller, mut input } = cx;
	let world = caller.world().clone();
	let dry_run = publish.0.dry_run;
	let publishing = Publishing::resolve(&caller)
		.await?
		.in_run(&mut input, dry_run);
	let mut records = publishing.records().await?;
	// every text composed first, so one too long refuses before any write
	let texts = announce
		.0
		.announce
		.iter()
		.map(|slug| {
			publishing
				.announcement(slug)
				.map(|text| (slug.as_str(), text))
		})
		.collect::<Result<Vec<_>>>()?;
	// every post the account holds, listed once and only when needed
	let mut posts = None;
	for (slug, text) in texts {
		let page = publishing.page(slug)?;
		let path = publishing.document_path(page)?;
		let Some(document) = records.document(&path).cloned() else {
			bevybail!(
				"`{slug}` has no document to announce: publish it first, \
				 `--only={slug}`"
			);
		};
		if let Some(post) = &document.value.bsky_post_ref {
			info!("{}", ConvergeVerb::Unchanged.line(dry_run, &post.uri));
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
				info!("{}", ConvergeVerb::Unchanged.line(dry_run, &entry.uri));
				entry.strong_ref()
			}
			None => {
				info!("`{slug}` announces as {text:?}");
				StandardSiteAnnounce::post(
					&publishing,
					&records,
					page,
					&document,
					canonical,
					text,
				)
				.await?
			}
		};
		let announced = StandardSiteDocument {
			bsky_post_ref: Some(post),
			..document.value.clone()
		};
		let written = publishing
			.pds
			.put(&Rkeyed::new(document.rkey().clone(), announced.clone()))
			.await?;
		info!("{}", ConvergeVerb::Updated.line(dry_run, &written.uri));
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
	StandardSiteAnnounce::reposts(
		&world,
		&publishing,
		&records,
		&mut input,
		dry_run,
	)
	.await?;
	Ok(Pass(input))
}

impl StandardSiteAnnounce {
	/// Post `text` announcing `document`, published off the listed page
	/// `page` at `canonical`, answering the post written.
	async fn post(
		publishing: &Publishing,
		records: &StandardSiteRecords,
		page: &SyndicationPage,
		document: &RecordEntry<StandardSiteDocument>,
		canonical: SmolStr,
		text: String,
	) -> Result<StrongRef> {
		let publication = records.publication.as_ref().ok_or_else(|| {
			bevyhow!("the repo holds no publication at `{}`", publishing.url)
		})?;
		let thumb = publishing.card(&page.path).await?;
		if let Some(thumb) = &thumb {
			thumb.upload(&publishing.pds).await?;
		}
		let card = EmbedExternal::new(External {
			uri: canonical,
			title: document.value.title.clone(),
			description: document.value.description.clone().unwrap_or_default(),
			thumb: thumb.map(|thumb| thumb.blob_ref),
			associated_refs: vec![
				document.strong_ref(),
				publication.strong_ref(),
			],
		});
		let text = RichText::resolve(text, &default()).await?;
		let post = FeedPost {
			embed: Some(card.into_union()?),
			..FeedPost::new(text, Timestamp::now())
		};
		Publishing::create(
			&publishing.pds,
			Rkeyed::new(Tid::now().into(), post),
		)
		.await
	}

	/// Converge the reposts of every announced document by each
	/// `auto_repost` account it credits, the announcing account aside, each
	/// account's repo the dry one `request` carries when `dry_run`.
	async fn reposts(
		world: &AsyncWorld,
		publishing: &Publishing,
		records: &StandardSiteRecords,
		request: &mut Request,
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
				.await?
				.xmap(|pds| DryRunRepos::resolve(request, pds, dry_run));
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
				&Converge::default().with_scope(move |repost: &FeedRepost| {
					scope.contains(&repost.subject.uri)
				}),
			)
			.await?
			.xref()
			.xmap(Publishing::log);
		}
		Ok(())
	}
}
