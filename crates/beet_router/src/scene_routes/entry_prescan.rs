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
/// [`StoreRoot`], [`TemplateDir`] and [`CrateCheck`] components are untouched:
/// they remain the authoring vocabulary, this is only the extraction.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct EntryPrescan {
	/// The `<StoreRoot src>` the entry widens its store to, relative to the entry
	/// document's directory.
	pub store_root: Option<SmolStr>,
	/// Every `<TemplateDir src>` the entry declares, in document order.
	pub template_dirs: Vec<SmolStr>,
	/// Every `<CrateCheck/>` the entry declares.
	pub checks: Vec<CrateCheck>,
	/// Every local `<Template src>` include. Remote includes are skipped: they are
	/// not local files a watcher sees.
	pub includes: Vec<SmolStr>,
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
			match element.tag.as_str() {
				// the first `<StoreRoot>` wins; a second is ignored rather than
				// silently re-rooting the store mid-document.
				"StoreRoot" => {
					if self.store_root.is_none() {
						self.store_root = Self::str_attr(element, "src");
					}
				}
				"TemplateDir" => {
					self.template_dirs.extend(Self::str_attr(element, "src"));
				}
				"CrateCheck" => self.checks.push(CrateCheck {
					features: Self::list_attr(element, "features"),
					versions: Self::list_attr(element, "versions"),
				}),
				"Template" => {
					self.includes.extend(
						Self::str_attr(element, "src")
							.filter(|src| !Self::is_remote(src)),
					);
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

	/// The string items of a list attribute, empty if absent. Reads the literal
	/// directly, since the reflect path that would coerce it runs after this
	/// scan; a non-string item is skipped rather than stringified, keeping the
	/// two readings of the same markup in step.
	fn list_attr(element: &BsxElement, key: &str) -> Vec<SmolStr> {
		element
			.attributes
			.iter()
			.find(|attr| attr.key == key)
			.and_then(|attr| match &attr.value {
				AttrValue::Expr(ValueExpr::Literal(DataLiteral::List(
					items,
				))) => Some(items),
				_ => None,
			})
			.map(|items| {
				items
					.iter()
					.filter_map(|item| match item {
						DataLiteral::Scalar(Value::Str(value)) => {
							Some(value.clone())
						}
						_ => None,
					})
					.collect()
			})
			.unwrap_or_default()
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
				<StoreRoot src="../.."/>
				<TemplateDir src="templates"/>
				<CrateCheck features={["sockets"]} versions={["0.1.0"]}/>
				<Template src="header.bsx"/>
				<Template src="https://example.org/remote.bsx"/>
				<div>
					<TemplateDir src="more"/>
					<CrateCheck features={["ssh"]}/>
					<Template src="footer.bsx"/>
				</div>
			</Router>"#,
		))
		.unwrap();
		prescan.store_root.xpect_eq(Some(SmolStr::from("../..")));
		prescan
			.template_dirs
			.xpect_eq(vec![SmolStr::from("templates"), SmolStr::from("more")]);
		prescan.checks.len().xpect_eq(2);
		prescan.checks[0]
			.features
			.xpect_eq(vec![SmolStr::from("sockets")]);
		prescan.checks[0]
			.versions
			.xpect_eq(vec![SmolStr::from("0.1.0")]);
		prescan.checks[1]
			.features
			.xpect_eq(vec![SmolStr::from("ssh")]);
		// the remote include is skipped: it is not a local file a watcher sees
		prescan.includes.xpect_eq(vec![
			SmolStr::from("header.bsx"),
			SmolStr::from("footer.bsx"),
		]);
	}

	/// The first `<StoreRoot>` wins, and a document declaring nothing yields the
	/// default.
	#[beet_core::test]
	fn first_store_root_wins() {
		EntryPrescan::parse(&MediaBytes::new_bsx(
			r#"<Router><StoreRoot src="../.."/><StoreRoot src="nope"/></Router>"#,
		))
		.unwrap()
		.store_root
		.xpect_eq(Some(SmolStr::from("../..")));
		EntryPrescan::parse(&MediaBytes::new_bsx("<Router/>"))
			.unwrap()
			.xpect_eq(EntryPrescan::default());
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
