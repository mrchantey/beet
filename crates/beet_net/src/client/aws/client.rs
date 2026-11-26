use crate::prelude::*;
use beet_core::prelude::*;
use bytes::Bytes;
use std::sync::Arc;

/// One AWS service in one region, called as signed requests through
/// [`Request::send`]: no SDK, no runtime and no `aws` cli, so it builds on
/// every target, a Cloudflare Worker included.
///
/// A request is signed as the client's [`AwsCredentialSource`], the
/// launch's own ([`AwsCredentialSource::default_chain`]) unless it is given
/// another, through [`AwsAuth`]; [`anonymous`](Self::anonymous) sends
/// unsigned, [`with_auth`](Self::with_auth) through any other
/// [`RequestAuth`]. The protocol helpers are one per wire format:
/// [`call_json`](Self::call_json) for `awsJson`,
/// [`call_query`](Self::call_query) for `awsQuery` and `ec2Query`, and
/// [`send`](Self::send) with an [`AwsRequest`] for `restJson1` and
/// `restXml`.
///
/// ## Retries
///
/// Up to three attempts with jittered backoff on a transport failure, a
/// server fault (5xx) or a throttle ([`AwsError::THROTTLING`]), plus one
/// retry after a refusal the auth recovers from (a lapsed credential, a
/// skewed clock). An S3 answer naming the bucket's own region
/// (`x-amz-bucket-region` on a redirect) is retried once there.
///
/// ## Endpoints
///
/// `https://{prefix}.{region}.amazonaws.com` for a regional service, the one
/// host of a global one ([`AwsService::global`]), and for S3 the bucket's
/// virtual host, or path style for a bucket name with a dot or an endpoint
/// override ([`with_endpoint`](Self::with_endpoint): R2, MinIO, a test's
/// local server).
///
/// Nothing here logs a request body, a response body or a credential, and a
/// request carrying a secret ([`AwsRequest::with_secret`],
/// [`call_json_secret`](Self::call_json_secret)) has it redacted from any
/// error it answers.
#[derive(Clone)]
pub struct AwsClient {
	service: AwsService,
	region: SmolStr,
	endpoint: Option<SmolStr>,
	auth: ClientAuth,
	transport: Option<AwsTransport>,
}

/// How a client's requests are authorized.
#[derive(Clone)]
enum ClientAuth {
	/// Signed as a source's credential, for the client's region.
	Signed(AwsCredentialSource),
	/// Through another auth, unsigned by the client itself.
	Custom(Arc<dyn RequestAuth>),
}

/// How a request reaches AWS: [`Request::send`] unless a test installs a
/// handler.
pub type AwsTransport = Arc<
	dyn 'static
		+ Send
		+ Sync
		+ Fn(Request) -> SendBoxedFuture<Result<Response>>,
>;

impl core::fmt::Debug for AwsClient {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("AwsClient")
			.field("service", &self.service.endpoint_prefix)
			.field("region", &self.region)
			.field("endpoint", &self.endpoint)
			.finish_non_exhaustive()
	}
}

impl AwsClient {
	/// `service` in `region`, signed as the launch's own credential.
	pub fn new(service: AwsService, region: impl Into<SmolStr>) -> Self {
		Self {
			service,
			region: region.into(),
			endpoint: None,
			auth: ClientAuth::Signed(AwsCredentialSource::default_chain()),
			transport: None,
		}
	}

	/// Sign as `credentials` rather than the launch's own.
	pub fn with_credentials(
		mut self,
		credentials: impl Into<AwsCredentialSource>,
	) -> Self {
		self.auth = ClientAuth::Signed(credentials.into());
		self
	}

	/// Authorize every request through `auth` rather than signing it, ie an
	/// Identity Center token.
	pub fn with_auth(mut self, auth: impl RequestAuth) -> Self {
		self.auth = ClientAuth::Custom(Arc::new(auth));
		self
	}

