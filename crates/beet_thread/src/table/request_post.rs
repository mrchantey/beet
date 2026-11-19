//! The message of the request a thread's session serves.
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_router::prelude::*;

/// The message of the request a thread's session serves: `text` with each
/// `{param}` replaced by that request param, ie a path param of the
/// [`ThreadRoute`] building the thread. Authored beside the seed, it is no
/// part of it: every request posts it as its session opens, a continued
/// thread included ([`RequestMessage`]), so each request asks its thread
/// something.
///
/// ```bsx
/// <Route path="review/:document" {ThreadRoute{template:"ReviewThread"}}/>
/// <!-- ReviewThread.bsx -->
/// <CreateActor name="User" kind="User"><RequestPost text="review {document}"/></CreateActor>
/// ```
///
/// Read from the [`RequestContextStack`] while the thread builds. Which thread a request continues is the route's metadata
/// ([`ThreadEntry`]), never this text. `{{` and `}}` spell a literal brace.
///
/// ## Errors
///
/// Errors outside a request build, on a param the request lacks, and on an
/// unclosed `{`.
#[template(system)]
pub fn RequestPost(
	/// The post's text, each `{param}` the request's value for it.
	#[prop(required)]
	text: String,
	stack: Res<RequestContextStack>,
) -> Result<impl Bundle> {
	let parts = stack
		.try_current()
		.ok_or_else(|| {
			bevyhow!(
				"`<RequestPost text=\"{text}\"/>` is built outside a request: \
				 it seeds a thread a route builds per request, ie a `ThreadRoute`"
			)
		})?
		.parts();
	(Post::spawn(fill(&text, parts)?), RequestMessage).xok()
}

/// Marks a [`SeedPost`] as the message of the request its thread's session
/// serves: the reduction sets it aside rather than seeding the thread, and
/// [`ThreadWindow::open_session`] posts it after the session's opened post.
#[derive(Debug, Default, Clone, Copy, Component)]
pub struct RequestMessage;

/// `text` with each `{param}` replaced by its value in `parts`.
pub(crate) fn fill(text: &str, parts: &RequestParts) -> Result<String> {
	let mut out = String::with_capacity(text.len());
	let mut chars = text.chars().peekable();
	while let Some(next) = chars.next() {
		match (next, chars.peek()) {
			('{', Some('{')) | ('}', Some('}')) => {
				out.push(next);
				chars.next();
			}
			('{', _) => {
				let mut name = String::new();
				loop {
					match chars.next() {
						Some('}') => break,
						Some(ch) => name.push(ch),
						None => {
							bevybail!("`{text}` opens a `{{` it never closes")
						}
					}
				}
				let value = parts.get_param(name.trim()).ok_or_else(|| {
					bevyhow!(
						"`{text}` names the param `{name}`, which the request `{}` lacks",
						parts.path_string()
					)
				})?;
				out.push_str(value);
			}
			(other, _) => out.push(other),
		}
	}
	out.xok()
}

#[cfg(test)]
mod test {
	use super::*;

	fn parts(request: Request) -> RequestParts { request.parts().clone() }

	#[beet_core::test]
	fn fills_each_param() {
		fill(
			"review {document} as {who}, {{literally}}",
			&parts(
				Request::get("review/product")
					.with_param("document", "product")
					.with_param("who", "a reader"),
			),
		)
		.unwrap()
		.xpect_eq("review product as a reader, {literally}");
	}

	#[beet_core::test]
	fn refuses_a_missing_or_unclosed_param() {
		let parts = parts(Request::get("review/product"));
		fill("review {document}", &parts)
			.unwrap_err()
			.to_string()
			.xpect_contains(
				"names the param `document`, which the request `/review/product` lacks",
			);
		fill("review {document", &parts)
			.unwrap_err()
			.to_string()
			.xpect_contains("never closes");
	}
}
