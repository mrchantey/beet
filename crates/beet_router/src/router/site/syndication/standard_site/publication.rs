//! `site.standard.publication`, the record describing a site that publishes
//! documents.
use crate::prelude::*;
use beet_core::prelude::*;
use beet_ui::prelude::*;

/// `site.standard.publication`: a site that publishes standard site
/// documents, its url, name and the look a reader renders it with. Keyed by
/// TID, minted on first write.
///
/// The `url` is the publication's natural key, and joined to a document's
/// `path` it forms that document's canonical page.
#[derive(Debug, Clone, PartialEq, Reflect, Serialize, Deserialize)]
#[reflect(Serialize, Deserialize)]
pub struct StandardSitePublication {
	/// The base url, no trailing slash, ie `https://beet.org/blog`.
	pub url: Uri,
	/// The publication's name.
	pub name: SmolStr,
	/// What the publication is about.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub description: Option<String>,
	/// A square image identifying the publication, at least 256 pixels a
	/// side and under 1MB. Without one Bluesky's card shows no avatar.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub icon: Option<BlobRef>,
	/// The colours a reader renders the publication's documents in.
	#[serde(
		default,
		rename = "basicTheme",
		skip_serializing_if = "Option::is_none"
	)]
	pub basic_theme: Option<ThemeBasic>,
	/// The publication's self labels, effectively content warnings.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub labels: Option<SelfLabels>,
	/// What a platform showing the publication should do with it.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub preferences: Option<PublicationPreferences>,
}

impl AtprotoRecord for StandardSitePublication {
	const COLLECTION: Nsid = Nsid::new_static("site.standard.publication");
}

impl StandardSitePublication {
	/// The publication `declaration` describes on the site `package` names,
	/// in the colours of `theme`, identified by `icon` once it is uploaded.
	///
	/// The url is [`PackageConfig::homepage`] joined with the declaration's
	/// path, its trailing slash trimmed as the lexicon asks.
	///
	/// # Errors
	/// Errors when the homepage is unset or names no origin (see
	/// [`PackageConfig::absolute_url`]), or when the theme does not resolve.
	pub fn from_declaration(
		declaration: &StandardSitePub,
		package: &PackageConfig,
		theme: &Theme,
		icon: Option<BlobRef>,
	) -> Result<Self> {
		Self {
			url: package
				.absolute_url(declaration.path.as_str())?
				.to_string()
				.trim_end_matches('/')
				.xmap(Uri::parse)?,
			name: declaration.name.clone(),
			description: declaration.description.clone(),
			icon,
			basic_theme: Some(ThemeBasic::from_theme(theme)?),
			labels: SelfLabels::from_values(declaration.labels.iter().cloned()),
			preferences: Some(PublicationPreferences {
				show_in_discover: declaration.show_in_discover,
			}),
		}
		.xok()
	}
}

/// `site.standard.publication#preferences`: what a platform showing the
/// publication should do with it.
#[derive(
	Debug, Clone, Copy, PartialEq, Eq, Reflect, Serialize, Deserialize,
)]
#[serde(default)]
#[reflect(Default, Serialize, Deserialize)]
pub struct PublicationPreferences {
	/// Whether the publication may appear in discovery feeds, true unless
	/// written otherwise.
	#[serde(rename = "showInDiscover")]
	pub show_in_discover: bool,
}

impl Default for PublicationPreferences {
	fn default() -> Self {
		Self {
			show_in_discover: true,
		}
	}
}

#[cfg(test)]
mod test {
	use super::super::test_fixtures::*;
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_ui::prelude::*;

	fn package(homepage: &str) -> PackageConfig {
		PackageConfig {
			homepage: Some(Url::coerce(homepage)),
			..default()
		}
	}

	/// Every field filled.
	fn filled() -> StandardSitePublication {
		StandardSitePublication::from_declaration(
			&StandardSitePub {
				labels: vec!["graphic-media".into()],
				..harvest()
			},
			&package("https://beet.org"),
			&Theme::default(),
			Some(BlobRef::of(ICON, MediaType::Png)),
		)
		.unwrap()
	}

	/// The bytes of the fixture's icon.
	const ICON: &[u8] = b"icon";

	#[beet_core::test]
	fn serializes_the_lexicon() { wire(&filled()).xpect_snapshot(); }

	#[beet_core::test]
	async fn converges() {
		converges_once(&[(ICON, MediaType::Png)], filled()).await;
	}

	/// The url carries no trailing slash, a publication at the site root
	/// included.
	#[beet_core::test]
	fn trims_the_url() {
		let url = |homepage: &str, path: &str| {
			StandardSitePublication::from_declaration(
				&StandardSitePub {
					path: RelPath::new(path),
					..harvest()
				},
				&package(homepage),
				&Theme::default(),
				None,
			)
			.unwrap()
			.url
			.to_string()
		};
		url("https://beet.org", "blog").xpect_eq("https://beet.org/blog");
		url("https://beet.org/", "blog/").xpect_eq("https://beet.org/blog");
		url("https://beet.org", "").xpect_eq("https://beet.org");
	}

	#[beet_core::test]
	fn requires_a_homepage() {
		StandardSitePublication::from_declaration(
			&harvest(),
			&PackageConfig::default(),
			&Theme::default(),
			None,
		)
		.unwrap_err()
		.to_string()
		.xpect_contains("homepage");
	}

	/// Leaflet's own publication as its PDS answered on 2026-10-06, the
	/// fields beet does not know (`theme`, the extra preferences) ignored.
	#[beet_core::test]
	fn reads_a_foreign_publication() {
		let json = r#"{"url":"https://lab.leaflet.pub","icon":{"ref":{"$link":"bafkreicchgde2juzzjey4opdbbh7h26mmi4c4pcbg7pc3muhmixb6mrm3u"},"size":3394,"$type":"blob","mimeType":"image/webp"},"name":"Leaflet Lab Notes","$type":"site.standard.publication","theme":{"$type":"pub.leaflet.publication#theme","primary":{"b":39,"g":39,"r":39,"$type":"pub.leaflet.theme.color#rgb"},"pageWidth":624},"basicTheme":{"$type":"site.standard.theme.basic","accent":{"b":0,"g":145,"r":116,"$type":"site.standard.theme.color#rgb"},"background":{"b":255,"g":255,"r":255,"$type":"site.standard.theme.color#rgb"},"foreground":{"b":0,"g":0,"r":0,"$type":"site.standard.theme.color#rgb"},"accentForeground":{"b":255,"g":255,"r":255,"$type":"site.standard.theme.color#rgb"}},"description":"Behind the scenes as we build Leaflet, an open social publishing tool for blogs, newsletters, and more!","preferences":{"showComments":true,"showInDiscover":true,"prevNextDirection":"rtl"}}"#;
		let publication =
			AtprotoValue::from_json(serde_json::from_str(json).unwrap())
				.unwrap()
				.into_serde::<StandardSitePublication>()
				.unwrap();
		publication.url.as_str().xpect_eq("https://lab.leaflet.pub");
		publication
			.icon
			.unwrap()
			.mime_type
			.xpect_eq(MediaType::Webp);
		publication.basic_theme.unwrap().accent.g.xpect_eq(145);
		publication
			.preferences
			.unwrap()
			.show_in_discover
			.xpect_true();
	}
}