	/// Send every request unsigned, ie an anonymous read of a public bucket.
	pub fn anonymous(self) -> Self { self.with_auth(NoAuth) }

	/// Send to `endpoint` (scheme and host, no trailing slash) rather than
	/// the service's public host: an S3-compatible service, or a test.
	pub fn with_endpoint(mut self, endpoint: impl AsRef<str>) -> Self {
		self.endpoint = Some(endpoint.as_ref().trim_end_matches('/').into());
		self
	}

	/// Send every request through `handler` rather than the network: a
	/// test's stand-in for AWS.
	pub fn with_transport(
		mut self,
		handler: impl 'static
		+ Send
		+ Sync
		+ Fn(Request) -> SendBoxedFuture<Result<Response>>,
	) -> Self {
		self.transport = Some(Arc::new(handler));
		self
	}

	/// The same client in `region`.
	pub fn with_region(mut self, region: impl Into<SmolStr>) -> Self {
		self.region = region.into();
		self
	}

	/// The region every request goes to.
	pub fn region(&self) -> &str { &self.region }

	/// The scheme and host requests go to: the endpoint override, else the
	/// service's public host in this region.
	pub fn endpoint_url(&self) -> String {
		match &self.endpoint {
			Some(endpoint) => endpoint.to_string(),
			None => format!("https://{}", self.service.host(&self.region)),
		}
	}

	/// The service this client calls.
	pub fn service(&self) -> &AwsService { &self.service }

	/// The credential source requests are signed as, `None` for a client
	/// authorized otherwise.
	pub fn credentials(&self) -> Option<&AwsCredentialSource> {
		match &self.auth {
			ClientAuth::Signed(source) => Some(source),
			ClientAuth::Custom(_) => None,
		}
	}

	/// Call an `awsJson` `action` with `body`, answering the response json,
	/// `null` for an empty answer.
	pub async fn call_json(&self, action: &str, body: &Value) -> Result<Value> {
		self.json_request(action, body, None).await
	}

	/// [`call_json`](Self::call_json) for a write carrying `secret`, which is
	/// redacted from any error it answers.
	pub async fn call_json_secret(
		&self,
		action: &str,
		body: &Value,
		secret: &str,
	) -> Result<Value> {
		self.json_request(action, body, Some(secret)).await
	}

	async fn json_request(
		&self,
		action: &str,
		body: &Value,
		secret: Option<&str>,
	) -> Result<Value> {
		let (content_type, target_prefix) = match self.service.protocol {
			AwsProtocol::AwsJson1_0 { target_prefix }
			| AwsProtocol::AwsJson1_1 { target_prefix } => (
				self.service
					.protocol
					.json_content_type()
					.unwrap_or_default(),
				target_prefix,
			),
			protocol => bevybail!(
				"{} speaks {protocol:?}, not an awsJson protocol",
				self.service.endpoint_prefix
			),
		};
		let mut request = AwsRequest::post(action, "/")
			.with_header("content-type", content_type)
			.with_header("x-amz-target", format!("{target_prefix}.{action}"))
			.with_body(body.to_json_string());
		request.secret = secret.map(ToString::to_string);
		let response = self.send(request).await?;
		Self::json_of(response).await
	}

	/// Call an `awsQuery` or `ec2Query` `action` with `params`, answering the
	/// `<{action}Result>` element (EC2's answer has none, so its root).
	pub async fn call_query(
		&self,
		action: &str,
		params: impl IntoIterator<Item = (impl AsRef<str>, impl AsRef<str>)>,
	) -> Result<BsxElement> {
		self.query_request(action, params, None).await
	}

	/// [`call_query`](Self::call_query) for a call carrying `secret`, which is
	/// redacted from any error it answers.
	pub async fn call_query_secret(
		&self,
		action: &str,
		params: impl IntoIterator<Item = (impl AsRef<str>, impl AsRef<str>)>,
		secret: &str,
	) -> Result<BsxElement> {
		self.query_request(action, params, Some(secret)).await
	}

