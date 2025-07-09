use beet::prelude::*;
use beet_new_web::prelude::*;

/// Server entry point, also used for generating static files.
#[cfg(not(feature = "client"))]
#[rustfmt::skip]
fn main() -> Result { 

	AppRouter::default()
		.add_plugins((PagesPlugin,DocsPlugin.layer(DocsLayout),ActionsPlugin))
		.run() 
}

/// Client entry point, used for client-side reactivity.
/// The wasm binary will only be loaded on routes containing
/// a client directive, ie `client:load`
#[cfg(feature = "client")]
fn main() {
	App::new()
		.add_plugins((TemplatePlugin, ClientIslandPlugin))
		.set_runner(ReactiveApp::runner)
		.run();
}
