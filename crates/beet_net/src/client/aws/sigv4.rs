//! AWS Signature Version 4 over beet's own [`Request`].
//!
//! The signing is a pure function of the request's wire form (method, path,
//! query, headers and payload hash), the credential, the region, the signing
//! name and the clock, so AWS's published test suite drives it directly and a
//! signature always covers exactly what the transport sends.

use crate::prelude::*;
use beet_core::prelude::*;
use hmac::KeyInit;
use hmac::Mac;
use sha2::Sha256;

/// How one service signs a request: the region and signing name its
/// credential scope names, and the service's flavour of the canonical form.
///
/// Every service but S3 encodes the path twice and normalizes its dot
/// segments; S3 encodes it once, leaves it as written and sends the payload
/// hash as `x-amz-content-sha256` (or `UNSIGNED-PAYLOAD` where the body is
/// not hashed, see [`with_unsigned_payload`](Self::with_unsigned_payload)).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SigV4 {
	region: SmolStr,
	service: SmolStr,
	rules: CanonicalRules,
}

impl SigV4 {
	/// The algorithm every signature here names.
	pub const ALGORITHM: &'static str = "AWS4-HMAC-SHA256";
	/// The payload hash of a body S3 is told not to verify.
	pub const UNSIGNED_PAYLOAD: &'static str = "UNSIGNED-PAYLOAD";

	/// `service` (its signing name, ie `dynamodb`) in `region`.
	pub fn new(
		region: impl Into<SmolStr>,
		service: impl Into<SmolStr>,
	) -> Self {
		Self {
			region: region.into(),
			service: service.into(),
			rules: CanonicalRules::STANDARD,
		}
	}

	/// S3 in `region`, its single-encoded path and payload hash header.
	pub fn s3(region: impl Into<SmolStr>) -> Self {
		Self {
			region: region.into(),
			service: "s3".into(),
			rules: CanonicalRules::S3,
		}
	}

	/// Sign the payload as `UNSIGNED-PAYLOAD` rather than its hash, which S3
	/// accepts over tls: a large upload is then never hashed twice.
	pub fn with_unsigned_payload(mut self) -> Self {
		self.rules.unsigned_payload = true;
		self
	}

	/// The region the credential scope names.
	pub fn region(&self) -> &str { &self.region }

	/// The signing name the credential scope names.
	pub fn service(&self) -> &str { &self.service }

	/// The same signer for `region`, ie after S3 named a bucket's own.
	pub fn with_region(mut self, region: impl Into<SmolStr>) -> Self {
		self.region = region.into();
		self
	}

	/// `request` signed as `credentials` at `at`: its `x-amz-date`, the
	/// session token where the credential carries one, the payload hash
	/// header where the service wants it, and the `authorization` over every
	/// header it already carries, its body and the host its url names.
	pub async fn sign(
		&self,
		request: Request,
		credentials: &AwsCredentials,
		at: Timestamp,
	) -> Result<Request> {
		// the redirect policy rides the request, not its parts
		let redirects = request.redirects;
		let (mut parts, body) = request.into_parts();
		let body = body.into_bytes().await?;
		let uri: http::Uri = parts.uri().parse()?;
		let mut headers = parts
			.headers
			.iter_all()
			.flat_map(|(name, values)| {
				values
					.iter()
					.map(move |value| (name.to_string(), value.clone()))
			})
			.collect::<Vec<_>>();
		if !headers
			.iter()
			.any(|(name, _)| name.eq_ignore_ascii_case("host"))
		{
			headers.push(("host".into(), Self::host_of(&uri)?));
		}
		let payload_hash = match self.rules.unsigned_payload {
			true => Self::UNSIGNED_PAYLOAD.to_string(),
			false => digest_ext::hex::<Sha256>(&body),
		};
		let signed = self.sign_wire(
			&WireRequest {
				method: parts.method().to_string().to_uppercase(),
				path: uri.path().to_string(),
				query: uri.query().unwrap_or_default().to_string(),
				headers,
				payload_hash,
			},
			credentials,
			at,
		);
		for (name, value) in signed.headers() {
			parts.headers.remove_raw(name);
			parts.headers.set_raw(name, value);
		}
		Request::from_parts(parts, body.into())
			.with_redirects(redirects)
			.xok()
	}

