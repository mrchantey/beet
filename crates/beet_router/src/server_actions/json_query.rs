
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
mod test {
	use crate::prelude::*;
	use axum::response::IntoResponse;
	use http::Method;
	use http::Request;
	use http_body_util::BodyExt;
	use tower::ServiceExt;

	fn req(a: i32, b: i32) -> Request<String> {
		Request::builder()
			.uri(format!("/?data=[{a},{b}]"))
			.method(Method::GET)
			.body(Default::default())
			.unwrap()
	}

	async fn add(data: JsonQuery<(i32, i32)>) -> impl IntoResponse {
		let (a, b) = data.into_inner();
		(a + b).to_string()
	}

	#[sweet::test]
	async fn works() {
		use axum::Router;
		use axum::routing::get;
		let router = Router::<()>::new().route("/", get(add));

		let resp = router.oneshot(req(1, 2)).await.unwrap();
		let body = resp.into_body().collect().await.unwrap().to_bytes();
		let res = String::from_utf8(body.to_vec()).unwrap();
		assert_eq!(res, "3");
	}
}
