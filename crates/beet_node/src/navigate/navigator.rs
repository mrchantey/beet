#![allow(unused, reason = "temp")]
use beet_core::prelude::*;
use beet_net::prelude::*;
use std::borrow::Cow;

use crate::parse::MediaParser;



#[derive(Debug, Clone, Component)]
#[component(on_add=on_add)]
pub struct Navigator {
	render_targets: Vec<Entity>,
	user_agent: Cow<'static, str>,
	/// A list of media types supported by
	/// this navigator. This should reflect the
	/// rendering capabililites.
	accepts: Vec<MediaType>,
	/// The current url is still loading
	loading: bool,
	home_url: Url,
	current_url: Url,
	history: Vec<Url>,
	history_position: usize,
}

fn on_add(mut world: DeferredWorld, cx: HookContext) {
	world
		.commands()
		.entity(cx.entity)
		.queue_async(async |entity| {
			let home_url = entity
				.get(|navigator: &Navigator| navigator.home_url.clone())
				.await?;
			Navigator::navigate_to(entity, home_url).await
		});
}

const DEFAULT_HOME: &str = "about:blank";

impl Default for Navigator {
	fn default() -> Self {
		Self {
			render_targets: Vec::new(),
			user_agent: "Mozilla/5.0 Beet/0.1".into(),
			home_url: Url::parse(DEFAULT_HOME),
			accepts: vec![
				// we prefer markdown, currently beet does not parse scripts and styles
				// etc so less content is simply faster
				MediaType::Markdown,
				// secondarily accept html
				MediaType::Html,
			],
			current_url: Url::parse(DEFAULT_HOME),
			loading: false,
			history: Vec::new(),
			history_position: 0,
		}
	}
}

impl Navigator {
	pub fn new(home_url: impl Into<Url>) -> Self {
		let home_url = home_url.into();
		Self {
			home_url: home_url.clone(),
			current_url: home_url,
			..default()
		}
	}

	pub fn current_url(&self) -> &Url { &self.current_url }

	/// ## Errors
	/// - Errors if no [`Navigator`]
	/// - Errors if request failes
	/// - Errors if no [`RenderTargets`]
	pub async fn navigate_to(
		entity: AsyncEntity,
		url: impl Into<Url>,
	) -> Result {
		let url = url.into();
		let url2 = url.clone();
		// 1. set current url and loading state
		let (user_agent, accepts) = entity
			.get_mut(move |mut navigator: Mut<Navigator>| {
				navigator.loading = true;
				navigator.current_url = url2;
				(navigator.user_agent.clone(), navigator.accepts.clone())
			})
			.await?;

		// 2. make request and get body
		let media_bytes = Request::get(url)
			.with_header::<header::UserAgent>(user_agent)
			.with_header::<header::Accept>(accepts)
			.send()
			.await?
			.into_result()
			.await?
			.media_bytes()
			.await?;

		// 3. trigger RenderMedia event for render targets
		let render_targets = entity.get_cloned::<RenderTargets>().await?;
		entity
			.world()
			.with_then(move |world| {
				for target in render_targets.iter() {
					world.entity_mut(target).trigger(|entity| RenderMedia {
						entity,
						media_bytes: media_bytes.clone(),
					});
				}
			})
			.await;

		// 4. set loading state
		entity
			.get_mut(move |mut navigator: Mut<Navigator>| {
				navigator.loading = false;
			})
			.await?;

		Ok(())
	}
}

/// Triggered on each [`RenderedBy`] component
/// when the [`Navigator`] visits a new page.
#[derive(Debug, Clone, EntityEvent)]
pub struct RenderMedia {
	entity: Entity,
	media_bytes: MediaBytes<'static>,
}
impl std::ops::Deref for RenderMedia {
	type Target = MediaBytes<'static>;

	fn deref(&self) -> &Self::Target { &self.media_bytes }
}

/// Assigned to a [`Navigator`], containing the list
/// of entities that should render on each url visit.
#[derive(Debug, Clone, Component)]
#[relationship_target(relationship = RenderedBy)]
pub struct RenderTargets(Vec<Entity>);

/// Assigned to an entity, for example a [`TuiNodeRenderer`] to be
/// the target of a render call
#[derive(Debug, Clone, Component)]
#[relationship(relationship_target = RenderTargets)]
pub struct RenderedBy(pub Entity);
