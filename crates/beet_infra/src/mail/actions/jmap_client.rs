//! The management channel into a running Stalwart.
use beet_core::prelude::*;
use beet_net::prelude::*;
use serde_json::Value;
use serde_json::json;

/// A JMAP session against a Stalwart server: the one protocol `0.16` speaks for
/// both mail and administration.
///
/// `0.16` deleted the `/api/...` REST endpoints outright and moved every
/// configuration setting into the data store as a JMAP object, so listeners,
/// ACME, routing, domains and accounts are all created and updated through the
/// method calls below. Nothing here is mail-specific: the same client reads a
/// mailbox for [`MailProbe`] and writes a listener for [`StalwartProvision`].
pub struct JmapClient {
	/// Where the server is, ie `http://127.0.0.1:18080` through a tunnel or
	/// `https://mail.beetmash.com` in the open.
	origin: String,
	/// The api path the session document advertised, usually `/jmap`. Read
	/// rather than assumed, because the server is entitled to move it.
	api_path: String,
	/// The `Authorization` header value, held once so no call site handles the
	/// password again.
	auth: String,
	/// The account the session's primary mail capability names, ie whose
	/// mailboxes `Email/query` searches. Absent for a management-only session,
	/// which is what a bootstrap admin has.
	mail_account: Option<String>,
}

