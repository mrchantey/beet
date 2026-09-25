//! The browser's cookies, read through `storage.getCookies`: what a plain
//! http client needs to act as the same session (the cookie hand-off: the
//! browser logs in, the client fetches), and what a test asserts on after a
//! login.

use super::Page;
use crate::prelude::*;
use beet_core::prelude::*;
use serde_json::Value;
use serde_json::json;

/// One cookie the browser holds. [`Debug`] redacts the value;
/// [`Self::value`] is the deliberate read.
#[derive(Clone, PartialEq, Eq)]
pub struct Cookie {
	/// The cookie's name.
	pub name: SmolStr,
	value: SmolStr,
	/// The domain attribute, a leading `.` stripped: the host it was set
	/// for, or the parent domain whose every subdomain carries it.
	pub domain: SmolStr,
	/// The path attribute, `/` when unset.
	pub path: SmolStr,
	/// Whether scripts on the page cannot read it.
	pub http_only: bool,
	/// Whether it is sent over https only.
	pub secure: bool,
}

impl Cookie {
	/// A cookie for `domain` at path `/`, neither http-only nor secure.
	pub fn new(
		name: impl Into<SmolStr>,
		value: impl Into<SmolStr>,
		domain: impl Into<SmolStr>,
	) -> Self {
		Self {
			name: name.into(),
			value: value.into(),
			domain: domain.into(),
			path: "/".into(),
			http_only: false,
			secure: false,
		}
	}

	/// The value. Mind where it goes: a header, never a log.
	pub fn value(&self) -> &str { &self.value }

	/// Whether a request to `host` carries this cookie: the host is the
	/// domain, or a subdomain of it.
	pub fn matches_host(&self, host: &str) -> bool {
		host == self.domain || host.ends_with(&format!(".{}", self.domain))
	}

	/// From one entry of `storage.getCookies`' `cookies` array; `None` for
	/// a shape this does not read (a base64 value), with a warning.
	fn parse(cookie: &Value) -> Option<Self> {
		let field = |name: &str| cookie.get(name).and_then(Value::as_str);
		let value = cookie.get("value")?;
		if value.get("type").and_then(Value::as_str) != Some("string") {
			warn!(
				"cookie `{}` has a non-string value, skipping",
				field("name").unwrap_or("?")
			);
			return None;
		}
		Some(Self {
			name: field("name")?.into(),
			value: value.get("value")?.as_str()?.into(),
			domain: field("domain")?.trim_start_matches('.').into(),
			path: field("path").unwrap_or("/").into(),
			http_only: cookie
				.get("httpOnly")
				.and_then(Value::as_bool)
				.unwrap_or(false),
			secure: cookie
				.get("secure")
				.and_then(Value::as_bool)
				.unwrap_or(false),
		})
	}
}

impl core::fmt::Debug for Cookie {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("Cookie")
			.field("name", &self.name)
			.field("value", &"<redacted>")
			.field("domain", &self.domain)
			.field("path", &self.path)
			.field("http_only", &self.http_only)
			.field("secure", &self.secure)
			.finish()
	}
}

/// The cookies of one [`Page::cookies`] read, in the order the browser
/// returned them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Cookies(Vec<Cookie>);

impl Cookies {
	/// From a `storage.getCookies` response.
	fn parse(resp: &Value) -> Result<Self> {
		resp.pointer("/result/cookies")
			.and_then(Value::as_array)
			.ok_or_else(|| {
				bevyhow!("storage.getCookies: cookies array missing")
			})?
			.iter()
			.filter_map(Cookie::parse)
			.collect::<Self>()
			.xok()
	}

	/// The cookies a request to `host` would carry.
	pub fn for_host(&self, host: &str) -> Self {
		self.0
			.iter()
			.filter(|cookie| cookie.matches_host(host))
			.cloned()
			.collect()
	}

	/// The cookie named `name`, the first if several domains set one.
	pub fn get(&self, name: &str) -> Option<&Cookie> {
		self.0.iter().find(|cookie| cookie.name == name)
	}

	/// The cookies in order.
	pub fn iter(&self) -> impl Iterator<Item = &Cookie> { self.0.iter() }
	/// How many cookies.
	pub fn len(&self) -> usize { self.0.len() }
	/// Whether there are none.
	pub fn is_empty(&self) -> bool { self.0.is_empty() }