	async fn query_request(
		&self,
		action: &str,
		params: impl IntoIterator<Item = (impl AsRef<str>, impl AsRef<str>)>,
		secret: Option<&str>,
	) -> Result<BsxElement> {
		let version = match self.service.protocol {
			AwsProtocol::AwsQuery { version }
			| AwsProtocol::Ec2Query { version } => version,
			protocol => bevybail!(
				"{} speaks {protocol:?}, not a query protocol",
				self.service.endpoint_prefix
			),
		};
		let body = [
			("Action".to_string(), action.to_string()),
			("Version".to_string(), version.to_string()),
		]
		.into_iter()
		.chain(params.into_iter().map(|(key, value)| {
			(key.as_ref().to_string(), value.as_ref().to_string())
		}))
		.map(|(key, value)| {
			format!("{}={}", uri_encode(&key, ""), uri_encode(&value, ""))
		})
		.collect::<Vec<_>>()
		.join("&");
		let mut request = AwsRequest::post(action, "/")
			.with_header(
				"content-type",
				"application/x-www-form-urlencoded; charset=utf-8",
			)
			.with_body(body);
		request.secret = secret.map(ToString::to_string);
		let response = self.send(request).await?;
		let root = BsxElement::parse_xml(&response.text().await?)?;
		let result = format!("{action}Result");
		match root.child(&result) {
			Some(result) => result.clone(),
			None => root,
		}
		.xok()
	}

	/// The region the launch names, `AWS_REGION` else `AWS_DEFAULT_REGION`:
	/// what a store built at the process boundary, which names no region of
	/// its own, signs for.
	pub fn launch_region() -> Option<SmolStr> {
		["AWS_REGION", "AWS_DEFAULT_REGION"]
			.iter()
			.find_map(|name| env_ext::var(name).ok())
			.filter(|region| !region.is_empty())
			.map(SmolStr::from)
	}

	/// Send a `restJson1` request, answering the response json, `null` for
	/// an empty answer.
	pub async fn send_json(&self, request: AwsRequest) -> Result<Value> {
		Self::json_of(self.send(request).await?).await
	}

	/// Send a `restXml` request, answering the response's root element.
	pub async fn send_xml(&self, request: AwsRequest) -> Result<BsxElement> {
		BsxElement::parse_xml(&self.send(request).await?.text().await?)
	}

	async fn json_of(response: Response) -> Result<Value> {
		let text = response.text().await?;
		match text.trim().is_empty() {
			true => Value::Null.xok(),
			false => Value::parse_json(&text)?.xok(),
		}
	}

	/// Send `request`, answering the successful response unread; any other
	/// answer is an [`AwsError`], retried as the client's docs describe.
	pub async fn send(&self, request: AwsRequest) -> Result<Response> {
		// the waits up front, so no rng is held across an await and the
		// future stays `Send`
		let waits = Backoff::default()
			.with_max_attempts(3)
			.iter()
			.map(|frame| frame.next_attempt)
			.collect::<Vec<_>>();
		let mut waits = waits.into_iter();
		let mut client = self.clone();
		let (mut recovered, mut redirected) = (false, false);
		loop {
			let auth = client.auth_for(&request);
			let sent = match auth.authorize(client.build(&request)?).await {
				Ok(signed) => client.dispatch(signed).await,
				Err(err) => return Err(err),
			};
			let response = match sent {
				Ok(response) => response,
				Err(err) => match waits.next().flatten() {
					Some(wait) => {
						warn!(
							"{} {} did not reach AWS, retrying in {wait:?}: {err}",
							self.service.endpoint_prefix, request.action
						);
						Self::sleep(wait).await;
						continue;
					}
					None => return Err(err),
				},
			};
			let status = response.status();
			if status.is_success() {
				return Ok(response);
			}
			let headers = response.headers().clone();
			let body = response.text().await.unwrap_or_default();
			let error = AwsError::read(
				&self.service,
				&request.action,
				status,
				&headers,
				&body,
			)
			.redact(request.secret.as_deref());
			if let Some(region) = client.redirect_region(&error, &headers)
				&& !redirected
			{
				redirected = true;
				client = client.with_region(region);
				continue;
			}
			if error.is_retryable()
				&& let Some(wait) = waits.next().flatten()
			{
				warn!(
					"{} {} answered {}, retrying in {wait:?}",
					self.service.endpoint_prefix,
					request.action,
					error.error_code()
				);
				Self::sleep(wait).await;
				continue;
			}
			if !recovered
				&& auth
					.recover(&AuthFailure {
						status,
						code: error.error_code().into(),
						message: error.message().to_string(),
						headers,
					})
					.await?
			{
				recovered = true;
				continue;
			}
			return Err(error.into());
		}
	}

