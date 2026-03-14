use beet_core::prelude::*;
use beet_net::prelude::*;
use std::borrow::Cow;
use std::collections::VecDeque;


/// Maximum number of history entries to retain.
const HISTORY_LIMIT: usize = 100;

const DEFAULT_HOME: &str = "about:blank";

/// A browser-style navigation component that manages page history and
/// dispatches [`RenderMedia`] events to its [`RenderTargets`].
///
/// History works like a browser: navigating to a new URL truncates any
/// forward entries and appends the new URL.  [`Navigator::back`] and
/// [`Navigator::forward`] move through that stack without making new
/// network requests unless the cursor actually moves.
#[derive(Debug, Clone, Component)]
#[component(on_add = on_add)]
pub struct Navigator {
	user_agent: Cow<'static, str>,
	/// Media types accepted by this navigator, in preference order.
	accepts: Vec<MediaType>,
	/// `true` while a request is in-flight.
	loading: bool,
	home_url: Url,
	/// Visited URLs, oldest first.
	history: VecDeque<Url>,
	/// Index into `history` for the currently displayed page.
	history_cursor: usize,
}

fn on_add(mut world: DeferredWorld, cx: HookContext) {
	world
		.commands()
		.entity(cx.entity)
		.queue_async(async |entity| {
			let home_url =
				entity.get(|nav: &Navigator| nav.home_url.clone()).await?;
			Navigator::navigate_to(entity, home_url).await
		});
}

impl Default for Navigator {
	fn default() -> Self {
		let home = Url::parse(DEFAULT_HOME);
		// let mut history = VecDeque::new();
		// history.push_back(home.clone());
		Self {
			user_agent: "Mozilla/5.0 Beet/0.1".into(),
			home_url: home.clone(),
			accepts: vec![
				// prefer markdown — beet skips scripts/styles so less content
				// means faster rendering
				MediaType::Markdown,
				MediaType::Html,
			],
			loading: false,
			// home navigated to by on_add
			history: default(),
			history_cursor: 0,
		}
	}
}

impl Navigator {
	pub fn new(home_url: impl Into<Url>) -> Self {
		Self {
			home_url: home_url.into(),
			..default()
		}
	}

	/// The URL currently being displayed (or loading).
	pub fn current_url(&self) -> &Url {
		// if history is empty (eg before on_add runs), fall back to home_url
		self.history
			.get(self.history_cursor)
			.unwrap_or(&self.home_url)
	}

	pub fn is_loading(&self) -> bool { self.loading }

	/// `true` when there is at least one entry behind the cursor.
	pub fn can_go_back(&self) -> bool { self.history_cursor > 0 }

	/// `true` when there is at least one entry ahead of the cursor.
	pub fn can_go_forward(&self) -> bool {
		self.history_cursor + 1 < self.history.len()
	}

	/// Resolve `url` against the current page, handling relative paths.
	fn resolve(&self, url: Url) -> Url {
		if url.authority().is_none() {
			let mut resolved = self.current_url().clone();
			resolved.set_path(url.path().clone());
			resolved
		} else {
			url
		}
	}

	/// Push a resolved URL onto the history stack, truncating any forward
	/// entries and enforcing [`HISTORY_LIMIT`].
	fn push_history(&mut self, url: Url) {
		// drop forward entries
		self.history.truncate(self.history_cursor + 1);
		self.history.push_back(url);
		// evict oldest entries when the limit is exceeded
		while self.history.len() > HISTORY_LIMIT {
			self.history.pop_front();
		}
		self.history_cursor = self.history.len() - 1;
	}

	// ------------------------------------------------------------------ //
	// Async commands
	// ------------------------------------------------------------------ //

	/// Navigate to `url`, pushing it onto the history stack.
	///
	/// ## Errors
	/// - No [`Navigator`] found on entity
	/// - Network request failed
	/// - No [`RenderTargets`] found
	pub async fn navigate_to(
		entity: AsyncEntity,
		url: impl Into<Url>,
	) -> Result {
		let url = url.into();
		// resolve relative url and push history
		let (user_agent, resolved, accepts) = entity
			.get_mut(move |mut nav: Mut<Navigator>| {
				nav.loading = true;
				let resolved = nav.resolve(url);
				nav.push_history(resolved.clone());
				(nav.user_agent.clone(), resolved, nav.accepts.clone())
			})
			.await?;

		Self::fetch_and_render(entity, user_agent, resolved, accepts).await
	}

	/// Navigate one step back in history, if possible.
	pub async fn back(entity: AsyncEntity) -> Result {
		let nav_state = entity
			.get_mut(|mut nav: Mut<Navigator>| {
				if !nav.can_go_back() {
					return None;
				}
				nav.history_cursor -= 1;
				nav.loading = true;
				Some((
					nav.user_agent.clone(),
					nav.current_url().clone(),
					nav.accepts.clone(),
				))
			})
			.await?;

		if let Some((user_agent, url, accepts)) = nav_state {
			Self::fetch_and_render(entity, user_agent, url, accepts).await?;
		}
		Ok(())
	}

	/// Navigate one step forward in history, if possible.
	pub async fn forward(entity: AsyncEntity) -> Result {
		let nav_state = entity
			.get_mut(|mut nav: Mut<Navigator>| {
				if !nav.can_go_forward() {
					return None;
				}
				nav.history_cursor += 1;
				nav.loading = true;
				Some((
					nav.user_agent.clone(),
					nav.current_url().clone(),
					nav.accepts.clone(),
				))
			})
			.await?;

		if let Some((user_agent, url, accepts)) = nav_state {
			Self::fetch_and_render(entity, user_agent, url, accepts).await?;
		}
		Ok(())
	}

	/// Shared fetch → render → clear-loading path used by all navigation
	/// methods.
	async fn fetch_and_render(
		entity: AsyncEntity,
		user_agent: Cow<'static, str>,
		url: Url,
		accepts: Vec<MediaType>,
	) -> Result {
		let media_bytes = Request::get(url)
			.with_header::<header::UserAgent>(user_agent)
			.with_header::<header::Accept>(accepts)
			.send()
			.await?
			.into_result()
			.await?
			.media_bytes()
			.await?;

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

		entity
			.get_mut(|mut nav: Mut<Navigator>| {
				nav.loading = false;
			})
			.await?;

		Ok(())
	}
}

/// Triggered on each [`RenderedBy`] entity when the [`Navigator`] visits a
/// new page.
#[derive(Debug, Clone, EntityEvent)]
pub struct RenderMedia {
	entity: Entity,
	media_bytes: MediaBytes<'static>,
}
impl std::ops::Deref for RenderMedia {
	type Target = MediaBytes<'static>;
	fn deref(&self) -> &Self::Target { &self.media_bytes }
}

/// Assigned to a [`Navigator`], listing entities that should render on each
/// url visit.
#[derive(Debug, Clone, Component)]
#[relationship_target(relationship = RenderedBy)]
pub struct RenderTargets(Vec<Entity>);

/// Assigned to a render entity (eg [`TuiNodeRenderer`]) to make it a target
/// of a [`Navigator`]'s render calls.
#[derive(Debug, Clone, Component)]
#[relationship(relationship_target = RenderTargets)]
pub struct RenderedBy(pub Entity);