	/// The names, for a log line or a report.
	pub fn names(&self) -> Vec<&str> {
		self.0.iter().map(|cookie| cookie.name.as_str()).collect()
	}

	/// The `cookie` header value, `name=value` pairs joined by `; `. Scope
	/// with [`Self::for_host`] first: the browser's whole jar sent to one
	/// host would carry every other site's session too.
	pub fn header(&self) -> String {
		self.0
			.iter()
			.map(|cookie| format!("{}={}", cookie.name, cookie.value))
			.collect::<Vec<_>>()
			.join("; ")
	}

	/// `request` carrying these cookies: the hand-off.
	pub fn apply(&self, request: Request) -> Request {
		match self.is_empty() {
			true => request,
			false => request.with_header_raw("cookie", &self.header()),
		}
	}
}

impl FromIterator<Cookie> for Cookies {
	fn from_iter<I: IntoIterator<Item = Cookie>>(iter: I) -> Self {
		Self(iter.into_iter().collect())
	}
}

impl Page {
	/// Every cookie the browser holds in its default partition, whatever
	/// the host (`storage.getCookies` unfiltered: a filter by domain is an
	/// exact match and misses a cookie set on a parent domain). Scope with
	/// [`Cookies::for_host`].
	pub async fn cookies(&self) -> Result<Cookies> {
		self.session
			.command("storage.getCookies", json!({}))
			.await?
			.xmap(|resp| Cookies::parse(&resp))
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::webdriver::*;

	#[beet_core::test]
	fn parses_scopes_and_redacts() {
		let resp = json!({"id": 1, "result": {"cookies": [
			{"name": "session", "value": {"type": "string", "value": "abc"}, "domain": "www.example.com", "path": "/", "httpOnly": true, "secure": true},
			{"name": "shared", "value": {"type": "string", "value": "xyz"}, "domain": ".example.com", "path": "/"},
			{"name": "other", "value": {"type": "string", "value": "no"}, "domain": "example.org", "path": "/"},
			{"name": "bin", "value": {"type": "base64", "value": "AA=="}, "domain": "www.example.com", "path": "/"},
		]}});
		let cookies = Cookies::parse(&resp).unwrap();
		cookies.len().xpect_eq(3);
		let scoped = cookies.for_host("www.example.com");
		scoped.names().xpect_eq(vec!["session", "shared"]);
		scoped.header().xpect_eq("session=abc; shared=xyz");
		scoped.get("session").unwrap().http_only.xpect_true();
		cookies
			.for_host("api.example.com")
			.names()
			.xpect_eq(vec!["shared"]);
		cookies
			.for_host("example.org")
			.names()
			.xpect_eq(vec!["other"]);
		let request = scoped.apply(Request::get("https://www.example.com/x"));
		request
			.headers
			.first_raw("cookie")
			.xpect_eq(Some("session=abc; shared=xyz"));
		Cookies::default()
			.apply(Request::get("https://www.example.com/x"))
			.headers
			.first_raw("cookie")
			.xpect_eq(None);
		format!("{scoped:?}")
			.xpect_contains("<redacted>")
			.xnot()
			.xpect_contains("abc");
		Cookie::new("a", "b", "example.com")
			.matches_host("deep.sub.example.com")
			.xpect_true();
	}

	/// A served page's `set-cookie` lands in the jar and scopes to its host.
	#[cfg(feature = "server")]
	#[beet_core::test(timeout_ms = 30_000)]
	#[ignore = "smoketest"]
	async fn reads_a_served_cookie() {
		let page = PageHarness::visit(
			(),
			exchange_ext::handler(|_| {
				Response::ok_body("<h1>cookie</h1>", MediaType::Html)
					.with_header(
						"set-cookie",
						"beet_session=abc; Path=/; HttpOnly",
					)
			}),
			"/",
		)
		.await
		.unwrap();
		let host = Url::parse(page.url()).unwrap().host().unwrap().to_string();
		let cookies = page.cookies().await.unwrap().for_host(&host);
		cookies.get("beet_session").unwrap().value().xpect_eq("abc");
		cookies.get("beet_session").unwrap().http_only.xpect_true();
		page.kill().await.unwrap();
	}
}
