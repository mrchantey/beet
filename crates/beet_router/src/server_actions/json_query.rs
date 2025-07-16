/// An extractor that extracts JSON data from the `data` query parameter.
///
/// Similar to `axum::extract::Json`, but works on bodyless requests like GET by pulling
/// data from a query parameter instead of the request body.
///
/// ## Example
/// ```sh
/// curl -X GET "http://localhost:3000/?data={\"key\":\"value\"}"
///
/// ```
pub struct JsonQuery<T>(pub T);

impl<T> JsonQuery<T> {
	/// Consume the `JsonQuery` and return the inner value
	pub fn into_inner(self) -> T { self.0 }
}



#[cfg(test)]
#[cfg(all(feature = "axum", not(target_arch = "wasm32")))]
mod test {
	use crate::prelude::*;
	use axum::response::IntoResponse;
	use beet_core::prelude::*;
	use sweet::prelude::*;


	async fn add(data: JsonQuery<(i32, i32)>) -> impl IntoResponse {
		let (a, b) = data.into_inner();
		(a + b).to_string()
	}

	#[sweet::test]
	async fn works() {
		use axum::Router;
		use axum::routing::get;
		Router::<()>::new()
			.route("/", get(add))
			.oneshot_str("/?data=[1,2]")
			.await
			.unwrap()
			.xpect()
			.to_be("3");
	}
}
