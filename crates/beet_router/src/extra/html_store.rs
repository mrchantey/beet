use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Serves prebuilt HTML from a [`BlobStore`], gating live rendering.
///
/// Add it alongside a [`router`](crate::prelude::router). It requires the
/// [`HtmlStoreAction`] middleware, which in `ssg_mode` serves
/// `<path>/index.html` from `store` and only renders live on a store miss; with
/// `ssg_mode` off it always renders live.
#[derive(Clone, Component)]
#[require(HtmlStoreAction)]
pub struct HtmlStore {
	/// When `true`, serve prebuilt HTML from `store`, rendering live only on a
	/// miss. When `false`, always render live.
	pub ssg_mode: bool,
	/// The store prebuilt HTML is served from.
	pub store: BlobStore,
}

impl HtmlStore {
	/// Serve prebuilt HTML from `store`, rendering live only on a store miss.
	pub fn ssg(store: BlobStore) -> Self {
		Self {
			ssg_mode: true,
			store,
		}
	}
	/// Always render live; `store` is retained for flipping back to `ssg`.
	pub fn ssr(store: BlobStore) -> Self {
		Self {
			ssg_mode: false,
			store,
		}
	}
}

/// Middleware that serves prebuilt HTML from an ancestor [`HtmlStore`] while
/// its `ssg_mode` is set, otherwise passes through to live rendering.
#[action]
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component)]
#[component(on_add = on_add_middleware::<Self, Request, Response>)]
pub async fn HtmlStoreAction(
	cx: ActionContext<(Request, Next<Request, Response>)>,
) -> Result<Response> {
	let caller = cx.caller.clone();
	let (request, next) = cx.take();

	let store = caller
		.with_state::<AncestorQuery<&HtmlStore>, Option<BlobStore>>(
			|entity, query| {
				query
					.get(entity)
					.ok()
					.filter(|html| html.ssg_mode)
					.map(|html| html.store.clone())
			},
		)
		.await?;
	// no ancestor store, or live rendering requested
	let Some(store) = store else {
		return next.call(request).await;
	};

	// serve the prebuilt file, falling through to live rendering on a miss
	match serve_blob(&store, &RelPath::from(request.path())).await {
		Ok(response) => Ok(response),
		Err(_) => next.call(request).await,
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	async fn html_store() -> BlobStore {
		let store = BlobStore::temp();
		store
			.insert(&RelPath::from("about/index.html"), "<p>prebuilt</p>")
			.await
			.unwrap();
		store
	}

	#[beet_core::test]
	async fn ssg_serves_prebuilt() {
		(AsyncPlugin, RouterPlugin)
			.into_world()
			.spawn((router(), HtmlStore::ssg(html_store().await), children![
				fixed_scene("about", rsx! { <p>"live about"</p> })
			]))
			.call::<Request, Response>(Request::get("about"))
			.await
			.unwrap()
			.unwrap_str()
			.await
			.xpect_contains("prebuilt")
			.xnot()
			.xpect_contains("live about");
	}

	#[beet_core::test]
	async fn ssr_renders_live() {
		(AsyncPlugin, RouterPlugin)
			.into_world()
			.spawn((router(), HtmlStore::ssr(html_store().await), children![
				fixed_scene("about", rsx! { <p>"live about"</p> })
			]))
			.call::<Request, Response>(Request::get("about"))
			.await
			.unwrap()
			.unwrap_str()
			.await
			.xpect_contains("live about");
	}

	#[beet_core::test]
	async fn ssg_falls_through_on_miss() {
		(AsyncPlugin, RouterPlugin)
			.into_world()
			.spawn((router(), HtmlStore::ssg(html_store().await), children![
				fixed_scene("contact", rsx! { <p>"live contact"</p> })
			]))
			.call::<Request, Response>(Request::get("contact"))
			.await
			.unwrap()
			.unwrap_str()
			.await
			.xpect_contains("live contact");
	}
}
