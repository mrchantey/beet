//! A seed post read from a route's answer.
use super::request_post::fill;
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_router::prelude::*;

/// A seed post whose text is a route's answer: a `GET` of `path` exchanged
/// with the url space the thread is attached in, asking for
/// `text/markdown`, so a seed is whatever a verb reports, ie the rows of a
/// table:
///
/// ```bsx
/// <Route path="coach/:claim" {ThreadRoute{template:"CoachThread"}}/>
/// <!-- CoachThread.bsx -->
/// <CreateActor name="System" kind="System">
///     <RoutePost path="eval/rows?table=claims&key={claim}"/>
/// </CreateActor>
/// ```
///
/// Each `{param}` of `path` is filled from the request the thread is built
/// for, as [`RequestPost`] fills its text, read while the template builds;
/// the exchange is a [`SeedSource`], read once the thread is attached.
///
/// ## Errors
///
/// Errors when `path` names a param outside a request build, or one the
/// request lacks. A non-OK answer is a [`TemplateError`] naming the path, so
/// the thread never reduces.
#[template(system)]
pub fn RoutePost(
	/// The path of the route, each `{param}` the request's value for it.
	#[prop(required)]
	path: String,
	stack: Res<RequestContextStack>,
) -> Result<impl Bundle> {
	let path = match stack.try_current() {
		Some(cx) => fill(&path, cx.parts())?,
		None if path.contains('{') => bevybail!(
			"`<RoutePost path=\"{path}\"/>` fills its path from a request, \
			 but is built outside one: it seeds a thread a route builds per \
			 request, ie a `ThreadRoute`"
		),
		None => path,
	};
	SeedSource::route(path).xok()
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
				async move |cx: ActionContext| -> Result<String> {
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

	/// A host url space over a temp store with the `notes` verb and a route
	/// building a thread seeded from `path`, its records under `threads`.
	fn host(path: &str) -> (App, Entity, BlobStore) {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.install_type::<ThreadPlugin>();
		app.world_mut()
			.get_resource_or_init::<BsxTemplateRegistry>()
			.insert_source(
				"NotesThread",
				&format!(
					r#"
<Thread {{(Sequence, MountThreadStore{{path:"threads"}})}}>
	<CreateActor name="System" kind="System" id=1><RoutePost path="{path}"/></CreateActor>
	<CreateActor name="Agent" kind="Agent" id=2 {{MockPostStreamer{{custom_response:"ok"}}}}/>
</Thread>
"#
				),
			)
			.unwrap();
		let store = BlobStore::temp();
		let router = BsxTemplate::parse_entry(
			app.world(),
			r#"<Router><Route path="review/:document" {ThreadRoute{template:"NotesThread"}}/></Router>"#,
		)
		.unwrap()
		.spawn(app.world_mut())
		.unwrap();
		app.world_mut().entity_mut(router).insert(store.clone());
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
		let (mut app, router, store) = host("notes?about={document}");
		review(&mut app, router).await.0.xpect_eq(StatusCode::OK);
		let records =
			BlobThreadStore::new(store.with_subdir(RelPath::new("threads")));
		let thread = records.threads().await.unwrap()[0].id();
		records.thread_posts(thread, None).await.unwrap()[0]
			.to_string()
			.xpect_eq("notes on product");
	}

	/// A route answering anything but OK fails the build, naming the path.
	#[beet_core::test]
	async fn a_refusal_fails_the_build() {
		let (mut app, router, _) = host("nowhere");
		let (status, text) = review(&mut app, router).await;
		status.xpect_eq(StatusCode::INTERNAL_SERVER_ERROR);
		text.xpect_contains("the seed route `nowhere` answered 404");
	}
}
