use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
// Brought in so the `-> impl Scene` return type on `AppInfoScene` resolves.
use beet_ui::prelude::Scene;

/// A scene route at `/app-info` rendering the [`PackageConfig`] as an article.
///
/// Requires a [`PackageConfig`] resource (eg via `pkg_config!()`).
pub fn app_info() -> impl Bundle {
	render_action::scene_route("app-info", |_cx: ActionContext<Request>| {
		AppInfoScene(AppInfoSceneProps::default())
	})
}

/// Reads [`PackageConfig`] synchronously at scene build, returning an
/// `<article>` describing the package.
#[scene(system)]
fn AppInfoScene(config: Res<PackageConfig>) -> impl Scene {
	let PackageConfig { title, description, version, stage, .. } = config.clone();
	rsx! {
		<article>
			<h1>"App Info"</h1>
			<p>{template_value(Value::new(format!("Title: {title}")))}</p>
			<p>{template_value(Value::new(format!("Description: {description}")))}</p>
			<p>{template_value(Value::new(format!("Version: {version}")))}</p>
			<p>{template_value(Value::new(format!("Stage: {stage}")))}</p>
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
