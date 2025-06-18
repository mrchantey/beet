use bevy::prelude::*;

/// A struct that should be set as a top level context value.
///
/// ## Example
///
/// In this example we dont need to set any of the Head values
/// they will fallback to Brand values.
/// ```
/// # #![feature(more_qualified_paths)]
/// # use beet_template::as_beet::*;
/// # use beet_design::prelude::*;
/// # use bevy::prelude::*;
///
/// #[template]
/// fn MyTemplate()-> impl Bundle {
/// 	ReactiveApp::insert_resource(Brand{
/// 		title: "My Site".into(),
/// 		description: "A site about stuff".into(),
///			site_url: "https://myapp.com".into(),
/// 		version: std::env!("CARGO_PKG_VERSION").into(),
/// 	});
///
/// 	rsx!{
/// 	  <Head/>
/// 	}
/// }
///
/// ```
#[derive(Clone, Resource)]
pub struct Brand {
	/// The pretty title of the application,
	/// ie `My App`
	pub title: String,
	/// A short description of the application
	/// ie `A site about stuff`
	pub description: String,
	/// The canonical url of the production site,
	/// ie `https://myapp.com`
	pub site_url: String,
	/// The site version, usually set via
	/// `std::env!("CARGO_PKG_VERSION")`
	pub version: String,
}
