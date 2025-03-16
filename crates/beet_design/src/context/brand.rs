use std::borrow::Cow;


/// A struct that should be set as a top level context value.
///
/// ## Example
///
/// In this example we dont need to set any of the Head values
/// they will fallback to Brand values.
/// ```
/// # use beet_rsx::prelude::*;
/// # use beet_design::prelude::*;
///
/// fn my_component()-> RsxRoot{
/// 	set_context(Brand{
/// 		title: "My Site".into(),
/// 		description: "A site about stuff".into(),
///
/// 	});
///
/// 	rsx!{
/// 	  <Head/>
/// 	}
/// }
///
/// ```
#[derive(Clone)]
pub struct Brand {
	/// The pretty title of the application,
	/// ie `My App`
	pub title: Cow<'static, str>,
	/// A short description of the application
	/// ie `A site about stuff`
	pub description: Cow<'static, str>,
	/// The canonical url of the production site,
	/// ie `https://myapp.com`
	pub site_url: Cow<'static, str>,
}