	/// The `host` header a url implies: its authority, the port dropped where
	/// it is the scheme's default.
	fn host_of(uri: &http::Uri) -> Result<String> {
		let Some(host) = uri.host() else {
			bevybail!("a signed request needs an absolute url, not `{uri}`");
		};
		let default_port = match uri.scheme_str() {
			Some("http") => Some(80),
			Some("https") => Some(443),
			_ => None,
		};
		match uri.port_u16() {
			Some(port) if Some(port) != default_port => {
				format!("{host}:{port}")
			}
			_ => host.to_string(),
		}
		.xok()
	}

	/// The headers signing `request` adds, over its wire form.
	fn sign_wire(
		&self,
		request: &WireRequest,
		credentials: &AwsCredentials,
		at: Timestamp,
	) -> SignedHeaders {
		let amz_date = Self::amz_date(at);
		let date = &amz_date[..8];
		let mut added = SignedHeaders {
			amz_date: amz_date.clone(),
			security_token: credentials
				.session_token()
				.map(ToString::to_string),
			content_sha256: self
				.rules
				.content_sha256
				.then(|| request.payload_hash.clone()),
			authorization: String::new(),
		};
		let (canonical, signed_names) =
			self.canonical_request(request, &added.headers());
		let scope =
			format!("{date}/{}/{}/aws4_request", self.region, self.service);
		let string_to_sign = format!(
			"{}\n{amz_date}\n{scope}\n{}",
			Self::ALGORITHM,
			digest_ext::hex::<Sha256>(canonical.as_bytes())
		);
		let key = [
			date,
			self.region.as_str(),
			self.service.as_str(),
			"aws4_request",
		]
		.into_iter()
		.fold(
			format!("AWS4{}", credentials.secret_access_key()).into_bytes(),
			|key, part| Self::hmac(&key, part.as_bytes()),
		);
		added.authorization = format!(
			"{} Credential={}/{scope}, SignedHeaders={signed_names}, Signature={}",
			Self::ALGORITHM,
			credentials.access_key_id(),
			digest_ext::to_hex(&Self::hmac(&key, string_to_sign.as_bytes()))
		);
		added
	}