	/// The auth one request goes through: the client's signer for its
	/// region, or its custom auth.
	fn auth_for(&self, request: &AwsRequest) -> Arc<dyn RequestAuth> {
		match &self.auth {
			ClientAuth::Signed(source) => {
				let region = self.service.signing_region(&self.region);
				let signer = match self.service.protocol {
					AwsProtocol::RestXml => {
						let signer = SigV4::s3(region);
						match request.unsigned_payload {
							true => signer.with_unsigned_payload(),
							false => signer,
						}
					}
					_ => SigV4::new(region, self.service.signing_name),
				};
				Arc::new(AwsAuth::new(source.clone(), signer))
			}
			ClientAuth::Custom(auth) => auth.clone(),
		}
	}

	/// The region an S3 refusal names as the bucket's own, when it is a
	/// wrong-region refusal and the region differs from this client's.
	fn redirect_region(
		&self,
		error: &AwsError,
		headers: &HeaderMap,
	) -> Option<SmolStr> {
		let is_s3 = self.service.protocol == AwsProtocol::RestXml;
		let is_redirect = matches!(error.status(), 301 | 307)
			|| [
				"PermanentRedirect",
				"TemporaryRedirect",
				"AuthorizationHeaderMalformed",
				"IllegalLocationConstraintException",
			]
			.contains(&error.error_code());
		let region = headers.first_raw("x-amz-bucket-region")?;
		(is_s3
			&& is_redirect
			&& self.endpoint.is_none()
			&& region != self.region)
			.then(|| region.into())
	}

	/// The url `request` goes to.
	fn url(&self, request: &AwsRequest) -> String {
		let host = self.service.host(&self.region);
		let base = match (&self.endpoint, &request.bucket) {
			(Some(endpoint), Some(bucket)) => {
				format!("{endpoint}/{}", uri_encode(bucket, ""))
			}
			(Some(endpoint), None) => endpoint.to_string(),
			(None, Some(bucket)) if bucket.contains('.') => {
				format!("https://{host}/{bucket}")
			}
			(None, Some(bucket)) => format!("https://{bucket}.{host}"),
			(None, None) => format!("https://{host}"),
		};
		let path = match request.path.starts_with('/') {
			true => uri_encode(&request.path, "/"),
			false => format!("/{}", uri_encode(&request.path, "/")),
		};
		let query = request
			.query
			.iter()
			.map(|(key, value)| match value.is_empty() {
				true => uri_encode(key, ""),
				false => {
					format!("{}={}", uri_encode(key, ""), uri_encode(value, ""))
				}
			})
			.collect::<Vec<_>>()
			.join("&");
		match query.is_empty() {
			true => format!("{base}{path}"),
			false => format!("{base}{path}?{query}"),
		}
	}

