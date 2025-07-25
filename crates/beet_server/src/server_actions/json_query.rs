use bevy::prelude::*;
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
#[derive(Resource)]
pub struct JsonQuery<T>(pub T);

impl<T> JsonQuery<T> {
	/// Consume the `JsonQuery` and return the inner value
	pub fn into_inner(self) -> T { self.0 }
}

use axum::extract::FromRequestParts;
use axum::extract::Query;
use http::request::Parts;
use serde::de::DeserializeOwned;
use serde_json::Error as JsonError;
use std::collections::HashMap;

// #[async_trait]
impl<T, S> FromRequestParts<S> for JsonQuery<T>
where
	T: DeserializeOwned,
	S: Send + Sync,
{
	type Rejection = JsonQueryRejection;

	async fn from_request_parts(
		parts: &mut Parts,
		state: &S,
	) -> Result<Self, Self::Rejection> {
		// Extract the query parameters
		let Query(params): Query<HashMap<String, String>> =
			Query::from_request_parts(parts, state)
				.await
				.map_err(JsonQueryRejection::QueryExtractionError)?;

		// Look for the 'data' parameter
		let data = params
			.get("data")
			.ok_or(JsonQueryRejection::MissingDataParam)?;

		// Parse the JSON string into the desired type
		let value = serde_json::from_str(data)
			.map_err(JsonQueryRejection::JsonParseError)?;

		Ok(JsonQuery(value))
	}
}

/// Possible rejection types for the `JsonQuery` extractor
#[derive(Debug, thiserror::Error)]
pub enum JsonQueryRejection {
	/// Failed to extract query parameters
	#[error("Failed to extract query parameters: {0}")]
	QueryExtractionError(axum::extract::rejection::QueryRejection),
	/// The 'data' query parameter is missing
	#[error("Missing 'data' query parameter")]
	MissingDataParam,
	/// Failed to parse the JSON in the 'data' parameter
	#[error("Failed to parse JSON: {0}")]
	JsonParseError(JsonError),
}

// If you want to integrate with axum's error handling
impl axum::response::IntoResponse for JsonQueryRejection {
	fn into_response(self) -> axum::response::Response {
		let status = match self {
			Self::QueryExtractionError(_) => {
				axum::http::StatusCode::BAD_REQUEST
			}
			Self::MissingDataParam => axum::http::StatusCode::BAD_REQUEST,
			Self::JsonParseError(_) => axum::http::StatusCode::BAD_REQUEST,
		};

		let body = match self {
			Self::QueryExtractionError(e) => {
				format!("Failed to extract query parameters: {}", e)
			}
			Self::MissingDataParam => {
				"Missing 'data' query parameter".to_string()
			}
			Self::JsonParseError(e) => format!("Failed to parse JSON: {}", e),
		};

		(status, body).into_response()
	}
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