	/// The canonical request and its signed header list: the request's own
	/// headers plus the `added` ones, lowercased, trimmed and sorted.
	fn canonical_request(
		&self,
		request: &WireRequest,
		added: &[(&'static str, String)],
	) -> (String, String) {
		// one entry per name, repeated values joined in the order sent
		let mut headers = Vec::<(String, String)>::new();
		// an added header replaces one the request already carried
		let is_added =
			|name: &str| added.iter().any(|(added, _)| *added == name);
		for (name, value) in request
			.headers
			.iter()
			.map(|(name, value)| (name.to_lowercase(), Self::trim_all(value)))
			.filter(|(name, _)| !is_added(name))
			.chain(
				added
					.iter()
					.filter(|(name, _)| *name != "authorization")
					.map(|(name, value)| (name.to_string(), value.clone())),
			)
			.filter(|(name, _)| {
				!CanonicalRules::UNSIGNED_HEADERS.contains(&name.as_str())
			}) {
			match headers.iter_mut().find(|(existing, _)| *existing == name) {
				Some((_, existing)) => {
					existing.push(',');
					existing.push_str(&value);
				}
				None => headers.push((name, value)),
			}
		}
		headers.sort_by(|(left, _), (right, _)| left.cmp(right));
		let signed_names = headers
			.iter()
			.map(|(name, _)| name.as_str())
			.collect::<Vec<_>>()
			.join(";");
		let canonical_headers = headers
			.iter()
			.map(|(name, value)| format!("{name}:{value}\n"))
			.collect::<String>();
		let canonical = format!(
			"{}\n{}\n{}\n{canonical_headers}\n{signed_names}\n{}",
			request.method,
			self.canonical_path(&request.path),
			Self::canonical_query(&request.query),
			request.payload_hash
		);
		(canonical, signed_names)
	}

	/// The path as the service canonicalizes it: normalized and the wire form
	/// encoded again for most, the decoded path encoded once for S3.
	fn canonical_path(&self, path: &str) -> String {
		let path = match path.is_empty() {
			true => "/",
			false => path,
		};
		let path = match self.rules.normalize_path {
			true => Self::normalize_path(path),
			false => path.to_string(),
		};
		match self.rules.double_encode_path {
			true => uri_encode(&path, "/"),
			false => path
				.split('/')
				.map(|segment| uri_encode(&percent_decode(segment), ""))
				.collect::<Vec<_>>()
				.join("/"),
		}
	}

	/// RFC 3986's dot segment removal, also collapsing empty segments, a
	/// trailing slash kept where the path ends in one.
	fn normalize_path(path: &str) -> String {
		if !(path.contains('.') || path.contains("//")) {
			return path.to_string();
		}
		let mut segments = Vec::new();
		for segment in path.split('/') {
			match segment {
				"" | "." => {}
				".." => {
					segments.pop();
				}
				segment => segments.push(segment),
			}
		}
		let mut normalized = format!("/{}", segments.join("/"));
		let ends_in_slash = ["/", "/.", "/./", "/..", "/../"]
			.iter()
			.any(|end| path.ends_with(end));
		if ends_in_slash && !normalized.ends_with('/') {
			normalized.push('/');
		}
		normalized
	}

	/// The query's pairs decoded, encoded strictly and sorted, a key without
	/// a value keeping its `=`.
	fn canonical_query(query: &str) -> String {
		let mut pairs = query
			.split('&')
			.filter(|pair| !pair.is_empty())
			.map(|pair| {
				let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
				(
					uri_encode(&percent_decode(key), ""),
					uri_encode(&percent_decode(value), ""),
				)
			})
			.collect::<Vec<_>>();
		pairs.sort();
		pairs
			.into_iter()
			.map(|(key, value)| format!("{key}={value}"))
			.collect::<Vec<_>>()
			.join("&")
	}

	/// A header value with its ends trimmed and each run of spaces one space.
	fn trim_all(value: &str) -> String {
		let mut trimmed = String::with_capacity(value.len());
		for word in value.split(' ').filter(|word| !word.is_empty()) {
			if !trimmed.is_empty() {
				trimmed.push(' ');
			}
			trimmed.push_str(word);
		}
		trimmed
	}

	/// `at` in the basic format the date header and the scope use,
	/// `20150830T123600Z`.
	fn amz_date(at: Timestamp) -> String {
		at.format_iso8601_secs().replace(['-', ':'], "")
	}

	fn hmac(key: &[u8], data: &[u8]) -> Vec<u8> {
		let mut mac = hmac::Hmac::<Sha256>::new_from_slice(key)
			.expect("hmac takes a key of any length");
		mac.update(data);
		mac.finalize().into_bytes().to_vec()
	}
}

/// What differs between the canonical forms services verify.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CanonicalRules {
	/// Encode the wire path again, rather than encoding the decoded path once.
	double_encode_path: bool,
	/// Remove dot segments and empty segments from the path.
	normalize_path: bool,
	/// Send and sign the payload hash as `x-amz-content-sha256`.
	content_sha256: bool,
	/// Sign `UNSIGNED-PAYLOAD` rather than the body's hash.
	unsigned_payload: bool,
}

impl CanonicalRules {
	/// Every service but S3.
	const STANDARD: Self = Self {
		double_encode_path: true,
		normalize_path: true,
		content_sha256: false,
		unsigned_payload: false,
	};
	/// S3.
	const S3: Self = Self {
		double_encode_path: false,
		normalize_path: false,
		content_sha256: true,
		unsigned_payload: false,
	};
	/// Headers a proxy or the transport may change, which are never signed.
	const UNSIGNED_HEADERS: &[&str] = &[
		"authorization",
		"user-agent",
		"x-amzn-trace-id",
		"transfer-encoding",
	];
}

/// A request as the transport sends it, the input of the signing.
struct WireRequest {
	method: String,
	/// The path as sent, percent-encoded.
	path: String,
	/// The query as sent, without its `?`.
	query: String,
	headers: Vec<(String, String)>,
	payload_hash: String,
}

/// The headers a signature adds to its request.
struct SignedHeaders {
	amz_date: String,
	security_token: Option<String>,
	content_sha256: Option<String>,
	authorization: String,
}

impl SignedHeaders {
	fn headers(&self) -> Vec<(&'static str, String)> {
		let mut headers = vec![("x-amz-date", self.amz_date.clone())];
		if let Some(token) = &self.security_token {
			headers.push(("x-amz-security-token", token.clone()));
		}
		if let Some(hash) = &self.content_sha256 {
			headers.push(("x-amz-content-sha256", hash.clone()));
		}
		if !self.authorization.is_empty() {
			headers.push(("authorization", self.authorization.clone()));
		}
		headers
	}
}

/// `text` with every byte outside the unreserved set (`A-Za-z0-9-._~`) and
/// `keep` written as `%XX`, uppercase: AWS's `UriEncode`.
pub(crate) fn uri_encode(text: &str, keep: &str) -> String {
	let mut out = String::with_capacity(text.len());
	for byte in text.bytes() {
		match byte {
			b'A'..=b'Z'
			| b'a'..=b'z'
			| b'0'..=b'9'
			| b'-'
			| b'.'
			| b'_'
			| b'~' => out.push(byte as char),
			byte if keep.as_bytes().contains(&byte) => out.push(byte as char),
			byte => out.push_str(&format!("%{byte:02X}")),
		}
	}
	out
}

/// `text` with its `%XX` escapes decoded, a malformed one kept as written.
pub(crate) fn percent_decode(text: &str) -> String {
	if !text.contains('%') {
		return text.to_string();
	}
	let bytes = text.as_bytes();
	let mut out = Vec::with_capacity(bytes.len());
	let mut index = 0;
	while index < bytes.len() {
		let hex = |at: usize| {
			bytes.get(at).and_then(|byte| (*byte as char).to_digit(16))
		};
		match (bytes[index], hex(index + 1), hex(index + 2)) {
			(b'%', Some(high), Some(low)) => {
				out.push((high * 16 + low) as u8);
				index += 3;
			}
			(byte, _, _) => {
				out.push(byte);
				index += 1;
			}
		}
	}
	String::from_utf8(out).unwrap_or_else(|_| text.to_string())
}

#[cfg(test)]
mod test {
	use super::*;

