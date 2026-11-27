//! The suites that talk to the network: unauthenticated reads of real
//! accounts, and writes to the test org the module docs' runbook sets up,
//! skipped with a note naming its secret when it is absent.
//!
//! Ignored by default, since they need the network: `just test-atproto-live`
//! compiles them with a transport and opts in.
use crate::prelude::*;
use beet_core::prelude::*;

/// beet.org's did, whose profile carries a blob.
const BEET_ORG: &str = "did:plc:vgxy56shhs3t3b2mu2avizjf";

/// A record read from a real PDS answers the cid the emulator computes for
/// its body, which is what keeps a PDS in a bucket and one on the network
/// interchangeable.
#[ignore = "live: talks to the public network"]
#[beet_core::test(timeout_ms = 30_000)]
async fn a_live_record_cid_matches() {
	let entry = Pds::new(XrpcPds::new(Did::parse(BEET_ORG).unwrap()))
		.get_record(&Nsid::new_static("app.bsky.actor.profile"), &Rkey::SELF)
		.await
		.unwrap()
		.unwrap();
	entry.value.cid().xpect_eq(entry.cid);
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
	HandleResolver::service(AppView::PUBLIC)
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
			Rkeyed::new(
				Rkey::parse("never").unwrap(),
				value!({}).try_into().unwrap(),
			),
		)
		.await
		.unwrap_err()
		.to_string()
		.xpect_contains("names no `secret`");
}

/// The converge suite the emulator passes, against the test org, whose
/// handle resolves to it once claimed.
#[ignore = "live: talks to the public network"]
#[beet_core::test(timeout_ms = 120_000)]
async fn converges_a_live_repo() {
	let Some(pds) = XrpcPds::test_org() else {
		return;
	};
	HandleResolver::default()
		.resolve(XrpcPds::TEST_ORG_HANDLE)
		.await
		.unwrap()
		.xpect_eq(pds.did().clone());
	super::converge::test::converge_suite(&pds).await;
}

/// The AppView's hydration paths: a published feed page, the same posts
/// fetched again by uri, and the thread of the first.
#[ignore = "live: talks to the public network"]
#[beet_core::test(timeout_ms = 60_000)]
async fn reads_the_live_appview() {
	let appview = AppView::default();
	let page = appview
		.get_feed(AppView::DISCOVER_FEED, 3, None)
		.await
		.unwrap()
		.feed;
	// the AppView omits entries it will not serve (deleted, moderated), so a
	// page is at most the limit asked for
	page.len().xpect_greater_than(0).xpect_less_or_equal_to(3);
	let uris = page
		.iter()
		.map(|item| item.post.uri.clone())
		.collect::<Vec<_>>();
	appview
		.get_posts(&uris)
		.await
		.unwrap()
		.into_iter()
		.map(|post| post.uri)
		.collect::<Vec<_>>()
		.xpect_eq(uris.clone());
	appview
		.get_post_thread(&uris[0], 1, 0)
		.await
		.unwrap()
		.post()
		.unwrap()
		.post
		.uri
		.xpect_eq(uris[0].clone());
}
