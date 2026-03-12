#![allow(unused, reason = "temp")]
use beet_core::prelude::*;
use beet_net::prelude::*;
use std::borrow::Cow;


#[derive(Debug, Clone, Component)]
pub struct Navigator {
	user_agent: Cow<'static, str>,
	/// A list of media types supported by
	/// this navigator. This should reflect the
	/// rendering capabililites.
	// media_types: Vec<MediaType>,
	/// The current url is still loading
	loading: bool,
	current_url: Url,
}

impl Default for Navigator {
	fn default() -> Self {
		Self {
			user_agent: "Mozilla/5.0 Beet/0.1".into(),
			current_url: Url::parse("about:blank"),
			loading: false,
		}
	}
}

impl Navigator {
	pub async fn navigate_to(
		entity: AsyncEntity,
		url: impl Into<Url>,
	) -> Result {
		let url = url.into();
		entity
			.get_mut::<Navigator, _>(move |mut navigator| {
				navigator.loading = true;
				navigator.current_url = url.clone();
			})
			.await?;
		todo!(
			"send request, filter out Scheme::About etc and handle appropriately, call Render"
		);
		// Request::get(url);
		// Ok(())
	}
}


// #[derive(EntityEvent)]
// pub struct NavigateTo {
// 	entity: Entity,
// 	url: Url,
// }
