//! Serving a [`TableStore`] over the table wire protocol.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// The greedy segment [`ServeTables`] captures the protocol path into.
const TABLE_PATH_PARAM: &str = "table_path";

/// Mounts the [`TableWire`] protocol under `prefix`, serving the nearest
/// self-or-ancestor [`TableStore`]: its rows and blobs, every partition of
/// it, to a [`TableWireClient`] in another process.
///
/// Beside [`ServeBlobs`] rather than merged with it: that route serves a
/// store as a static host does (an `index.html` for a directory, a redirect
/// to a public url), this one serves rows and blobs exactly, every write
/// guarded by its precondition, so the same bucket served both ways is a
/// site and a database. Anything that can reach the mount can write the
/// store, so it belongs behind whatever authenticates its callers.
///
/// ```html
/// <SqliteStore path="data.db">
/// 	<ServeTables prefix="tables"/>
/// </SqliteStore>
/// ```
#[template]
pub fn ServeTables(
	/// The mount path the protocol is served under, eg `tables`.
	#[prop]
	prefix: String,
) -> impl Bundle {
	(
		PathPartial::new(format!("{prefix}/*{TABLE_PATH_PARAM}?")),
		ServeTablesHandler,
		route::exchange_overload::<Request, Response, _, _>(),
	)
}

/// The handler behind [`ServeTables`]: answers the captured protocol path
/// from the nearest self-or-ancestor [`TableStore`].
#[action]
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component)]
pub(crate) async fn ServeTablesHandler(
	cx: ActionContext<Request>,
) -> Result<Response> {
	let store = cx
		.caller
		.with_state::<AncestorQuery<&TableStore>, Result<TableStore>>(
			|entity, stores| stores.get(entity).cloned(),
		)
		.await??;
	let path = cx
		.input
		.request_parts()
		.get_params(TABLE_PATH_PARAM)
		.cloned()
		.unwrap_or_default();
	TableWire::serve_path(store.provider(), &path, cx.input)
		.await
		.xok()
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	/// The mounted route answers the protocol below its prefix: a put, a
	/// read, a failed precondition and a partition kept apart.
	#[beet_core::test]
	async fn serves_the_protocol() {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let router = world
			.spawn((Router, InMemoryStore::new(), children![
				ServeTables {
					prefix: "tables".into()
				}
				.into_snippet_bundle()
			]))
			.flush();
		let row = StoredRow::new(
			"k",
			Row::from_value(value!({ "key": "k", "value": 7 })).unwrap(),
		)
		.unwrap();
		let put = || {
			Request::put("/tables/named_rows/k").with_body(
				serde_json::to_vec(&serde_json::json!({ "row": row })).unwrap(),
			)
		};
		let mut exchange = async |request: Request| {
			world.entity_mut(router).exchange(request).await
		};
		exchange(put())
			.await
			.status()
			.xpect_eq(StatusCode::NO_CONTENT);
		exchange(Request::get("/tables/named_rows/k"))
			.await
			.json::<StoredRow>()
			.await
			.unwrap()
			.xpect_eq(row.clone());
		exchange(put().with_header_raw("if-none-match", "*"))
			.await
			.status()
			.xpect_eq(StatusCode::PRECONDITION_FAILED);
		exchange(Request::get("/tables/@lobby/named_rows/k"))
			.await
			.status()
			.xpect_eq(StatusCode::NOT_FOUND);
	}
}
