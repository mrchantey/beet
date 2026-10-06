//! The suites that talk to the network: unauthenticated reads of real
//! accounts, and writes to the throwaway account the module docs' runbook
//! creates, skipped with a note naming its env vars when any is absent.
//!
//! Ignored by default, since they need the network: `just test-atproto-live`
//! compiles them with a transport and opts in.
use crate::prelude::*;
use beet_core::prelude::*;

/// beet.org's did, whose profile carries a blob.
const BEET_ORG: &str = "did:plc:vgxy56shhs3t3b2mu2avizjf";

/// The throwaway account's repo, or `None` after a note naming what is
/// missing.
fn throwaway() -> Option<(SmolStr, Pds)> {
	const VARS: [&str; 3] = [
		"BEET_TEST_ATPROTO_HANDLE",
		"BEET_TEST_ATPROTO_DID",
		"BEET_TEST_ATPROTO_APP_PASSWORD",
	];
	let missing = VARS
		.iter()
		.filter(|var| env_ext::var(var).is_err())
		.copied()
		.collect::<Vec<_>>();
	if !missing.is_empty() {
		warn!(
			"skipping a live atproto write: {} not set, see the `atproto` \
			 module docs for the throwaway account",
			missing.join(", ")
		);
		return None;
	}
	let handle = env_ext::var(VARS[0]).ok()?;
	let did = Did::parse(&env_ext::var(VARS[1]).ok()?).unwrap();
	let auth = AppPassword::new(did.clone(), VARS[2]);
	Some((
		handle.into(),
		Pds::new(XrpcPds::new(did).with_auth(Some(auth))),
	))
}

/// A record read from a real PDS answers the cid the emulator computes for
/// its body, which is what keeps a repo in a bucket and one on the network
/// interchangeable.
#[ignore = "live: talks to the public network"]
#[beet_core::test(timeout_ms = 30_000)]
async fn a_live_record_cid_matches() {
	let entry = Pds::new(XrpcPds::new(Did::parse(BEET_ORG).unwrap()))
		.get_record(&Nsid::new_static("app.bsky.actor.profile"), &Rkey::SELF)
		.await
		.unwrap()
		.unwrap();
	dag_cbor_ext::record_cid(&entry.value)
		.unwrap()
		.xpect_eq(entry.cid);
}

/// Both resolvers agree on beet.org's handles.
#[ignore = "live: talks to the public network"]
#[beet_core::test(timeout_ms = 30_000)]
async fn resolves_live_handles() {
	let did = Did::parse(BEET_ORG).unwrap();
	HandleResolver::default()
		.resolve("beet.org")
		.await
		.unwrap()
		.xpect_eq(did.clone());
	HandleResolver::service(HandleResolver::PUBLIC_APPVIEW)
		.resolve("@beet.org")
		.await
		.unwrap()
		.xpect_eq(did.clone());
	DidResolver::default()
		.resolve(&did)
		.await
		.unwrap()
		.handle()
		.xpect_eq(Some("beet.org"));
}

/// A read-only repo refuses a write naming the missing secret, before any
/// request.
#[ignore = "live: talks to the public network"]
#[beet_core::test(timeout_ms = 30_000)]
async fn a_read_only_repo_refuses_writes() {
	Pds::new(XrpcPds::new(Did::parse(BEET_ORG).unwrap()))
		.put_record(
			&Nsid::new_static("com.example.doc"),
			&Rkey::parse("never").unwrap(),
			value!({}).into(),
		)
		.await
		.unwrap_err()
		.to_string()
		.xpect_contains("names no `secret`");
}

/// The converge suite the emulator passes, against the throwaway account.
#[ignore = "live: talks to the public network"]
#[beet_core::test(timeout_ms = 120_000)]
async fn converges_a_live_repo() {
	let Some((handle, pds)) = throwaway() else {
		return;
	};
	HandleResolver::default()
		.resolve(&handle)
		.await
		.unwrap()
		.xpect_eq(pds.did().clone());
	super::converge::test::converge_suite(&pds).await;
}
