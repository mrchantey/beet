//! A seed post read from a route's answer.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// A seed post whose text is a route's answer: a `GET` of `path` exchanged
/// with the url space the thread is attached in, asking for
/// `text/markdown`, so a seed is whatever a verb reports, ie the rows of a
/// table:
///
/// ```bsx
/// <Route path="coach/:claim" {ThreadRoute{source:Document("templates/CoachThread.bsx")}}/>
/// <!-- CoachThread.bsx -->
/// <CreateActor name="System" kind="System">
///     <RoutePost path="eval/rows?table=claims&key={claim}"/>
/// </CreateActor>
/// ```
///
/// A [`SeedSource`], so the route is called by the entry creating the
/// thread and by nothing else, each `{param}` of `path` filled from the
/// request entering ([`ThreadEntry::fill`]).
///
/// ## Errors
///
/// The entry creating the thread fails when `path` names a param the
/// request lacks, or the route answers anything but OK, naming the path.
#[template]
pub fn RoutePost(
	/// The path of the route, each `{param}` the request's value for it.
	#[prop(required)]
	path: SmolStr,
) -> impl Bundle {
	SeedSource::route(path)
}

impl SeedSource {
	/// A seed reading the answer of the route at `path` in the url space at
	/// or above the thread, see [`RoutePost`].
	pub fn route(path: impl Into<SmolStr>) -> Self {
		let path: SmolStr = path.into();
		let label = format!("<RoutePost path=\"{path}\">");
		Self::new(
			label,
			Action::new_async(
				async move |cx: ActionContext<ThreadEntry>| -> Result<String> {
					let path = cx.input.fill(&path)?;
					let space = cx
						.caller
						.with_state::<(Query<&ChildOf>, Query<&PathPartial>), _>(
							|entity, (parents, partials)| {
								PathPattern::namespace_root(
									entity, &parents, &partials,
								)
							},
						)
						.await?;
					let response = cx
						.world()
						.entity(space)
						.exchange(
							Request::get(path.as_str())
								.with_accept(MediaType::Markdown),
						)
						.await;
					let status = response.status();
					let text = response.text().await?;
					match status.is_success() {
						true => text.xok(),
						false => bevybail!(
							"the seed route `{path}` answered {status}: {}",
							text.trim()
						),
					}
				},
			),
		)
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use beet_scene::prelude::*;

	/// Stands in for a verb reporting on a subject.
	#[action(route = "notes")]
	#[derive(Default, Clone, Component, Reflect)]
	#[reflect(Component)]
	async fn Notes(
		/// The subject.
		#[param]
		about: String,
	) -> Result<MediaBytes> {
		MediaBytes::new_text(format!("notes on {about}")).xok()
	}

	/// A url space over a temp store with the `notes` verb and a route
	/// building a thread seeded from `path`, its records under `threads`.
	async fn url_space(path: &str) -> (App, Entity, BlobStore) {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.install_types::<ThreadPlugin>();
		let store = BlobStore::new(InMemoryBlobStore::new());
		store
			.insert(
				&RelPath::new("NotesThread.bsx"),
				format!(
					r#"
<Thread {{(Sequence, MountThreadStore{{path:"threads"}})}}>
	<CreateActor name="System" kind="System" id=1><RoutePost path="{path}"/></CreateActor>
	<CreateActor name="Agent" kind="Agent" id=2 {{MockPostStreamer{{custom_response:"ok"}}}}/>
</Thread>
"#
				),
			)
			.await
			.unwrap();
		let body = BsxParser::body(
			app.world(),
			r#"<Router><Route path="review/:document" {ThreadRoute{source:Document("NotesThread.bsx")}}/></Router>"#,
		)
		.unwrap();
		let router = app.world_mut().spawn_template(body).unwrap().id();
		app.world_mut().entity_mut(router).insert((
			store.clone(),
			JsonTableStore::over(store.clone()).unwrap(),
		));
		app.world_mut()
			.spawn((Notes::default(), ChildOf(router)))
			.flush();
		(app, router, store)
	}

	/// The answer to `review/product`, its status and text.
	async fn review(app: &mut App, router: Entity) -> (StatusCode, String) {
		let response = app
			.world_mut()
			.entity_mut(router)
			.exchange(Request::get("review/product"))
			.await;
		(response.status(), response.text().await.unwrap())
	}

	/// A seed is the answer of a route, its path filled from the request.
	#[beet_core::test]
	async fn seeds_from_a_route() {
		let (mut app, router, store) =
			url_space("notes?about={document}").await;
		review(&mut app, router).await.0.xpect_eq(StatusCode::OK);
		let records = ThreadStore::new(
			JsonTableStore::over(store).unwrap(),
			RelPath::new("threads"),
		);
		let thread = records.threads().await.unwrap()[0].id();
		records.posts(thread).await.unwrap()[0]
			.to_string()
			.xpect_eq("notes on product");
	}

	/// A route answering anything but OK fails the entry, naming the path.
	#[beet_core::test]
	async fn a_refusal_fails_the_entry() {
		let (mut app, router, _) = url_space("nowhere").await;
		let (status, text) = review(&mut app, router).await;
		status.xpect_eq(StatusCode::INTERNAL_SERVER_ERROR);
		text.xpect_contains("the seed route `nowhere` answered 404");
	}
}
