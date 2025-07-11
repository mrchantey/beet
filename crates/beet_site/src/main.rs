use beet::prelude::*;
use beet_site::prelude::*;


#[cfg(not(feature = "client"))]
fn main() -> Result {
	AppRouter::default()
		.add_plugins((
			PagesPlugin,
			ActionsPlugin,
			DocsPlugin.layer(ArticleLayout),
			BlogPlugin.layer(ArticleLayout),
			BeetDesignMockupsPlugin.layer(ArticleLayout),
		))
		.run()
}

#[cfg(feature = "client")]
fn main() {
	App::new()
		.add_plugins((TemplatePlugin, ClientIslandPlugin))
		.set_runner(ReactiveApp::runner)
		.run();
}
