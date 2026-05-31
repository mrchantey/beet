//! Static-site export driven by the runtime [`RouteTree`].
//!
//! Walks the router's route tree for every static-path scene route or route
//! marked [`CacheStrategy::Static`], renders each through the same dispatch path
//! a live request would take, and writes the resulting HTML to an output
//! [`BlobStore`] as `<path>/index.html`.

use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Renders every static route in the router to HTML.
///
/// A route is exported when its path is fully static, its method is `GET`, and
/// it is either a scene route or marked [`CacheStrategy::Static`].
pub async fn collect_static_html(
	world: &AsyncWorld,
	router: Entity,
) -> Result<Vec<(SmolPath, String)>> {
	let paths = world
		.with(move |world: &mut World| -> Result<Vec<SmolPath>> {
			let tree = world
				.entity(router)
				.get::<RouteTree>()
				.ok_or_else(|| {
					bevyhow!("router entity {router} has no RouteTree")
				})?
				.clone();

			let mut paths = Vec::new();
			for node in tree.flatten_nodes() {
				if !node.path.is_static() {
					continue;
				}
				if node.method.map(|method| method != HttpMethod::Get)
					.unwrap_or(false)
				{
					continue;
				}
				let cache = world
					.entity(node.entity)
					.get::<CacheStrategy>()
					.copied()
					.unwrap_or_default();
				if node.is_scene() || cache == CacheStrategy::Static {
					paths.push(node.path.annotated_path());
				}
			}
			Ok(paths)
		})
		.await?;

	let entity = world.entity(router);
	let mut pages = Vec::new();
	for path in paths {
		let request = Request::get(path.with_leading_slash())
			.with_accept(MediaType::Html);
		let response = entity.call::<Request, Response>(request).await?;
		let html = response
			.into_result()
			.await
			.map_err(|err| bevyhow!("failed to render '{path}': {err}"))?
			.text()
			.await?;
		pages.push((path, html));
	}
	Ok(pages)
}

/// Renders every static route and writes it to the output store as
/// `<path>/index.html`, returning the written paths.
pub async fn export_static(
	world: &AsyncWorld,
	router: Entity,
	out: &BlobStore,
) -> Result<Vec<SmolPath>> {
	let pages = collect_static_html(world, router).await?;
	let mut written = Vec::new();
	for (path, html) in pages {
		let out_path = if path.segments().is_empty() {
			SmolPath::new("index.html")
		} else {
			path.join("index.html")
		};
		out.insert(&out_path, html).await?;
		written.push(out_path);
	}
	Ok(written)
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	#[beet_core::test]
	async fn exports_static_scenes() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		// `default_router` also wires the std-only `/app-info` scene route, which
		// reads a `PackageConfig` at render; insert one so it exports cleanly.
		world.insert_resource(pkg_config!());
		let router = world
			.spawn(default_router(children![
				(
					render_action::fixed_route("about", rsx_direct!{ <p>"About"</p> }),
					HttpMethod::Get
				),
				(
					render_action::fixed_route("", rsx_direct!{ <h1>"Home"</h1> }),
					HttpMethod::Get
				),
			]))
			.flush();

		let out = BlobStore::temp();
		let out2 = out.clone();
		let written = world
			.run_async_then(async move |world| {
				export_static(&world, router, &out2).await
			})
			.await
			.unwrap();

		// the two user scene routes plus the `app-info` route wired by
		// `default_router`.
		written.len().xpect_eq(3);
		out.get(&SmolPath::new("index.html"))
			.await
			.unwrap()
			.xmap(|bytes| String::from_utf8(bytes.to_vec()).unwrap())
			.xpect_contains("Home");
		out.get(&SmolPath::new("about/index.html"))
			.await
			.unwrap()
			.xmap(|bytes| String::from_utf8(bytes.to_vec()).unwrap())
			.xpect_contains("About");
		out.get(&SmolPath::new("app-info/index.html"))
			.await
			.unwrap()
			.xmap(|bytes| String::from_utf8(bytes.to_vec()).unwrap())
			.xpect_contains("App Info");
	}
}
