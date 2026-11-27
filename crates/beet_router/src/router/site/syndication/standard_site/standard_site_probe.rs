//! `<StandardSiteProbe/>`: the published records checked against the live
//! site.
use super::publishing::*;
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Assert the live site claims every record its publication's public repo holds,
/// the way a standard.site reader verifies them:
///
/// - the well-known file answers the publication's uri;
/// - each document's canonical page answers 200 and links both the
///   publication and the document in its head;
/// - the committed index matches the public repo, else `publish` rewrites it.
///
/// Reports the documents with no announcement beside, which is no failure.
/// Read only and unauthenticated, the shape of `<AtprotoHandleProbe/>`: run
/// it after a deploy serves a new index. Every failure is collected, so one
/// run names them all.
#[action]
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn StandardSiteProbe(
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let publishing = Publishing::resolve(&cx.caller).await?;
	StandardSiteProbe::audit(&publishing, async |url: Url| {
		Request::get(url).with_accept(MediaType::Html).send().await
	})
	.await?;
	Ok(Pass(cx.input))
}

impl StandardSiteProbe {
	/// Check `publishing`'s records against the site `fetch` answers for an
	/// absolute url. A plain [`Fn`] rather than an [`AsyncFn`], whose
	/// borrowing call future rustc cannot prove `Send` inside an action.
	async fn audit<Fut: Future<Output = Result<Response>>>(
		publishing: &Publishing,
		fetch: impl Fn(Url) -> Fut,
	) -> Result {
		let Some(index) = publishing.records().await?.index() else {
			bevybail!(
				"{} holds no publication at `{}`: `{}` writes it",
				publishing.pds.describe(),
				publishing.url,
				StandardSitePub::WRITTEN_BY
			);
		};
		let publication = index.publication.uri.to_string();
		let mut failures = Vec::new();
		// the well-known file names the publication
		let well_known = publishing
			.package
			.absolute_url(publishing.declaration.well_known_path().as_str())?;
		match Self::get(&fetch, well_known.clone()).await {
			Ok(body) if body.trim() == publication => {}
			Ok(body) => failures.push(format!(
				"{well_known} answers `{}`, not {publication}",
				body.trim()
			)),
			Err(err) => failures.push(err.to_string()),
		}
		// each document's page links both records
		for document in &index.documents {
			let url = Url::parse(&publishing.canonical(&document.path))?;
			match Self::get(&fetch, url.clone()).await {
				Ok(html) => {
					for (rel, href) in [
						("site.standard.publication", publication.clone()),
						("site.standard.document", document.uri.to_string()),
					] {
						if !html
							.contains(&format!(r#"rel="{rel}" href="{href}""#))
						{
							failures.push(format!(
								"{url} carries no `<link rel=\"{rel}\" \
								 href=\"{href}\">`"
							));
						}
					}
				}
				Err(err) => failures.push(err.to_string()),
			}
		}
		// the committed index is the public repo's
		let derivation = publishing.declaration.derivation();
		match derivation.read(&publishing.store).await? {
			None => failures.push(format!(
				"no index at `{}`: `{}` writes it",
				derivation.path,
				StandardSitePub::WRITTEN_BY
			)),
			Some(body) if body.get_typed::<StandardSiteIndex>()? != index => {
				failures.push(format!(
					"the index at `{}` differs from the public repo: `{}` rewrites it",
					derivation.path,
					StandardSitePub::WRITTEN_BY
				))
			}
			Some(_) => {}
		}
		let unannounced = index
			.documents
			.iter()
			.filter(|document| document.bsky_post.is_none())
			.map(|document| document.path.as_str())
			.collect::<Vec<_>>();
		if !unannounced.is_empty() {
			info!("not yet announced: {}", unannounced.join(", "));
		}
		if !failures.is_empty() {
			bevybail!(
				"standard site probe failed:\n{}",
				failures
					.iter()
					.map(|failure| format!("- {failure}"))
					.collect::<Vec<_>>()
					.join("\n")
			);
		}
		info!(
			"standard site probe passed: the publication and {} document(s) \
			 verify",
			index.documents.len()
		);
		Ok(())
	}

	/// The body `fetch` answers for `url`, an error naming it unless 200.
	async fn get<Fut: Future<Output = Result<Response>>>(
		fetch: &impl Fn(Url) -> Fut,
		url: Url,
	) -> Result<String> {
		let response = fetch(url.clone()).await?;
		let status = response.status();
		if status != StatusCode::OK {
			bevybail!("{url} answers {status}");
		}
		response.text().await
	}
}

#[cfg(test)]
mod test {
	use super::super::publishing::test::*;
	use super::super::publishing::*;
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	/// Probe `site` against what its own router serves.
	async fn probe(site: &mut Site) -> Result {
		let (root, publication) = (site.root, site.publication);
		let step = site.world.spawn(PublicationRef(publication)).id();
		site.world
			.run_async_then(async move |world| {
				let publishing =
					Publishing::resolve(&world.entity(step)).await?;
				StandardSiteProbe::audit(&publishing, |url: Url| {
					let entity = world.entity(root);
					async move {
						entity
							.exchange(Request::get(url.path_string()))
							.await
							.xok()
					}
				})
				.await
			})
			.await
	}

	/// A published site verifies; a document published since its index was
	/// written fails twice, its page linking no document and the index
	/// behind the public repo.
	#[beet_core::test]
	async fn verifies_the_served_site() {
		let mut site = Site::new().await;
		probe(&mut site)
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("`publish` writes it");
		site.publish("--only=full-stack-bevy").await.unwrap();
		probe(&mut site).await.unwrap();
		let publication = site.publication;
		site.run(
			(PublicationRef(publication), ExchangeSequence, children![
				StandardSitePublish
			]),
			"--all",
		)
		.await
		.unwrap();
		probe(&mut site)
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains(
				"blog/ecs-router carries no `<link rel=\"site.standard.document\"",
			)
			.xpect_contains("differs from the public repo");
	}
}
