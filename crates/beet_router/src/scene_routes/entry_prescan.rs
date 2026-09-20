use beet_core::prelude::*;

/// Everything entry resolution reads out of a raw entry document, from one
/// registry-free walk.
///
/// The declarations here are needed *before* the entry can build: the store root
/// widens before the store exists, the template dirs register before the entry
/// parses so its own tags resolve, and the crate checks fire even when the tree
/// itself cannot build (its root tag feature-gated out of this binary). None of
/// that can go through the registry, so it is a plain pre-scan.
///
/// One parse per bootstrap, versus the three the same bytes used to get. The
/// [`RepoRoot`], [`TemplateDir`], [`RequireCfg`] and `Secrets` components are
/// untouched: they remain the authoring vocabulary, this is only the
/// extraction.
///
/// A subtree carrying a `bx:cfg` is NOT scanned, whatever its condition says.
/// This walk runs before the world that would answer the condition is reachable,
/// and the two possible mistakes are not symmetric: skipping a met branch only
/// defers work the build does anyway (a template dir registers via its observer
/// instead, an include is one the watcher picks up on rebuild), while scanning an
/// EXCLUDED branch would fire a `<RequireCfg>` this build was never meant to
/// satisfy and fail the load over a requirement that does not apply. So the
/// conditional case is left to the build, which is the stage that can answer it.
/// Entry-level declarations belong at the entry's top level regardless.
///
/// What the entry *instantiates* is not read here: the build resolves every tag
/// and marks each registry template it expands
/// ([`TemplateInstance`](beet_core::prelude::TemplateInstance)), which is where
/// the watch path reads the structural templates from.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct EntryPrescan {
	/// The `<RepoRoot src>` the entry widens its store to, relative to the entry
	/// document's directory.
	pub repo_root: Option<SmolStr>,
	/// Every `<TemplateDir src>` the entry declares, in document order.
	pub template_dirs: Vec<RelPath>,
	/// Every unconditional `<RequireCfg/>` the entry declares.
	pub requirements: Vec<RequireCfg>,
	/// Every local `<Template src>` include. Remote includes are skipped: they are
	/// not local files a watcher sees.
	pub includes: Vec<RelPath>,
	/// Every unconditional `<Secrets>` declaration in the repo store (one
	/// carrying a spread names another store, ie `{StoreRef($cold)}`, and is
	/// an export target the verbs read, never loaded into an environment).
	/// Entry resolution loads each into the process environment before the
	/// entry builds, so a declaration constructed in the build walk finds its
	/// credentials set.
	#[cfg(feature = "vault")]
	pub secrets: Vec<Secrets>,
}

impl EntryPrescan {
	/// Pre-scan an entry document. A non-markup (serde) entry declares none of
	/// these, so it yields the default; a markup entry that fails to parse is an
	/// error, since every later stage would fail on it too.
	pub fn parse(entry: &MediaBytes) -> Result<Self> {
		if !matches!(entry.media_type(), MediaType::Bsx | MediaType::Html) {
			return Ok(Self::default());
		}
		let nodes =
			BsxNode::parse_document(entry.as_utf8()?, &BsxParseConfig::bsx())?;
		let mut prescan = Self::default();
		prescan.collect(&nodes);
		prescan.xok()
	}

	/// Pre-scan an entry document, yielding the default rather than erroring on
	/// unreadable or unparseable content. The watch path's transitive include
	/// walk uses it, so a broken include never blocks watch startup.
	pub fn parse_lossy(entry: &MediaBytes) -> Self {
		Self::parse(entry).unwrap_or_default()
	}

	/// Recursively collect every declaration from `nodes`.
	fn collect(&mut self, nodes: &[BsxNode]) {
		for node in nodes {
			let BsxNode::Element(element) = node else {
				continue;
			};
			// a conditional subtree is the build's to answer, see the type docs
			if element.attributes.iter().any(|attr| attr.key == "bx:cfg") {
				continue;
			}
			match element.tag.as_str() {
				// the first `<RepoRoot>` wins; a second is ignored rather than
				// silently re-rooting the store mid-document.
				"RepoRoot" => {
					if self.repo_root.is_none() {
						self.repo_root = Self::str_attr(element, "src");
					}
				}
				"TemplateDir" => {
					self.template_dirs.extend(
						Self::str_attr(element, "src").map(RelPath::new),
					);
				}
				"RequireCfg" => {
					self.requirements.extend(
						Self::str_attr(element, "cfg").map(RequireCfg::new),
					);
				}
				"Template" => {
					self.includes.extend(
						Self::str_attr(element, "src")
							.filter(|src| !Self::is_remote(src))
							.map(RelPath::new),
					);
				}
				#[cfg(feature = "vault")]
				"Secrets" if !Self::has_spread(element) => {
					let mut secrets = Secrets::default();
					if let Some(label) = Self::str_attr(element, "label") {
						secrets.label = label;
					}
					if let Some(path) = Self::str_attr(element, "path") {
						secrets.path = SmolPath::new(path);
					}
					self.secrets.push(secrets);
				}
				_ => {}
			}
			self.collect(&element.children);
		}
	}