	/// The unsigned request `request` describes.
	fn build(&self, request: &AwsRequest) -> Result<Request> {
		// a redirect is AWS naming another region or host, which the client
		// reads itself: S3's `301` carries `x-amz-bucket-region`, no `Location`
		let mut built = Request::new(request.method, self.url(request))
			.with_redirects(Redirects::None)
			.with_body(request.body.clone());
		for (name, value) in &request.headers {
			built = built.with_header_raw(name, value);
		}
		built.xok()
	}

	async fn dispatch(&self, request: Request) -> Result<Response> {
		match &self.transport {
			Some(transport) => transport(request).await,
			None => request.send().await,
		}
	}

	/// Sleep for `wait` on every target, the future `Send` there too: on
	/// wasm the js timer is polled only on the thread that made it.
	pub(crate) async fn sleep(wait: Duration) {
		cfg_if! {
			if #[cfg(target_arch = "wasm32")] {
				// a js timer, polled only on the one thread that made it, so
				// the call stays `Send` for a provider
				send_wrapper::SendWrapper::new(time_ext::sleep(wait)).await;
			} else {
				time_ext::sleep(wait).await;
			}
		}
	}
}

/// [`AwsClient::call_json`] and [`AwsClient::send`] are `Send` on every
/// target, the guarantee a provider whose futures must be `Send` (a secret
/// store, a blob store) relies on.
#[allow(dead_code)]
fn call_is_send() {
	fn assert_send(_: impl Send) {}
	let client = AwsClient::new(AwsService::SSM, "us-west-2");
	assert_send(client.call_json("GetParameter", &Value::Null));
	assert_send(client.send(AwsRequest::get("GetObject", "/")));
}

/// One `restJson1` or `restXml` request: the method, the path below the
/// service (or the bucket) and its query, headers and body, named by the
/// operation it calls for errors and logs.
///
/// The path is written decoded (`/2015-03-31/functions/my-fn/code`, an
/// object key as it is), and encoded once when the url is built.
#[derive(Debug, Clone)]
pub struct AwsRequest {
	action: SmolStr,
	method: HttpMethod,
	bucket: Option<SmolStr>,
	path: String,
	query: Vec<(SmolStr, String)>,
	headers: Vec<(SmolStr, String)>,
	body: Bytes,
	secret: Option<String>,
	unsigned_payload: bool,
}

impl AwsRequest {
	/// `action` as `method` to `path`.
	pub fn new(
		action: impl Into<SmolStr>,
		method: HttpMethod,
		path: impl Into<String>,
	) -> Self {
		Self {
			action: action.into(),
			method,
			bucket: None,
			path: path.into(),
			query: Vec::new(),
			headers: Vec::new(),
			body: Bytes::new(),
			secret: None,
			unsigned_payload: false,
		}
	}

	/// A `GET`, see [`new`](Self::new).
	pub fn get(action: impl Into<SmolStr>, path: impl Into<String>) -> Self {
		Self::new(action, HttpMethod::Get, path)
	}
	/// A `HEAD`, see [`new`](Self::new).
	pub fn head(action: impl Into<SmolStr>, path: impl Into<String>) -> Self {
		Self::new(action, HttpMethod::Head, path)
	}
	/// A `PUT`, see [`new`](Self::new).
	pub fn put(action: impl Into<SmolStr>, path: impl Into<String>) -> Self {
		Self::new(action, HttpMethod::Put, path)
	}
	/// A `POST`, see [`new`](Self::new).
	pub fn post(action: impl Into<SmolStr>, path: impl Into<String>) -> Self {
		Self::new(action, HttpMethod::Post, path)
	}
	/// A `DELETE`, see [`new`](Self::new).
	pub fn delete(action: impl Into<SmolStr>, path: impl Into<String>) -> Self {
		Self::new(action, HttpMethod::Delete, path)
	}

	/// Address S3's `bucket`, its virtual host or path style as the client
	/// decides.
	pub fn with_bucket(mut self, bucket: impl Into<SmolStr>) -> Self {
		self.bucket = Some(bucket.into());
		self
	}