	/// The credential of AWS's published test suite.
	fn suite_credentials() -> AwsCredentials {
		AwsCredentials::new(
			"AKIDEXAMPLE",
			"wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY",
		)
	}

	/// The suite's clock, `20150830T123600Z`.
	fn suite_time() -> Timestamp {
		Timestamp::parse_iso8601("2015-08-30T12:36:00Z").unwrap()
	}

	/// One case of the vendored suite: its request, its context and the
	/// canonical request, string to sign and signature it publishes.
	struct SuiteCase {
		name: &'static str,
		request: &'static str,
		context: &'static str,
		canonical: &'static str,
		string_to_sign: Option<&'static str>,
		/// The signed request, `None` where the suite's own is inconsistent
		/// with its canonical request (`double-encode-path` signs another
		/// day and key than it canonicalizes).
		signed: Option<&'static str>,
	}

	macro_rules! suite_case {
		($name:literal) => {
			SuiteCase {
				name: $name,
				request: include_str!(concat!(
					"sigv4_suite/",
					$name,
					"/request.txt"
				)),
				context: include_str!(concat!(
					"sigv4_suite/",
					$name,
					"/context.json"
				)),
				canonical: include_str!(concat!(
					"sigv4_suite/",
					$name,
					"/header-canonical-request.txt"
				)),
				string_to_sign: Some(include_str!(concat!(
					"sigv4_suite/",
					$name,
					"/header-string-to-sign.txt"
				))),
				signed: Some(include_str!(concat!(
					"sigv4_suite/",
					$name,
					"/header-signed-request.txt"
				))),
			}
		};
	}

