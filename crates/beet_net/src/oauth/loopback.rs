use crate::prelude::*;
use beet_core::prelude::*;

/// A one-shot receiver for an authorization redirect (RFC 8252's loopback
/// redirect): beet's own [`HttpServer`] on `127.0.0.1` at an ephemeral port,
/// answering `/oauth/callback` with a page saying the tab may close and
/// handing the code to [`wait`](Self::wait). The server is spawned in the
/// caller's world and despawned shortly after the code arrives, once the
/// browser has its page.
pub struct LoopbackRedirect {
	server: AsyncEntity,
	port: u16,
	answers: async_channel::Receiver<AuthorizationAnswer>,
}

/// What an authorization redirect carried.
#[derive(Debug, Clone, PartialEq, Eq)]
struct AuthorizationAnswer {
	code: Option<SmolStr>,
	state: Option<SmolStr>,
	error: Option<SmolStr>,
}

impl LoopbackRedirect {
	/// The path the redirect lands on.
	pub const PATH: &'static str = "/oauth/callback";
	/// How long the server lingers after the redirect, for its page.
	const LINGER: Duration = Duration::from_secs(1);

	/// Listen in `world` for one redirect.
	pub async fn open(world: &AsyncWorld) -> Result<Self> {
		let (server, on_spawn) =
			HttpServer::bind_loopback(HttpServer::start_mini_with_tcp);
		let port = server.port.unwrap_or_default();
		let (send, answers) = async_channel::bounded(1);
		let server = world
			.spawn((server, on_spawn, children![exchange_ext::handler(
				move |cx| {
					let request = cx.take();
					if request.url().path_string() != Self::PATH {
						return Response::from_status(StatusCode::NOT_FOUND);
					}
					let param =
						|name: &str| request.get_param(name).map(SmolStr::new);
					let answer = AuthorizationAnswer {
						code: param("code"),
						state: param("state"),
						error: param("error"),
					};
					let page = match answer.error.is_some() {
						true => {
							"The sign in was refused. You can close this tab."
						}
						false => "Signed in. You can close this tab.",
					};
					send.try_send(answer).ok();
					Response::ok_text(page)
				}
			)]))
			.await;
		Self {
			server,
			port,
			answers,
		}
		.xok()
	}

	/// The redirect uri to register and to name in the authorization url.
	pub fn redirect_uri(&self) -> String {
		format!("http://127.0.0.1:{}{}", self.port, Self::PATH)
	}

	/// The port the receiver listens on.
	pub fn port(&self) -> u16 { self.port }

	/// The authorization code the redirect carried, its state checked
	/// against `state`; the receiver closes either way.
	pub async fn wait(self, state: &OAuthState) -> Result<SmolStr> {
		let answer = self.answers.recv().await;
		// the code arrives as the page is written, so the server outlives
		// this call by a moment rather than cutting the browser off
		let server = self.server.clone();
		async_ext::spawn(async move {
			time_ext::sleep(Self::LINGER).await;
			server.despawn().await.ok();
		})
		.detach();
		let answer =
			answer.map_err(|_| bevyhow!("the redirect receiver closed"))?;
		if let Some(error) = answer.error {
			bevybail!("the sign in was refused: {error}");
		}
		state.check(answer.state.as_deref())?;
		answer.code.ok_or_else(|| {
			bevyhow!("the redirect carried no authorization code")
		})
	}
}

#[cfg(all(test, feature = "ureq"))]
mod test {
	use super::*;

	/// The redirect's code is handed over once its state checks, the browser
	/// answered a closing page.
	#[beet_core::test]
	async fn receives_one_redirect() {
		let receiver = LoopbackRedirect::open(&crate::oauth::threaded_world())
			.await
			.unwrap();
		let state = OAuthState::new().unwrap();
		let uri = format!(
			"{}?code=abc&state={}",
			receiver.redirect_uri(),
			state.as_str()
		);
		let browser = async move {
			time_ext::sleep_millis(50).await;
			Request::get(uri)
				.send()
				.await
				.unwrap()
				.text()
				.await
				.unwrap()
		};
		let (code, page) = futures::join!(receiver.wait(&state), browser);
		code.unwrap().as_str().xpect_eq("abc");
		page.xpect_contains("close this tab");
	}
}
