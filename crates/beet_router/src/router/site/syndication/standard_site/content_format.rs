//! The `content` formats a standard site document can carry.
use beet_core::prelude::*;

/// The `content` formats a publication may name, each by its NSID and mapped
/// to the media type of the render target that answers it, so a declaration
/// says exactly which object lands in each record and a name with no target
/// fails listing the registered ones.
///
/// Initialized empty by `RouterPlugin`, each format registered by the
/// installer of the target answering it: with the `leaflet` feature,
/// `pub.leaflet.content` by the `LeafletRenderer`.
#[derive(Debug, Default, Clone, Resource)]
pub struct StandardSiteContentFormats(Vec<(Nsid, MediaType)>);

impl StandardSiteContentFormats {
	/// Register `nsid` as answered by the target for `media_type`, replacing
	/// any format registered under the same NSID.
	pub fn register(&mut self, nsid: Nsid, media_type: MediaType) -> &mut Self {
		self.0.retain(|(registered, _)| registered != &nsid);
		self.0.push((nsid, media_type));
		self
	}

	/// The media type `nsid` renders as, an error listing the registered
	/// formats when none is.
	pub fn media_type(&self, nsid: &Nsid) -> Result<&MediaType> {
		self.0
			.iter()
			.find(|(registered, _)| registered == nsid)
			.map(|(_, media_type)| media_type)
			.ok_or_else(|| {
				bevyhow!(
					"no standard site content format `{nsid}`: the registered \
					 formats are {:?}",
					self.0
						.iter()
						.map(|(registered, _)| registered.as_str())
						.collect::<Vec<_>>()
				)
			})
	}

	/// The member of a document's `content` union that `bytes`, a render as
	/// `nsid`'s media type, makes: the rendered object wrapped under `nsid`,
	/// so its `$type` cannot disagree with the declaration.
	#[cfg(feature = "json")]
	pub fn member(&self, nsid: &Nsid, bytes: &MediaBytes) -> Result<OpenUnion> {
		let media_type = self.media_type(nsid)?;
		if bytes.media_type() != media_type {
			bevybail!(
				"a `{nsid}` member must be rendered as `{media_type}`, found `{}`",
				bytes.media_type()
			);
		}
		OpenUnion::new(nsid.clone(), MediaType::Json.deserialize(bytes)?)
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// A format of the tests' own, so the registry is tested apart from any
	/// target.
	fn example() -> (Nsid, MediaType) {
		(
			Nsid::new_static("com.example.content"),
			MediaType::other("application/vnd.com.example.content+json"),
		)
	}

	/// An unregistered NSID fails naming the registered ones.
	#[beet_core::test]
	fn names_the_registered_formats() {
		let (nsid, media_type) = example();
		let mut formats = StandardSiteContentFormats::default();
		formats.register(nsid, media_type);
		formats
			.media_type(&Nsid::new_static("com.example.unknown"))
			.unwrap_err()
			.to_string()
			.xpect_contains("com.example.unknown")
			.xpect_contains("com.example.content");
	}

	/// The member's `$type` is the declared NSID, and an object naming
	/// another def is refused.
	#[cfg(feature = "json")]
	#[beet_core::test]
	fn wraps_under_the_nsid() {
		let (nsid, media_type) = example();
		let mut formats = StandardSiteContentFormats::default();
		formats.register(nsid.clone(), media_type.clone());
		formats
			.member(
				&nsid,
				&MediaBytes::new_str(media_type.clone(), r#"{"pages":[]}"#),
			)
			.unwrap()
			.r#type()
			.xpect_eq(nsid.clone());
		formats
			.member(
				&nsid,
				&MediaBytes::new_str(
					media_type,
					r#"{"$type":"com.example.other"}"#,
				),
			)
			.unwrap_err()
			.to_string()
			.xpect_contains("declares `$type");
	}
}
