use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// A scene route at `/app-info` rendering the [`PackageConfig`] as an article.
///
/// Requires a [`PackageConfig`] resource (eg via `pkg_config!()`).
pub fn app_info() -> impl Bundle {
	render_action::async_route("app-info", app_info_scene)
}

async fn app_info_scene(cx: ActionContext<Request>) -> impl Bundle {
	let PackageConfig {
		title,
		description,
		version,
		stage,
		..
	} = cx
		.caller
		.with_state::<Res<PackageConfig>, PackageConfig>(|_entity, config| {
			config.clone()
		})
		.await
		.unwrap();
	rsx! {
		<article>
			<h1>"App Info"</h1>
			<p>{format!("Title: {title}")}</p>
			<p>{format!("Description: {description}")}</p>
			<p>{format!("Version: {version}")}</p>
			<p>{format!("Stage: {stage}")}</p>
		</article>
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	#[beet_core::test]
	async fn renders_package_title() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		world.insert_resource(pkg_config!());
		world
			.spawn((router(), children![app_info()]))
			.call::<Request, Response>(Request::get("app-info"))
			.await
			.unwrap()
			.unwrap_str()
			.await
			.xpect_contains("App Info")
			.xpect_contains("beet_router");
	}
}
