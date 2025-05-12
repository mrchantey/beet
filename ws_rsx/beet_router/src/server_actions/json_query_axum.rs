use super::JsonQuery;
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