	/// Add the query parameter `key`, its value written decoded.
	pub fn with_query(
		mut self,
		key: impl Into<SmolStr>,
		value: impl Into<String>,
	) -> Self {
		self.query.push((key.into(), value.into()));
		self
	}

	/// Add a query flag with no value, ie S3's `?versioning`.
	pub fn with_flag(self, key: impl Into<SmolStr>) -> Self {
		self.with_query(key, "")
	}

	/// Add the header `name`.
	pub fn with_header(
		mut self,
		name: impl Into<SmolStr>,
		value: impl Into<String>,
	) -> Self {
		self.headers.push((name.into(), value.into()));
		self
	}

	/// Send `body`.
	pub fn with_body(mut self, body: impl Into<Bytes>) -> Self {
		self.body = body.into();
		self
	}

	/// Send `json` as an `application/json` body.
	pub fn with_json(self, json: &Value) -> Result<Self> {
		self.with_header("content-type", "application/json")
			.with_body(json.to_json_string())
			.xok()
	}

	/// Send `xml` as an `application/xml` body.
	pub fn with_xml(self, xml: impl Into<String>) -> Self {
		self.with_header("content-type", "application/xml")
			.with_body(xml.into())
	}

	/// Redact `secret`, a value this request carries, from any error it
	/// answers.
	pub fn with_secret(mut self, secret: impl Into<String>) -> Self {
		self.secret = Some(secret.into());
		self
	}

	/// The body this request sends.
	pub fn body(&self) -> &Bytes { &self.body }