	/// Every case of the suite that signs in headers. Those that publish no
	/// context (`double-encode-path`, `double-url-encode`) take the scope
	/// their signed request names and the double encoding they are named
	/// for; every other case is the single-encoded form the suite was written
	/// against.
	fn suite() -> Vec<SuiteCase> {
		vec![
			suite_case!("get-header-key-duplicate"),
			suite_case!("get-header-value-multiline"),
			suite_case!("get-header-value-order"),
			suite_case!("get-header-value-trim"),
			suite_case!("get-relative-normalized"),
			suite_case!("get-relative-relative-normalized"),
			suite_case!("get-relative-relative-unnormalized"),
			suite_case!("get-relative-unnormalized"),
			suite_case!("get-slash-dot-slash-normalized"),
			suite_case!("get-slash-dot-slash-unnormalized"),
			suite_case!("get-slash-normalized"),
			suite_case!("get-slash-pointless-dot-normalized"),
			suite_case!("get-slash-pointless-dot-unnormalized"),
			suite_case!("get-slash-unnormalized"),
			suite_case!("get-slashes-normalized"),
			suite_case!("get-slashes-unnormalized"),
			suite_case!("get-space-normalized"),
			suite_case!("get-space-unnormalized"),
			suite_case!("get-unreserved"),
			suite_case!("get-utf8"),
			suite_case!("get-vanilla"),
			suite_case!("get-vanilla-empty-query-key"),
			suite_case!("get-vanilla-query"),
			suite_case!("get-vanilla-query-order-encoded"),
			suite_case!("get-vanilla-query-order-key-case"),
			suite_case!("get-vanilla-query-unreserved"),
			suite_case!("get-vanilla-utf8-query"),
			suite_case!("get-vanilla-with-session-token"),
			suite_case!("post-header-key-case"),
			suite_case!("post-header-key-sort"),
			suite_case!("post-header-value-case"),
			// `post-sts-header-after` is left out: it adds the session token
			// unsigned, which no service beet calls asks for
			suite_case!("post-sts-header-before"),
			suite_case!("post-vanilla"),
			suite_case!("post-vanilla-empty-query-value"),
			suite_case!("post-vanilla-query"),
			suite_case!("post-x-www-form-urlencoded"),
			suite_case!("post-x-www-form-urlencoded-parameters"),
			SuiteCase {
				name: "double-encode-path",
				request: include_str!(
					"sigv4_suite/double-encode-path/request.txt"
				),
				context: "",
				canonical: include_str!(
					"sigv4_suite/double-encode-path/header-canonical-request.txt"
				),
				string_to_sign: None,
				signed: None,
			},
			SuiteCase {
				name: "double-url-encode",
				request: include_str!(
					"sigv4_suite/double-url-encode/request.txt"
				),
				context: "",
				canonical: include_str!(
					"sigv4_suite/double-url-encode/header-canonical-request.txt"
				),
				string_to_sign: Some(include_str!(
					"sigv4_suite/double-url-encode/header-string-to-sign.txt"
				)),
				signed: Some(include_str!(
					"sigv4_suite/double-url-encode/header-signed-request.txt"
				)),
			},
		]
	}

	impl SuiteCase {
		/// The case's context, `None` for the two that publish none.
		fn context(&self) -> Option<serde_json::Value> {
			match self.context.is_empty() {
				true => None,
				false => {
					serde_json::from_str::<serde_json::Value>(self.context)
						.unwrap()
						.xsome()
				}
			}
		}

		/// The signer and credential the case's context names.
		fn signer(&self) -> (SigV4, AwsCredentials, Timestamp) {
			let Some(context) = self.context() else {
				// the two double encoding cases sign at their own clock
				let date = self
					.canonical
					.lines()
					.find_map(|line| line.strip_prefix("x-amz-date:"))
					.unwrap();
				let at = Timestamp::parse_iso8601(&format!(
					"{}-{}-{}T{}:{}:{}Z",
					&date[0..4],
					&date[4..6],
					&date[6..8],
					&date[9..11],
					&date[11..13],
					&date[13..15]
				))
				.unwrap();
				// the scope the signed request names, else the suite's own
				let scope = self
					.authorization()
					.and_then(|auth| auth.split("Credential=").nth(1))
					.map(|scope| scope.split('/').collect::<Vec<_>>());
				let (region, service) = match scope.as_deref() {
					Some([_, _, region, service, ..]) => (*region, *service),
					_ => ("us-east-1", "service"),
				};
				return (SigV4::new(region, service), suite_credentials(), at);
			};
			let mut signer = SigV4::new(
				context["region"].as_str().unwrap(),
				context["service"].as_str().unwrap(),
			);
			signer.rules = CanonicalRules {
				double_encode_path: false,
				normalize_path: context["normalize"].as_bool().unwrap(),
				content_sha256: context["sign_body"].as_bool().unwrap(),
				unsigned_payload: false,
			};
			let credentials = &context["credentials"];
			let mut creds = AwsCredentials::new(
				credentials["access_key_id"].as_str().unwrap(),
				credentials["secret_access_key"].as_str().unwrap(),
			);
			if let Some(token) = credentials["token"].as_str() {
				creds = creds.with_session_token(token);
			}
			let at = Timestamp::parse_iso8601(
				context["timestamp"].as_str().unwrap(),
			)
			.unwrap();
			(signer, creds, at)
		}