impl JmapClient {
	/// The capabilities every management call declares: JMAP core plus
	/// Stalwart's own administrative extension, which is what makes the `x:`
	/// configuration objects addressable.
	pub const USING: &'static [&'static str] =
		&["urn:ietf:params:jmap:core", "urn:stalwart:jmap"];

	/// The capability urn whose primary account holds a user's mail.
	pub const MAIL_CAPABILITY: &'static str = "urn:ietf:params:jmap:mail";

	/// The well-known session document, the one path a client may assume.
	pub const SESSION_PATH: &'static str = "/jmap/session";

	/// Authenticate against `origin` and read the session document, which is
	/// both the handshake and the liveness check: a server that answers here is
	/// a server whose data store opened.
	pub async fn connect(
		origin: impl Into<String>,
		user: &str,
		password: &str,
	) -> Result<Self> {
		let origin = origin.into();
		let auth = Self::basic_auth(user, password);
		let response = Request::get(format!("{origin}{}", Self::SESSION_PATH))
			.with_auth_raw(&auth)
			.send()
			.await?;
		if !response.status().is_ok() {
			bevybail!(
				"jmap session at {origin} refused {user}: {}",
				response.status()
			);
		}
		let session: Value = response.json().await?;
		let api_path = session["apiUrl"]
			.as_str()
			.map(Self::url_to_path)
			.ok_or_else(|| bevyhow!("jmap session carried no apiUrl"))?;
		let mail_account = session["primaryAccounts"][Self::MAIL_CAPABILITY]
			.as_str()
			.map(String::from);
		Ok(Self {
			origin,
			api_path,
			auth,
			mail_account,
		})
	}

	/// The account whose mail this session reads.
	pub fn mail_account(&self) -> Result<&str> {
		self.mail_account.as_deref().ok_or_else(|| {
			bevyhow!(
				"this jmap session has no primary mail account, so it cannot \
				read a mailbox: authenticate as the mailbox owner rather than \
				as an administrator"
			)
		})
	}

	/// Invoke one method, returning its result. A JMAP-level `error` response
	/// is an error here rather than a value the caller has to inspect.
	pub async fn call(&self, method: &str, args: Value) -> Result<Value> {
		self.call_using(Self::USING, method, args).await
	}

	/// [`call`](Self::call) for a MAILBOX method (`Email/query`, `Email/get`):
	/// the same session, declaring the mail capability instead of the
	/// management one. Separate rather than merged into one `using` set
	/// because a bootstrap-mode session has no mail capability to declare, and
	/// a server may reject a capability it did not advertise.
	pub async fn call_mail(&self, method: &str, args: Value) -> Result<Value> {
		self.call_using(
			&["urn:ietf:params:jmap:core", Self::MAIL_CAPABILITY],
			method,
			args,
		)
		.await
	}

	async fn call_using(
		&self,
		using: &[&str],
		method: &str,
		args: Value,
	) -> Result<Value> {
		let mut responses = self
			.call_many_using(using, vec![(
				method.to_string(),
				args,
				"c0".to_string(),
			)])
			.await?;
		if responses.len() != 1 {
			bevybail!(
				"{method}: expected 1 method response, got {}",
				responses.len()
			);
		}
		let (name, result, _) = responses.remove(0);
		if name == "error" {
			bevybail!(
				"{method} failed: {} {}",
				result["type"].as_str().unwrap_or("(unknown)"),
				result["description"].as_str().unwrap_or_default()
			);
		}
		Ok(result)
	}

	/// Invoke a batch, returning the raw `(name, result, callId)` triples. The
	/// batch is what makes a back-reference possible (`#ids` reading the result
	/// of the call before it), which is how a query feeds a get in one trip.
	pub async fn call_many(
		&self,
		calls: Vec<(String, Value, String)>,
	) -> Result<Vec<(String, Value, String)>> {
		self.call_many_using(Self::USING, calls).await
	}

	async fn call_many_using(
		&self,
		using: &[&str],
		calls: Vec<(String, Value, String)>,
	) -> Result<Vec<(String, Value, String)>> {
		let response =
			Request::post(format!("{}{}", self.origin, self.api_path))
				.with_auth_raw(&self.auth)
				.with_json_body(&json!({
					"using": using,
					"methodCalls": calls,
				}))?
				.send()
				.await?;
		let status = response.status();
		let body = response.text().await.unwrap_or_default();
		if !status.is_ok() {
			bevybail!("jmap call failed: {status} - {body}");
		}
		serde_json::from_str::<Value>(&body)?["methodResponses"]
			.as_array()
			.ok_or_else(|| {
				bevyhow!("jmap response carried no methodResponses")
			})?
			.iter()
			.map(|call| -> Result<(String, Value, String)> {
				(
					call[0].as_str().unwrap_or_default().to_string(),
					call[1].clone(),
					call[2].as_str().unwrap_or_default().to_string(),
				)
					.xok()
			})
			.collect()
	}

	/// Every object of `object_type`, as full bodies.
	///
	/// A query for the ids and a get for the bodies in one batch, then matching
	/// happens locally. Deliberately not a server-side filter: the properties a
	/// declaration matches on differ per object type, and a filter the server
	/// does not implement fails as a silent empty result, which reads as
	/// "nothing exists yet" and creates a duplicate.
	pub async fn list(&self, object_type: &str) -> Result<Vec<Value>> {
		let query = format!("{object_type}/query");
		let get = format!("{object_type}/get");
		let responses = self
			.call_many(vec![
				(query.clone(), json!({ "filter": {} }), "q".to_string()),
				(
					get.clone(),
					json!({
						"#ids": { "resultOf": "q", "name": query, "path": "/ids" },
						"properties": Value::Null,
					}),
					"g".to_string(),
				),
			])
			.await?;
		let (name, result, _) = responses
			.into_iter()
			.find(|(name, _, id)| name == &get || id == "g")
			.ok_or_else(|| bevyhow!("{get}: no response"))?;
		if name == "error" {
			bevybail!(
				"{get} failed: {} {}",
				result["type"].as_str().unwrap_or("(unknown)"),
				result["description"].as_str().unwrap_or_default()
			);
		}
		result["list"].as_array().cloned().unwrap_or_default().xok()
	}

	/// Create one object, returning the id the server assigned it.
	pub async fn create(
		&self,
		object_type: &str,
		value: &Value,
	) -> Result<String> {
		let result = self
			.call(
				&format!("{object_type}/set"),
				json!({ "create": { "new": value } }),
			)
			.await?;
		Self::reject_set_errors(object_type, &result, "notCreated")?;
		result["created"]["new"]["id"]
			.as_str()
			.map(String::from)
			.ok_or_else(|| {
				bevyhow!("{object_type}/set created an object with no id")
			})
	}

	/// Patch one object by id. `patch` carries only the properties that differ,
	/// so a server-set property the declaration never mentions is left alone.
	pub async fn update(
		&self,
		object_type: &str,
		id: &str,
		patch: &Value,
	) -> Result {
		self.update_returning(object_type, id, patch).await?;
		Ok(())
	}

	/// [`update`](Self::update), returning whatever the server attached to the
	/// updated id. Almost always `null`; the exception is the `Bootstrap`
	/// claim, whose response carries the admin credential the server minted.
	pub async fn update_returning(
		&self,
		object_type: &str,
		id: &str,
		patch: &Value,
	) -> Result<Value> {
		let result = self
			.call(
				&format!("{object_type}/set"),
				json!({ "update": { id: patch } }),
			)
			.await?;
		Self::reject_set_errors(object_type, &result, "notUpdated")?;
		result["updated"][id].clone().xok()
	}

	/// Delete one object by id.
	///
	/// The one write that is not additive, so it is called only where a
	/// declaration says an object should not exist at all — a listener the
	/// server seeded and this stack never asked for. `x:` objects carry no
	/// disable flag, so removal is the only way to close a port.
	pub async fn destroy(&self, object_type: &str, id: &str) -> Result {
		let result = self
			.call(&format!("{object_type}/set"), json!({ "destroy": [id] }))
			.await?;
		Self::reject_set_errors(object_type, &result, "notDestroyed")
	}

	/// The singleton of `object_type` if the server holds one, else `None`.
	///
	/// The one caller that matters is the bootstrap probe: `x:Bootstrap/get`
	/// answers with its singleton exactly while the server is in bootstrap
	/// mode, and `notFound` from the moment the data store is claimed.
	pub async fn try_get_singleton(
		&self,
		object_type: &str,
	) -> Result<Option<Value>> {
		let result = self
			.call(
				&format!("{object_type}/get"),
				json!({ "ids": [Self::SINGLETON_ID] }),
			)
			.await?;
		result["list"]
			.as_array()
			.and_then(|list| list.first())
			.cloned()
			.xok()
	}

	/// Patch the one instance of a singleton object type (system settings, the
	/// spam filter, the outbound strategy), which is addressed by the literal
	/// id `singleton` rather than by a query.
	pub async fn update_singleton(
		&self,
		object_type: &str,
		patch: &Value,
	) -> Result {
		self.update(object_type, Self::SINGLETON_ID, patch).await
	}

	/// The id every singleton object type answers to.
	pub const SINGLETON_ID: &'static str = "singleton";

	/// Turn a `/set` response's per-object failures into an error. They arrive
	/// beside a `200`, so a caller that only checked the status would report a
	/// successful provision of nothing.
	fn reject_set_errors(
		object_type: &str,
		result: &Value,
		key: &str,
	) -> Result {
		let Some(failures) = result[key].as_object() else {
			return Ok(());
		};
		if failures.is_empty() {
			return Ok(());
		}
		let (id, error) = failures.iter().next().unwrap();
		bevybail!(
			"{object_type}/set rejected '{id}': {}",
			serde_json::to_string(error).unwrap_or_default()
		)
	}

	/// `Basic <base64>`, the credential both a bootstrap admin and a mailbox
	/// owner authenticate with.
	fn basic_auth(user: &str, password: &str) -> String {
		use base64::Engine;
		format!(
			"Basic {}",
			base64::engine::general_purpose::STANDARD
				.encode(format!("{user}:{password}"))
		)
	}

	/// The path half of the session's `apiUrl`, which may be absolute. The
	/// origin is the one we dialled: through a tunnel the server's own idea of
	/// its public origin is not reachable from here.
	fn url_to_path(url: &str) -> String {
		let after = match url.split_once("://") {
			Some((scheme, rest))
				if scheme.eq_ignore_ascii_case("http")
					|| scheme.eq_ignore_ascii_case("https") =>
			{
				rest
			}
			_ => return url.trim_end_matches('/').to_string(),
		};
		match after.find('/') {
			Some(start) => after[start..].trim_end_matches('/').to_string(),
			None => "/".to_string(),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	/// The tunnel is the reason this exists: the server publishes its public
	/// https origin in `apiUrl`, which is not the origin a forwarded port
	/// reached it on, so only the path may be taken from it.
	#[beet_core::test]
	fn the_api_url_contributes_only_its_path() {
		JmapClient::url_to_path("https://mail.beetmash.com/jmap")
			.xpect_eq("/jmap");
		JmapClient::url_to_path("http://127.0.0.1:8080/jmap/")
			.xpect_eq("/jmap");
		JmapClient::url_to_path("/jmap/").xpect_eq("/jmap");
	}

	/// The header is the one every management call carries, so its encoding is
	/// pinned rather than trusted.
	#[beet_core::test]
	fn basic_auth_is_the_standard_encoding() {
		JmapClient::basic_auth("user", "pass").xpect_eq("Basic dXNlcjpwYXNz");
	}

	/// A `/set` reports per-object failure inside a 200, so a provision that
	/// only checked the status would report success having created nothing.
	#[beet_core::test]
	fn set_failures_inside_a_success_are_errors() {
		let result = serde_json::json!({
			"created": {},
			"notCreated": { "new": { "type": "invalidProperties" } }
		});
		JmapClient::reject_set_errors("x:Domain", &result, "notCreated")
			.unwrap_err()
			.to_string()
			.xpect_contains("invalidProperties");
		JmapClient::reject_set_errors(
			"x:Domain",
			&serde_json::json!({ "created": { "new": { "id": "a" } } }),
			"notCreated",
		)
		.unwrap();
	}
}
