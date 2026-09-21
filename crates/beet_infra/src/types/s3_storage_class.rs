//! The storage class a bucket keeps its objects in, declared once on the
//! bucket and read by both the transition rule and the push that lands them.

use beet_core::prelude::*;

/// The storage class a bucket's objects are kept in, in the S3 API vocabulary
/// the aws cli, the tofu provider and an S3-compatible store (R2) all read.
///
/// Declared once on the bucket, ie `<S3BucketBlock label="archive"
/// storage_class="GlacierIr"/>`, and read by both halves of the declaration:
/// the deploy renders a lifecycle transition, so every object already in the
/// bucket moves in place without a re-upload, and a push into the bucket
/// uploads new objects there directly. The classes below Standard are cheaper
/// per GB-month and trade for it in the same three coins: a minimum billable
/// size per object, a minimum storage duration (an object deleted or expired
/// sooner is billed for the rest of it) and a per-GB retrieval fee.
///
/// # Pricing
///
/// Rates are us-east-1 (N. Virginia) as of 26-09-21. Sydney (ap-southeast-2)
/// runs slightly higher; confirm on the
/// [AWS S3 pricing page](https://aws.amazon.com/s3/pricing/).
///
/// | Class | Rate/GB-month | 100 GB/month | Retrieval time | Min duration |
/// |---|---|---|---|---|
/// | Standard | $0.023 | $2.30 | Instant | None |
/// | Standard-IA (Infrequent Access) | $0.0125 | $1.25 | Instant | 30 days |
/// | One Zone-IA | $0.01 | $1.00 | Instant | 30 days |
/// | Glacier Instant Retrieval | $0.004 | $0.40 | Milliseconds | 90 days |
/// | Glacier Flexible Retrieval | $0.0036 | $0.36 | Minutes to 12 hrs | 90 days |
/// | Glacier Deep Archive | $0.00099 | $0.10 | 12 to 48 hrs | 180 days |
#[derive(
	Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect,
)]
#[reflect(Default)]
pub enum S3StorageClass {
	/// No minimums and no retrieval fee: the class for data read often, and the
	/// only one a lifecycle rule cannot transition into.
	#[default]
	Standard,
	/// Millisecond access at about half the price, a 128 KB minimum billable
	/// size, a 30 day minimum duration and a retrieval fee. A transition into
	/// it waits the 30 days at Standard, so a bucket declaring it pays full
	/// price for its first month of objects.
	StandardIa,
	/// [`StandardIa`](Self::StandardIa) in one availability zone: cheaper, and
	/// lost with the zone.
	OneZoneIa,
	/// Tiered by S3 itself on each object's access, for a monitoring fee per
	/// object; an object under 128 KB is never tiered and always bills as
	/// Standard. For an access pattern nobody can state.
	IntelligentTiering,
	/// Glacier Instant Retrieval: millisecond access at about a sixth of the
	/// price, a 128 KB minimum billable size, a 90 day minimum duration and a
	/// retrieval fee. The class for a write-once record read a few times a
	/// year, ie an archive bucket.
	GlacierIr,
	/// Glacier Flexible Retrieval: an object must be restored (minutes to
	/// hours, priced by speed) before it can be read, and `aws s3 sync` skips
	/// its objects until then, so a `pull` from it needs a restore step first.
	Glacier,
	/// The cheapest class, hours to restore and a 180 day minimum duration:
	/// for a copy that is only ever read in a disaster.
	DeepArchive,
}

impl S3StorageClass {
	/// The AWS spelling, ie `GLACIER_IR`.
	pub fn as_str(&self) -> &'static str {
		match self {
			Self::Standard => "STANDARD",
			Self::StandardIa => "STANDARD_IA",
			Self::OneZoneIa => "ONEZONE_IA",
			Self::IntelligentTiering => "INTELLIGENT_TIERING",
			Self::GlacierIr => "GLACIER_IR",
			Self::Glacier => "GLACIER",
			Self::DeepArchive => "DEEP_ARCHIVE",
		}
	}

	/// The class an upload declares, `None` for the bucket default: a push
	/// passes it as `--storage-class` and a transition rule is rendered for
	/// it, neither of which [`Standard`](Self::Standard) needs.
	pub fn declared(self) -> Option<Self> {
		(self != Self::Standard).then_some(self)
	}

	/// The days a lifecycle transition into this class waits after an object
	/// is created: the Glacier classes and Intelligent-Tiering accept an object
	/// on day zero, the infrequent-access classes only after 30 days at
	/// Standard.
	pub fn min_transition_days(&self) -> i64 {
		match self {
			Self::StandardIa | Self::OneZoneIa => 30,
			_ => 0,
		}
	}
}

impl std::fmt::Display for S3StorageClass {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(self.as_str())
	}
}
