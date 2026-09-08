//! The crawler policy route: `robots.txt`.

use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// The site's `robots.txt`: a Static `GET` route serving a crawler policy that
/// allows everything and points at the sitemap.
///
/// Declared as `<Robots/>`, or `<Robots disallow="/admin,/tmp"/>` to keep
/// crawlers out of a subtree. The [`PackageConfig`] is read at dispatch rather
/// than at build, so a markup-patched `homepage` is respected.
///
/// Unlisted pages are deliberately absent: a `Disallow` line publishes the very
/// path it hides, so their exclusion is a `noindex` meta tag on the page plus
/// their absence from the sitemap, the feeds and the search index.
#[template]
pub fn Robots(
	/// Paths crawlers are asked not to visit, one `Disallow:` line each.
	#[prop]
	disallow: Vec<SmolStr>,
) -> impl Bundle {
	(
		route::exchange(
			"robots.txt",
			Action::<Request, Response>::new_async(
				async move |cx: ActionContext<Request>| -> Result<Response> {
					let sitemap = cx
						.caller
						.with_state::<Res<PackageConfig>, _>(|_, package| {
							package.absolute_url("sitemap.xml").ok()
						})
						.await?;
					Response::ok_body(
						robots_txt(&disallow, sitemap.as_deref()),
						MediaType::Text,
					)
					.xok()
				},
			),
		),
		HttpMethod::Get,
		ExportStrategy::Static,
	)
}

/// The policy document: a blanket allow, the caller's `Disallow` lines, then
/// the sitemap pointer when the site names an origin to resolve it against.
fn robots_txt(disallow: &[SmolStr], sitemap: Option<&str>) -> String {
	let mut body = String::from("User-agent: *\nAllow: /\n");
	for path in disallow {
		body.push_str(&format!("Disallow: {path}\n"));
	}
	if let Some(sitemap) = sitemap {
		body.push_str(&format!("\nSitemap: {sitemap}\n"));
	}
	body
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	/// A router serving `<Robots {..}/>`, with the site identity the sitemap
	/// pointer resolves against.
	async fn robots(world: &mut World, root: Entity) -> Response {
		world
			.entity_mut(root)
			.exchange(Request::get("robots.txt"))
			.await
	}

	#[beet_core::test]
	async fn serves_the_policy() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		world.insert_resource(PackageConfig {
			homepage: Some("https://beet.org".into()),
			..default()
		});
		let root = world.spawn(Router).flush();
		world
			.spawn_template(Snippet::from_bundle((ChildOf(root), rsx! {
				<Robots disallow={vec![SmolStr::new("/admin")]}/>
			})))
			.unwrap();
		world.flush();

		let response = robots(&mut world, root).await;
		response
			.parts
			.headers
			.get::<header::ContentType>()
			.unwrap()
			.unwrap()
			.xpect_eq(MediaType::Text);
		response.unwrap_str().await.xpect_eq(
			"User-agent: *\nAllow: /\nDisallow: /admin\n\nSitemap: https://beet.org/sitemap.xml\n"
				.to_string(),
		);
	}

	/// A site that names no origin still gets a policy, minus the sitemap
	/// pointer no crawler could resolve.
	#[beet_core::test]
	async fn omits_the_sitemap_without_a_homepage() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		world.insert_resource(PackageConfig::default());
		let root = world.spawn(Router).flush();
		world
			.spawn_template(Snippet::from_bundle((ChildOf(root), rsx! {
				<Robots/>
			})))
			.unwrap();
		world.flush();

		robots(&mut world, root)
			.await
			.unwrap_str()
			.await
			.xpect_eq("User-agent: *\nAllow: /\n".to_string());
	}
}