		/// The `Authorization` the signed request carries.
		fn authorization(&self) -> Option<&'static str> {
			self.signed?.lines().find_map(|line| {
				line.strip_prefix("Authorization:").map(str::trim)
			})
		}

		/// The case's `request.txt` as the wire request it describes: the
		/// request line, headers with folded continuation lines, then the
		/// body after a blank line.
		fn wire(&self) -> WireRequest {
			let (head, body) = self
				.request
				.split_once("\n\n")
				.unwrap_or((self.request, ""));
			let mut lines = head.lines();
			let request_line = lines.next().unwrap();
			let (method, rest) = request_line.split_once(' ').unwrap();
			let target = rest.rsplit_once(' ').unwrap().0;
			let (path, query) = target.split_once('?').unwrap_or((target, ""));
			let mut headers: Vec<(String, String)> = Vec::new();
			for line in lines {
				match line.starts_with([' ', '\t']) {
					// a folded continuation of the header above
					true => {
						let (_, value) = headers.last_mut().unwrap();
						value.push(' ');
						value.push_str(line.trim());
					}
					false => {
						let (name, value) = line.split_once(':').unwrap();
						headers.push((name.to_string(), value.to_string()));
					}
				}
			}
			WireRequest {
				method: method.into(),
				path: path.into(),
				query: query.into(),
				headers,
				payload_hash: digest_ext::hex::<Sha256>(body.as_bytes()),
			}
		}
	}

	/// Every applicable case of AWS's Signature Version 4 suite: the
	/// canonical request, the string to sign and the signature it publishes.
	#[beet_core::test]
	fn passes_the_suite() {
		for case in suite() {
			let (signer, credentials, at) = case.signer();
			let wire = case.wire();
			let added = SignedHeaders {
				amz_date: SigV4::amz_date(at),
				security_token: credentials
					.session_token()
					.map(ToString::to_string),
				content_sha256: signer
					.rules
					.content_sha256
					.then(|| wire.payload_hash.clone()),
				authorization: String::new(),
			};
			let (canonical, _) =
				signer.canonical_request(&wire, &added.headers());
			(case.name, canonical.as_str())
				.xpect_eq((case.name, case.canonical.trim_end()));
			let signed = signer.sign_wire(&wire, &credentials, at);
			if let Some(string_to_sign) = case.string_to_sign {
				let hash = string_to_sign.lines().last().unwrap();
				(case.name, digest_ext::hex::<Sha256>(canonical.as_bytes()))
					.xpect_eq((case.name, hash.to_string()));
			}
			if let Some(expected) = case.authorization() {
				(case.name, signed.authorization.as_str())
					.xpect_eq((case.name, expected));
			}
		}
	}

	/// `get-vanilla` through a beet [`Request`]: the published signature,
	/// the host taken from the url.
	#[beet_core::test]
	async fn signs_a_request() {
		SigV4::new("us-east-1", "service")
			.sign(
				Request::get("https://example.amazonaws.com/"),
				&suite_credentials(),
				suite_time(),
			)
			.await
			.unwrap()
			.headers
			.first_raw("authorization")
			.unwrap()
			.xpect_eq(
				"AWS4-HMAC-SHA256 \
				Credential=AKIDEXAMPLE/20150830/us-east-1/service/aws4_request, \
				SignedHeaders=host;x-amz-date, \
				Signature=5fa00fa31553b73ebf1942676e86291e8372ff2a2260956d9b8aae1d763fbf31",
			);
	}

