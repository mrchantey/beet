//! [`DataPage`], an answer that is both a scene and the data it shows.
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_ui::prelude::Snippet;

/// A route's answer as a scene and the data it shows, so a person and a tool
/// read the same answer: rendered as the request accepts when that is a
/// markup or text type, ie markdown for an agent, ANSI in a terminal or HTML
/// in a browser, and the data serialized when it is a serde format, ie
/// `application/json`. The first accepted type either way decides, and no
/// `Accept` at all renders the scene.
///
/// ```no_run
/// # // the action macro reaches its own crate through `crate::prelude`
/// # use beet_router::prelude;
/// # use beet_router::prelude::*;
/// # use beet_action::prelude::*;
/// # use beet_core::prelude::*;
/// # use beet_net::prelude::*;
/// # use beet_ui::prelude::*;
/// #[action(route = "count")]
/// #[derive(Default, Component, Reflect)]
/// #[reflect(Component, Default)]
/// async fn Count(cx: ActionContext<Request>) -> Result<DataPage<u32>> {
/// 	DataPage::new(&cx.caller, rsx! { <p>"three"</p> }, 3).await
/// }
/// # fn main() {}
/// ```
pub struct DataPage<T> {
	/// The scene, an ephemeral render root.
	page: PageRequest,
	/// What it shows.
	data: T,
	/// The status the answer carries.
	status: StatusCode,
}

/// Marker for the [`IntoResponseWithRequestParts`] impl of [`DataPage`].
#[derive(TypePath)]
pub struct DataPageMarker;

impl<T: 'static + Send + Sync + Serialize> DataPage<T> {
	/// Builds `scene` through the template substrate as an ephemeral render
	/// root beside `data`, answered with a success.
	pub async fn new(
		caller: &AsyncEntity,
		scene: impl 'static + Send + Sync + Bundle,
		data: T,
	) -> Result<Self> {
		let page = caller
			.world()
			.with(move |world: &mut World| -> Result<PageRequest> {
				let mut entity =
					world.spawn_template(Snippet::from_bundle(scene))?;
				let id = entity.id();
				PageRoot::insert(&mut entity, vec![id]);
				Ok(PageRequest(id))
			})
			.await?;
		Self {
			page,
			data,
			status: StatusCode::OK,
		}
		.xok()
	}

	/// The answer with `status`, ie a refusal a process exits 1 on.
	pub fn with_status(mut self, status: StatusCode) -> Self {
		self.status = status;
		self
	}

	/// The data the scene shows.
	pub fn data(&self) -> &T { &self.data }
}

impl<T: 'static + Send + Sync + Serialize>
	IntoResponseWithRequestParts<DataPageMarker> for DataPage<T>
{
	fn into_response_with_request_parts(
		self,
		caller: AsyncEntity,
		parts: RequestParts,
	) -> MaybeSendBoxedFuture<'static, Result<Response>> {
		Box::pin(async move {
			let serialized = parts
				.accept()
				.into_iter()
				.find(|media_type| {
					media_type.is_serializable()
						|| media_type.is_text()
						|| media_type.is_wildcard()
				})
				.filter(|media_type| media_type.is_serializable());
			match serialized {
				Some(media_type) => {
					let page = self.page.0;
					caller
						.world()
						.with(move |world| {
							if let Ok(entity) = world.get_entity_mut(page) {
								entity.despawn();
							}
						})
						.await;
					let bytes = media_type.serialize_with_options(
						&self.data,
						SerializeOptions { pretty: true },
					)?;
					Response::ok()
						.with_media(MediaBytes::new(media_type, bytes))
						.with_status(self.status)
						.xok()
				}
				None => LivePage::respond(self.page.0, &caller, parts)
					.await?
					.with_status(self.status)
					.xok(),
			}
		})
	}

	/// The scene renders as any page does, so it takes the render params a
	/// page takes.
	fn params() -> ParamsDeclaration {
		<PageRequest as IntoResponseWithRequestParts<PageRequest>>::params()
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	#[action(route = "count")]
	#[derive(Default, Component, Reflect)]
	#[reflect(Component, Default)]
	async fn Count(cx: ActionContext<Request>) -> Result<DataPage<u32>> {
		DataPage::new(&cx.caller, rsx! { <p>"three"</p> }, 3)
			.await
			.map(|page| page.with_status(StatusCode::UNPROCESSABLE_CONTENT))
	}

	/// A data page answers markdown with its scene and JSON with its data.
	#[beet_core::test]
	async fn answers_its_scene_or_its_data() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let root = world
			.spawn((Router::with_defaults(), children![Count]))
			.flush();
		let mut answer = async |accept: MediaType| {
			world
				.entity_mut(root)
				.exchange(
					Request::get("count")
						.with_header::<header::Accept>(vec![accept]),
				)
				.await
		};
		let markdown = answer(MediaType::Markdown).await;
		markdown
			.status()
			.xpect_eq(StatusCode::UNPROCESSABLE_CONTENT);
		markdown.text().await.unwrap().xpect_eq("three\n");
		answer(MediaType::Json)
			.await
			.text()
			.await
			.unwrap()
			.xpect_eq("3");
	}

	/// The scene takes the render params any page takes, so a strict server
	/// accepts them on a data page's route.
	#[beet_core::test]
	async fn takes_the_render_params() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let server = world
			.spawn((CliServer::default(), children![(Router, children![
				Count
			])]))
			.flush();
		let router = world.entity(server).get::<Children>().unwrap()[0];
		world
			.entity_mut(router)
			.exchange(
				Request::get("count")
					.with_param("frontmatter", "toml")
					.with_param("root", "content")
					.with_header::<header::Accept>(vec![MediaType::Markdown]),
			)
			.await
			.status()
			.xpect_eq(StatusCode::UNPROCESSABLE_CONTENT);
	}

	/// A data page's scene renders exactly as a plain page holding the same
	/// scene does, under every markup and text `Accept`, so the two answers
	/// cannot drift.
	#[beet_core::test]
	async fn renders_its_scene_as_a_plain_page_does() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let root = world
			.spawn((Router, children![
				Count,
				(PathPartial::new("plain"), FixedPage, children![rsx! {
					<p>"three"</p>
				}]),
			]))
			.flush();
		for accept in [
			MediaType::Markdown,
			MediaType::Html,
			MediaType::Text,
			MediaType::AnsiTerm,
		] {
			let mut body = async |path: &str| {
				world
					.entity_mut(root)
					.exchange(
						Request::get(path).with_header::<header::Accept>(vec![
							accept.clone(),
						]),
					)
					.await
					.text()
					.await
					.unwrap()
			};
			let page = body("count").await;
			page.clone().xpect_contains("three");
			page.xpect_eq(body("plain").await);
		}
	}
}