	/// Sign the body as `UNSIGNED-PAYLOAD` rather than its hash (S3 only).
	pub fn with_unsigned_payload(mut self) -> Self {
		self.unsigned_payload = true;
		self
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use std::sync::Arc;
	use std::sync::Mutex;

	type Handler =
		Arc<dyn Fn(Request) -> SendBoxedFuture<Result<Response>> + Send + Sync>;

	/// A client whose AWS is `handler`, signing as a fixed credential.
	fn client(service: AwsService, handler: Handler) -> AwsClient {
		AwsClient::new(service, "ap-southeast-2")
			.with_credentials(AwsCredentials::new("AKID", "secret"))
			.with_transport(move |request| handler(request))
	}

	/// A handler answering each request with the next of `answers`, keeping
	/// what was asked.
	fn scripted(
		answers: Vec<Response>,
	) -> (Handler, Arc<Mutex<Vec<(String, String)>>>) {
		let answers = Arc::new(Mutex::new(answers.into_iter()));
		let asked = Arc::new(Mutex::new(Vec::new()));
		let seen = asked.clone();
		let handler: Handler = Arc::new(move |request: Request| {
			let answers = answers.clone();
			let asked = seen.clone();
			Box::pin(async move {
				let url = request.url().to_string();
				let body = String::from_utf8(
					request.body.into_bytes().await?.to_vec(),
				)?;
				asked.lock().unwrap().push((url, body));
				answers
					.lock()
					.unwrap()
					.next()
					.ok_or_else(|| bevyhow!("no answer left"))
			})
		});
		(handler, asked)
	}

	/// A json call is a signed POST to the regional endpoint targeting the
	/// action, answering the response json.
	#[beet_core::test]
	async fn calls_a_json_target() {
		let handler: Handler = Arc::new(|request: Request| {
			Box::pin(async move {
				request
					.url()
					.to_string()
					.xpect_eq("https://ssm.ap-southeast-2.amazonaws.com/");
				request
					.headers
					.first_raw("x-amz-target")
					.unwrap()
					.xpect_eq("AmazonSSM.DescribeParameters");
				request
					.headers
					.first_raw("authorization")
					.unwrap()
					.xpect_contains("/ap-southeast-2/ssm/aws4_request");
				let body = request.body.into_bytes().await?;
				Value::parse_json(core::str::from_utf8(&body)?)?
					.xpect_eq(value!({"MaxResults": 50}));
				Response::ok().with_body(r#"{"Parameters":[]}"#).xok()
			})
		});
		client(AwsService::SSM, handler)
			.call_json("DescribeParameters", &value!({"MaxResults": 50}))
			.await
			.unwrap()
			.xpect_eq(value!({"Parameters": []}));
	}

	/// A query call is a form body naming the action and version, answering
	/// its result element.
	#[beet_core::test]
	async fn calls_a_query_action() {
		let (handler, asked) = scripted(vec![Response::ok().with_body(
			"<GetCallerIdentityResponse><GetCallerIdentityResult>\
			<Account>123456789012</Account></GetCallerIdentityResult>\
			</GetCallerIdentityResponse>",
		)]);
		client(AwsService::STS, handler)
			.call_query("GetCallerIdentity", [("Extra", "a b")])
			.await
			.unwrap()
			.text_at("Account")
			.xpect_eq(Some("123456789012"));
		asked.lock().unwrap()[0].clone().xpect_eq((
			"https://sts.ap-southeast-2.amazonaws.com/".into(),
			"Action=GetCallerIdentity&Version=2011-06-15&Extra=a%20b".into(),
		));
	}

	/// A global service goes to its one host, signed for its own region.
	#[beet_core::test]
	async fn signs_a_global_service_for_its_region() {
		let handler: Handler = Arc::new(|request: Request| {
			Box::pin(async move {
				request
					.url()
					.to_string()
					.xpect_eq("https://iam.amazonaws.com/");
				request
					.headers
					.first_raw("authorization")
					.unwrap()
					.xpect_contains("/us-east-1/iam/aws4_request");
				Response::ok().with_body("<R/>").xok()
			})
		});
		client(AwsService::IAM, handler)
			.call_query("ListUsers", Vec::<(&str, &str)>::new())
			.await
			.unwrap();
	}

	/// S3 addresses a bucket by virtual host, path style with a dot in its
	/// name or behind an endpoint, the key encoded and a flag bare.
	#[beet_core::test]
	fn addresses_s3_buckets() {
		let s3 = AwsClient::new(AwsService::S3, "us-west-2");
		let request = AwsRequest::get("GetObject", "/dir/a b+c.txt")
			.with_bucket("bucket")
			.with_flag("versioning");
		s3.url(&request).xpect_eq(
			"https://bucket.s3.us-west-2.amazonaws.com/dir/a%20b%2Bc.txt?versioning",
		);
		s3.url(&request.clone().with_bucket("my.bucket")).xpect_eq(
			"https://s3.us-west-2.amazonaws.com/my.bucket/dir/a%20b%2Bc.txt?versioning",
		);
		s3.clone()
			.with_endpoint("https://acct.r2.cloudflarestorage.com/")
			.url(&request)
			.xpect_eq(
				"https://acct.r2.cloudflarestorage.com/bucket/dir/a%20b%2Bc.txt?versioning",
			);
	}

	/// A throttle and a server fault are retried, the third answer counts.
	#[beet_core::test]
	async fn retries_throttles_and_faults() {
		let (handler, asked) = scripted(vec![
			Response::from_status(StatusCode::BAD_REQUEST).with_body(
				r#"{"__type":"ThrottlingException","message":"slow"}"#,
			),
			Response::from_status(StatusCode::SERVICE_UNAVAILABLE),
			Response::ok().with_body("{}"),
		]);
		client(AwsService::SSM, handler)
			.call_json("GetParameter", &value!({}))
			.await
			.unwrap();
		asked.lock().unwrap().len().xpect_eq(3);
	}

	/// An error that no retry fixes is answered at once, by its code, with
	/// the secret a write carries redacted.
	#[beet_core::test]
	async fn answers_errors_by_code() {
		let (handler, asked) = scripted(vec![
			Response::from_status(StatusCode::BAD_REQUEST).with_body(
				r#"{"__type":"com.amazonaws.ssm#ParameterAlreadyExists","message":"value hunter2 exists"}"#,
			),
		]);
		let err = client(AwsService::SSM, handler)
			.call_json_secret(
				"PutParameter",
				&value!({"Value": "hunter2"}),
				"hunter2",
			)
			.await
			.unwrap_err();
		AwsError::code(&err).xpect_eq(Some("ParameterAlreadyExists"));
		err.to_string()
			.xpect_contains("PutParameter")
			.xnot()
			.xpect_contains("hunter2");
		asked.lock().unwrap().len().xpect_eq(1);
	}

	/// A credential that lapses mid-run is refreshed and the request retried
	/// once.
	#[beet_core::test]
	async fn retries_after_recovering() {
		struct Rotating(Arc<core::sync::atomic::AtomicU32>);
		impl AwsCredentialProvider for Rotating {
			fn describe(&self) -> SmolStr { "rotating".into() }
			fn load(&self) -> SendBoxedFuture<Result<Option<AwsCredentials>>> {
				let count =
					self.0.fetch_add(1, core::sync::atomic::Ordering::SeqCst);
				Box::pin(async move {
					Ok(Some(AwsCredentials::new(
						format!("KEY{count}"),
						"secret",
					)))
				})
			}
		}
		let keys = Arc::new(Mutex::new(Vec::new()));
		let seen = keys.clone();
		let answered = Arc::new(core::sync::atomic::AtomicU32::new(0));
		let handler: Handler = Arc::new(move |request: Request| {
			let auth = request
				.headers
				.first_raw("authorization")
				.unwrap()
				.to_string();
			seen.lock().unwrap().push(auth);
			let first = answered
				.fetch_add(1, core::sync::atomic::Ordering::SeqCst)
				== 0;
			Box::pin(async move {
				match first {
					true => Response::from_status(StatusCode::FORBIDDEN)
						.with_body(
							r#"{"__type":"ExpiredTokenException","message":"expired"}"#,
						)
						.xok(),
					false => Response::ok().with_body("{}").xok(),
				}
			})
		});
		AwsClient::new(AwsService::SSM, "us-east-1")
			.with_credentials(AwsCredentialSource::new(Rotating(default())))
			.with_transport(move |request| handler(request))
			.call_json("GetParameter", &value!({}))
			.await
			.unwrap();
		let keys = keys.lock().unwrap().clone();
		keys.len().xpect_eq(2);
		keys[0].as_str().xpect_contains("Credential=KEY0/");
		keys[1].as_str().xpect_contains("Credential=KEY1/");
	}

	/// An S3 refusal naming the bucket's region is retried there once.
	#[beet_core::test]
	async fn follows_an_s3_region() {
		let (handler, asked) = scripted(vec![
			Response::from_status(StatusCode::MOVED_PERMANENTLY)
				.with_header("x-amz-bucket-region", "eu-west-1"),
			Response::ok(),
		]);
		client(AwsService::S3, handler)
			.send(AwsRequest::head("HeadObject", "/key").with_bucket("bucket"))
			.await
			.unwrap();
		asked
			.lock()
			.unwrap()
			.iter()
			.map(|(url, _)| url.clone())
			.collect::<Vec<_>>()
			.xpect_eq(vec![
				"https://bucket.s3.ap-southeast-2.amazonaws.com/key"
					.to_string(),
				"https://bucket.s3.eu-west-1.amazonaws.com/key".to_string(),
			]);
	}

	/// An anonymous client signs nothing.
	#[beet_core::test]
	async fn sends_anonymously() {
		let handler: Handler = Arc::new(|request: Request| {
			Box::pin(async move {
				request.headers.first_raw("authorization").xpect_none();
				Response::ok().xok()
			})
		});
		client(AwsService::S3, handler)
			.anonymous()
			.send(AwsRequest::get("GetObject", "/key").with_bucket("public"))
			.await
			.unwrap();
	}
}
