use axum::Router;
use beet_core::prelude::*;
use beet_net::exports::http;
use beet_net::exports::http_body_util::BodyExt;
use beet_net::prelude::*;
use bevy::prelude::*;
use bytes::Bytes;
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


#[extend::ext(name=AxumRouterExt)]
#[allow(async_fn_in_trait)]
pub impl Router {
	async fn oneshot_res(
		&mut self,
		req: impl Into<RouteInfo>,
	) -> Result<Response> {
		let req = req.into().into_request(String::new())?;
		let res = self.oneshot(req).await?;
		let res = Response::from_axum(res).await?;
		Ok(res)
	}
	async fn oneshot_str(
		&mut self,
		req: impl Into<RouteInfo>,
	) -> Result<String> {
		let req = req.into().into_request(String::new())?;
		let res = self.oneshot(req).await?;
		if res.status() != StatusCode::OK {
			bevybail!("Expected status code 200 OK, got {}", res.status());
		}
		let body = res.into_body().collect().await?.to_bytes();
		let res = String::from_utf8(body.to_vec())?;
		Ok(res)
	}
	#[cfg(feature = "serde")]
	async fn oneshot_json<T: serde::de::DeserializeOwned>(
		&mut self,
		req: impl Into<RouteInfo>,
	) -> Result<T> {
		let req = req.into().into_request(String::new())?;
		let res = self.oneshot(req).await?;
		if res.status() != StatusCode::OK {
			bevybail!("Expected status code 200 OK, got {}", res.status());
		}
		let body = res.into_body().collect().await?.to_bytes();
		let res: T = serde_json::from_slice(&body)?;
		Ok(res)
	}
	// fn oneshot_with_body(&mut self, req:impl RouteInfo)
}
