use crate::prelude::*;
use beet_core::prelude::*;

/// A template route at `/app-info` rendering the [`PackageConfig`] as an article.
///
/// Requires a [`PackageConfig`] resource (eg via `pkg_config!()`).
pub fn app_info() -> impl Bundle {
	render_action::func_route("app-info", |_: ()| rsx! { <AppInfoContent/> })
}

/// Reads [`PackageConfig`] synchronously at template build, returning an
/// `<article>` describing the package.
#[template(system)]
fn AppInfoContent(config: Res<PackageConfig>) -> impl Bundle {
	let PackageConfig {
		title,
		description,
		version,
		stage,
		..
	} = config.clone();
	rsx! {
		<article>
			<h1>"App Info"</h1>
			<p>"Title: "{title}</p>
			<p>"Description: "{description}</p>
			<p>"Version: "{version}</p>
			<p>"Stage: "{stage}</p>
		</article>
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	#[beet_core::test]
	async fn renders_package_title() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		world.insert_resource(pkg_config!());
		// `default_router` already wires `app_info()` as a child under std.
		world
			.spawn(default_router())
			.route(Request::get("app-info"))
			.await
			.unwrap_str()
			.await
			.xpect_contains("App Info")
			.xpect_contains("beet_router");
	}
}