	/// The value of a string attribute, if present. A non-literal (block) value
	/// is skipped: it cannot resolve without the registry this scan runs before.
	fn str_attr(element: &BsxElement, key: &str) -> Option<SmolStr> {
		element.attributes.iter().find_map(|attr| {
			match (attr.key == key, &attr.value) {
				(true, AttrValue::Str(value)) => {
					Some(SmolStr::from(value.as_str()))
				}
				_ => None,
			}
		})
	}

	/// Whether the element carries a bare-position spread (`<el {..}>`).
	#[cfg(feature = "vault")]
	fn has_spread(element: &BsxElement) -> bool {
		element
			.attributes
			.iter()
			.any(|attr| matches!(attr.value, AttrValue::Spread(_)))
	}

	/// Whether `src` names a remote endpoint rather than a local path.
	fn is_remote(src: &str) -> bool {
		src.starts_with("http://")
			|| src.starts_with("https://")
			|| src.starts_with("s3://")
	}
}

#[cfg(test)]
mod test {
	use super::*;

	/// One walk reads every declaration kind, at any depth, from one document.
	#[beet_core::test]
	fn parses_every_declaration() {
		let prescan = EntryPrescan::parse(&MediaBytes::new_bsx(
			r#"<Router>
				<RepoRoot src="../.."/>
				<TemplateDir src="templates"/>
				<RequireCfg cfg="feature:sockets && version:0.1.0"/>
				<Template src="header.bsx"/>
				<Template src="https://example.org/remote.bsx"/>
				<div>
					<TemplateDir src="more"/>
					<RequireCfg cfg="feature:ssh"/>
					<Template src="footer.bsx"/>
				</div>
			</Router>"#,
		))
		.unwrap();
		prescan.repo_root.xpect_eq(Some(SmolStr::from("../..")));
		prescan
			.template_dirs
			.xpect_eq(vec![RelPath::from("templates"), RelPath::from("more")]);
		prescan.requirements.xpect_eq(vec![
			RequireCfg::new("feature:sockets && version:0.1.0"),
			RequireCfg::new("feature:ssh"),
		]);
		// the remote include is skipped: it is not a local file a watcher sees
		prescan.includes.xpect_eq(vec![
			RelPath::from("header.bsx"),
			RelPath::from("footer.bsx"),
		]);
	}

	/// A `bx:cfg` subtree declares nothing: the build answers the condition.
	#[beet_core::test]
	fn conditional_subtree_declares_nothing() {
		EntryPrescan::parse(&MediaBytes::new_bsx(
			r#"<Router>
				<Fragment bx:cfg="feature:infra">
					<TemplateDir src="templates"/>
					<Template src="header.bsx"/>
				</Fragment>
			</Router>"#,
		))
		.unwrap()
		.xpect_eq(EntryPrescan::default());
	}

	/// The first `<RepoRoot>` wins, and a document declaring nothing yields the
	/// default.
	#[beet_core::test]
	fn first_repo_root_wins() {
		EntryPrescan::parse(&MediaBytes::new_bsx(
			r#"<Router><RepoRoot src="../.."/><RepoRoot src="nope"/></Router>"#,
		))
		.unwrap()
		.repo_root
		.xpect_eq(Some(SmolStr::from("../..")));
		EntryPrescan::parse(&MediaBytes::new_bsx("<Router/>"))
			.unwrap()
			.xpect_eq(EntryPrescan::default());
	}

	/// The entry's own documents are read off the top level; one in another
	/// store (a spread naming it) and one under a `bx:cfg` are left to the
	/// build.
	#[cfg(feature = "vault")]
	#[beet_core::test]
	fn collects_the_entry_documents() {
		let prescan = EntryPrescan::parse(&MediaBytes::new_bsx(
			r#"<Router>
				<Secrets/>
				<Secrets label="mail-prod" path="infra/secrets/mail--prod.toml"/>
				<Secrets label="mail-cold" path="secrets/export.toml" {StoreRef($cold)}/>
				<Secrets label="gated" bx:cfg="feature:vault"/>
			</Router>"#,
		))
		.unwrap();
		prescan.secrets.xpect_eq(vec![
			Secrets::default(),
			Secrets::new("infra/secrets/mail--prod.toml")
				.with_label("mail-prod"),
		]);
	}

	/// A serde entry declares none of these, so it pre-scans to the default
	/// rather than erroring on non-markup bytes.
	#[beet_core::test]
	fn non_markup_yields_default() {
		EntryPrescan::parse(&MediaBytes::new(MediaType::Json, b"{}".to_vec()))
			.unwrap()
			.xpect_eq(EntryPrescan::default());
	}
}
