use bytes::Bytes;
use http_body_util::BodyExt;


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
