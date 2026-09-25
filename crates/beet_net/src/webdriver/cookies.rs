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

	/// Whether a request for `path` carries this cookie, the RFC 6265 §5.1.4
	/// path-match: the paths are equal, or the cookie's path is a prefix of
	/// it ending at a `/` boundary. This is what separates two cookies of the
	/// same name, which Blackboard and any other app mounting a servlet under
	/// a prefix will set (a `JSESSIONID` for `/` and another for
	/// `/learn/api`): sending both unscoped hands the server the wrong
	/// session half the time.
	pub fn matches_path(&self, path: &str) -> bool {
		let cookie_path = self.path.as_str();
		if cookie_path == "/" || cookie_path == path {
			return true;
		}
		let Some(rest) = path.strip_prefix(cookie_path) else {
			return false;
		};
		cookie_path.ends_with('/') || rest.starts_with('/')
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

	/// The cookies a request to `host` would carry, whatever its path: the
	/// whole of one site's session, for holding across many requests.
	/// [`Self::for_url`] narrows it to one request.
	pub fn for_host(&self, host: &str) -> Self {
		self.0
			.iter()
			.filter(|cookie| cookie.matches_host(host))
			.cloned()
			.collect()
	}

	/// The cookies one request to `url` carries: its host and its path, most
	/// specific path first (RFC 6265 §5.4, which is the order a server
	/// reading only the first of a repeated name depends on). A url with no
	/// host is scoped by path alone, on the assumption the caller has already
	/// picked the site.
	pub fn for_url(&self, url: &Url) -> Self {
		let host = url.host();
		let path = url.path_string();
		let mut scoped = self
			.0
			.iter()
			.filter(|cookie| {
				host.is_none_or(|host| cookie.matches_host(host))
					&& cookie.matches_path(&path)
			})
			.cloned()
			.collect::<Vec<_>>();
		// stable, so cookies of equal path depth keep the browser's order
		scoped.sort_by_key(|cookie| core::cmp::Reverse(cookie.path.len()));
		Self(scoped)
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

	/// The `cookie` header value, `name=value` pairs joined by `; `, of
	/// exactly the cookies held. Scope first, with [`Self::for_url`] for one
	/// request or [`Self::for_host`] for a site: the browser's whole jar sent
	/// to one host would carry every other site's session too, and its
	/// unscoped paths would repeat a name.
	pub fn header(&self) -> String {
		self.0
			.iter()
			.map(|cookie| format!("{}={}", cookie.name, cookie.value))
			.collect::<Vec<_>>()
			.join("; ")
	}

	/// `request` carrying the cookies it is scoped to, the hand-off: the
	/// scoping reads the request's own url ([`Self::for_url`]), so a set held
	/// for a whole site cannot leak a path's cookie into a sibling path's
	/// request. No header is set when nothing matches.
	pub fn apply(&self, request: Request) -> Request {
		let scoped = self.for_url(request.url());
		match scoped.is_empty() {
			true => request,
			false => request.with_header_raw("cookie", &scoped.header()),
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
		// a host whose cookies match nothing sets no header
		cookies
			.for_host("example.org")
			.apply(Request::get("https://example.org/x"))
			.headers
			.first_raw("cookie")
			.xpect_eq(Some("other=no"));
		format!("{scoped:?}")
			.xpect_contains("<redacted>")
			.xnot()
			.xpect_contains("abc");
		Cookie::new("a", "b", "example.com")
			.matches_host("deep.sub.example.com")
			.xpect_true();
	}

	/// One name set at two paths, the shape a servlet under a prefix
	/// produces: a request carries only the matching one, most specific
	/// first, never both unscoped.
	#[beet_core::test]
	fn scopes_a_repeated_name_by_path() {
		let at = |path: &str, value: &str| Cookie {
			path: path.into(),
			..Cookie::new("JSESSIONID", value, "example.com")
		};
		let cookies: Cookies = [
			at("/", "root"),
			at("/learn/api", "api"),
			at("/webapps/other", "other"),
		]
		.into_iter()
		.collect();

		// the deeper path wins the ordering, and the sibling is excluded
		cookies
			.for_url(
				&Url::parse("https://example.com/learn/api/v1/users/me")
					.unwrap(),
			)
			.header()
			.xpect_eq("JSESSIONID=api; JSESSIONID=root");
		// a path outside both prefixes gets only the root cookie
		cookies
			.for_url(&Url::parse("https://example.com/ultra/courses").unwrap())
			.header()
			.xpect_eq("JSESSIONID=root");
		// a prefix match must land on a boundary: `/learn/apix` is not under
		// `/learn/api`
		at("/learn/api", "api")
			.matches_path("/learn/apix")
			.xpect_false();
		at("/learn/api", "api")
			.matches_path("/learn/api")
			.xpect_true();
		at("/learn/api/", "api")
			.matches_path("/learn/api/v1")
			.xpect_true();
		// apply scopes from the request itself, so a site-wide set is safe
		cookies
			.apply(Request::get("https://example.com/learn/api/v1/x"))
			.headers
			.first_raw("cookie")
			.xpect_eq(Some("JSESSIONID=api; JSESSIONID=root"));
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
