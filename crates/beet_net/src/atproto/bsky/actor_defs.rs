//! `app.bsky.actor.defs`, the account views an AppView hydrates.
use beet_core::prelude::*;

/// `app.bsky.actor.defs#profileViewBasic`: the account fields every hydrated
/// post carries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileViewBasic {
	/// The account's did, its permanent identity.
	pub did: Did,
	/// The current handle, ie `alice.bsky.social`.
	pub handle: SmolStr,
	/// The chosen display name, absent when never set.
	#[serde(
		default,
		rename = "displayName",
		skip_serializing_if = "Option::is_none"
	)]
	pub display_name: Option<String>,
	/// An absolute avatar url on the Bluesky cdn.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub avatar: Option<String>,
}

impl ProfileViewBasic {
	/// The name to show for this account: the display name when set, else the
	/// handle.
	pub fn label(&self) -> &str {
		self.display_name
			.as_deref()
			.filter(|name| !name.is_empty())
			.unwrap_or(&self.handle)
	}
}
