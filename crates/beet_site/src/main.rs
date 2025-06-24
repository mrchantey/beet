use beet::prelude::*;
use beet_site::prelude::*;


#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result {
	AppRouter::default()
		.add_plugins((
			PagesPlugin,
			ActionsPlugin,
			DocsPlugin.layer(ArticleLayout),
			BeetDesignMockupsPlugin.layer(ArticleLayout),
		))
		.run()
}

#[cfg(target_arch = "wasm32")]
fn main() {
	App::new()
		.add_plugins((TemplatePlugin, ClientIslandPlugin))
		// .add_resource(SiteUrl::new("https://beetrs.dev"))
		.run();
}
