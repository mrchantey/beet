use axum::Router;
use crate::net::RouteInfo;
use bytes::Bytes;
use http::StatusCode;
use http_body_util::BodyExt;
use serde::de::DeserializeOwned;
use tower::ServiceExt;


pub struct AxumExt;



impl AxumExt {
	pub async fn collect_response(
		res: axum::response::Response,
	) -> Result<http::Response<Bytes>, axum::Error> {
		let (parts, body) = res.into_parts();

		let body = body.collect().await.map(|b| b.to_bytes())?;

		let res = http::Response::from_parts(parts, body);
		Ok(res)
	}
}


#[extend::ext(name=BeetRouterExt)]
#[allow(async_fn_in_trait)]
pub impl Router {
	async fn oneshot_str(
		&mut self,
		req: impl Into<RouteInfo>,
	) -> anyhow::Result<String> {
		let req = req.into().into_request(String::new())?;
		let res = self.oneshot(req).await?;
		if res.status() != StatusCode::OK {
			anyhow::bail!("Expected status code 200 OK, got {}", res.status());
		}
		let body = res.into_body().collect().await?.to_bytes();
		let res = String::from_utf8(body.to_vec())?;
		Ok(res)
	}
	async fn oneshot_json<T: DeserializeOwned>(
		&mut self,
		req: impl Into<RouteInfo>,
	) -> anyhow::Result<T> {
		let req = req.into().into_request(String::new())?;
		let res = self.oneshot(req).await?;
		if res.status() != StatusCode::OK {
			anyhow::bail!("Expected status code 200 OK, got {}", res.status());
		}
		let body = res.into_body().collect().await?.to_bytes();
		let res: T = serde_json::from_slice(&body)?;
		Ok(res)
	}
	// fn oneshot_with_body(&mut self, req:impl RouteInfo)
}