	/// A parameter store call: the content type, the target and the session
	/// token are signed beside the host and the date, and the body is hashed
	/// in. The signature is a fixture computed independently of this crate
	/// (Python's `hmac` over the canonical request), so a change in what is
	/// signed is caught here.
	#[beet_core::test]
	async fn signs_a_json_call() {
		let request =
			Request::post("https://ssm.ap-southeast-2.amazonaws.com/")
				.with_header_raw("content-type", "application/x-amz-json-1.1")
				.with_header_raw("x-amz-target", "AmazonSSM.GetParameter")
				.with_body(r#"{"Name":"/app/prod/x","WithDecryption":true}"#);
		let signed = SigV4::new("ap-southeast-2", "ssm")
			.sign(
				request,
				&suite_credentials().with_session_token("TOKEN"),
				suite_time(),
			)
			.await
			.unwrap();
		signed
			.headers
			.first_raw("x-amz-date")
			.unwrap()
			.xpect_eq("20150830T123600Z");
		signed
			.headers
			.first_raw("x-amz-security-token")
			.unwrap()
			.xpect_eq("TOKEN");
		signed
			.headers
			.first_raw("authorization")
			.unwrap()
			.xpect_eq(
				"AWS4-HMAC-SHA256 \
				Credential=AKIDEXAMPLE/20150830/ap-southeast-2/ssm/aws4_request, \
				SignedHeaders=content-type;host;x-amz-date;x-amz-security-token;x-amz-target, \
				Signature=08d0f5fc85d81cf05c7636ff5ccd41b412f1c0587f634ca9bbb9ecbb7e47bbe8",
			);
		// the body is untouched by the signing
		signed
			.body
			.into_bytes()
			.await
			.unwrap()
			.as_ref()
			.xpect_eq(br#"{"Name":"/app/prod/x","WithDecryption":true}"#);
	}

	/// S3 signs the decoded key encoded once, sends the payload hash, and a
	/// query flag keeps its `=`; a port other than the scheme's is in the host.
	#[beet_core::test]
	fn s3_canonical_form() {
		let signer = SigV4::s3("us-east-1");
		signer
			.canonical_path("/bucket/a+b/c%20d/e(1)")
			.xpect_eq("/bucket/a%2Bb/c%20d/e%281%29");
		SigV4::canonical_query("versioning&list-type=2&prefix=a%2Fb")
			.xpect_eq("list-type=2&prefix=a%2Fb&versioning=");
		SigV4::host_of(&"http://127.0.0.1:9000/x".parse().unwrap())
			.unwrap()
			.xpect_eq("127.0.0.1:9000");
		SigV4::host_of(&"https://s3.amazonaws.com:443/x".parse().unwrap())
			.unwrap()
			.xpect_eq("s3.amazonaws.com");
	}

	/// Signing keeps the request's redirect policy.
	///
	/// REGRESSION: the signed request was rebuilt from its parts, which do not
	/// carry the policy, so every AWS call followed redirects again and S3's
	/// region `301` (no `Location`) failed in the transport rather than
	/// reaching the client's region retry.
	#[beet_core::test]
	async fn keeps_the_redirect_policy() {
		SigV4::s3("us-east-1")
			.sign(
				Request::head("https://bucket.s3.amazonaws.com/")
					.with_redirects(Redirects::None),
				&suite_credentials(),
				suite_time(),
			)
			.await
			.unwrap()
			.redirects
			.xpect_eq(Redirects::None);
	}

	/// An S3 request carries its payload hash, or `UNSIGNED-PAYLOAD` when told.
	#[beet_core::test]
	async fn s3_sends_the_payload_hash() {
		let sign = |signer: SigV4| async move {
			signer
				.sign(
					Request::put("https://bucket.s3.amazonaws.com/key")
						.with_body("hello"),
					&suite_credentials(),
					suite_time(),
				)
				.await
				.unwrap()
				.headers
				.first_raw("x-amz-content-sha256")
				.unwrap()
				.to_string()
		};
		sign(SigV4::s3("us-east-1")).await.xpect_eq(
			"2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824",
		);
		sign(SigV4::s3("us-east-1").with_unsigned_payload())
			.await
			.xpect_eq(SigV4::UNSIGNED_PAYLOAD);
	}
}
