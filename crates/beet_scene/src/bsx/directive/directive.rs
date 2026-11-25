//! [`BsxDirective`] and the [`BsxDirectives`] set the rewrite pass reads.
use crate::prelude::*;
use alloc::borrow::Cow;
use alloc::sync::Arc;
use beet_core::prelude::*;

/// A `bx:` directive: sugar for explicit BSX an author could have written by
/// hand. It matches attribute keys by predicate and rewrites the element
/// carrying one, its key, spreads, attributes and children, touching no
/// world, so a directive runs as a document lowers and the body never learns
/// it existed.
///
/// What needs no world is a directive; what needs one is a template, run when
/// the body spawns. Registered by installing it, `app.install(MyDirective)`;
/// its [`Installer`] impl, a supertrait so no directive skips it, forwards to
/// [`BsxDirectives::register_on`]. The set clones the registered instance for
/// every rewrite, so its configuration is set once at registration.
pub trait BsxDirective: 'static + Send + Sync + Clone + Installer {
	/// The keys this directive matches as an author reads them, ie
	/// `bx:on<event>`, named in the error for a key nothing matches.
	fn pattern(&self) -> SmolStr;

	/// Whether this directive rewrites an attribute keyed `key`.
	fn matches(&self, key: &str) -> bool;

	/// Rewrite `element`, which carries an attribute keyed `key`, into
	/// explicit BSX, removing that attribute.
	fn rewrite(&self, key: &str, element: &mut BsxElement) -> Result;
}

/// Every [`BsxDirective`] a world's markup parsers read, a
/// [setting](DocParsers::setting) of [`DocParsers`] so the BSX parser and
/// markdown's embedded BSX read one set.
///
/// The standard set ([`Default`]) is in every registry whatever the
/// features, each rewrite naming its target type as text, so a lean binary
/// rewrites a directive whose type it lacks and holds that type as any
/// unregistered name. For a key two directives match the one registered last
/// wins, shadowing silently, so a downstream crate replaces a standard
/// directive by registering its own. A `bx:` key nothing matches refuses the
/// document, since a directive has no later chance to resolve.
#[derive(Clone)]
pub struct BsxDirectives {
	directives: Vec<Arc<dyn ErasedBsxDirective>>,
}

/// The standard set.
impl Default for BsxDirectives {
	fn default() -> Self {
		let mut directives = Self {
			directives: Vec::new(),
		};
		directives
			.register(ScopeDirective)
			.register(ForDirective)
			.register(OnEventDirective)
			.register(StyleDirective)
			.register(RunDirective)
			.register(EventDirective)
			.register(RefDirective)
			.register(CfgDirective);
		directives
	}
}

impl BsxDirectives {
	/// What every directive's key starts with.
	const PREFIX: &'static str = "bx:";

	/// Register `directive`, matching ahead of any registered before it.
	pub fn register(&mut self, directive: impl BsxDirective) -> &mut Self {
		self.directives.push(Arc::new(directive));
		self
	}

	/// The set `parsers` carries, the standard one until a directive is
	/// registered.
	pub fn of(parsers: &DocParsers) -> Cow<'_, Self> {
		match parsers.setting::<Self>() {
			Some(directives) => Cow::Borrowed(directives),
			None => Cow::Owned(Self::default()),
		}
	}

	/// [`register`](Self::register) `directive` in the set `app`'s
	/// [`DocParsers`] carries, creating the registry if absent and never
	/// touching a parser: the body of every [`BsxDirective`]'s [`Installer`]
	/// impl.
	pub fn register_on(app: &mut App, directive: impl BsxDirective) {
		app.world_mut()
			.get_resource_or_init::<DocParsers>()
			.edit_setting(|directives: &mut Self| {
				directives.register(directive);
			});
	}

	/// The pattern of every registered directive, in registration order.
	pub fn available(&self) -> Vec<SmolStr> {
		let mut available = Vec::new();
		for pattern in
			self.directives.iter().map(|directive| directive.pattern())
		{
			if !available.contains(&pattern) {
				available.push(pattern);
			}
		}
		available
	}

	/// Rewrite every directive in `nodes` into the explicit BSX it is sugar
	/// for, depth first, so what a rewrite adds is rewritten in turn.
	pub fn rewrite(&self, nodes: &mut [BsxNode]) -> Result {
		for node in nodes {
			if let BsxNode::Element(element) = node {
				self.rewrite_element(element)
					.map_err(|error| element.span.locate(error))?;
			}
		}
		Ok(())
	}

	fn rewrite_element(&self, element: &mut BsxElement) -> Result {
		while let Some(key) = element
			.attributes
			.iter()
			.map(|attr| attr.key.as_str())
			.find(|key| key.starts_with(Self::PREFIX))
			.map(String::from)
		{
			let directive = self
				.directives
				.iter()
				.rev()
				.find(|directive| directive.matches(&key))
				.ok_or_else(|| {
					bevyhow!(
						"`{key}` on `<{tag}>` is no directive this build knows: \
						 the directives are {available}",
						tag = element.tag,
						available = self
							.available()
							.iter()
							.map(|pattern| format!("`{pattern}`"))
							.collect::<Vec<_>>()
							.join(", ")
					)
				})?;
			directive.rewrite(&key, element)?;
			if element.attributes.iter().any(|attr| attr.key == key) {
				bevybail!(
					"the `{}` directive left `{key}` on `<{}>`",
					directive.pattern(),
					element.tag
				);
			}
		}
		element.place_children();
		self.rewrite(&mut element.children)
	}
}

/// The object-safe face of a [`BsxDirective`], which a set can hold.
trait ErasedBsxDirective: 'static + Send + Sync {
	fn pattern(&self) -> SmolStr;
	fn matches(&self, key: &str) -> bool;
	fn rewrite(&self, key: &str, element: &mut BsxElement) -> Result;
}

impl<T: BsxDirective> ErasedBsxDirective for T {
	fn pattern(&self) -> SmolStr { BsxDirective::pattern(self) }

	fn matches(&self, key: &str) -> bool { BsxDirective::matches(self, key) }

	fn rewrite(&self, key: &str, element: &mut BsxElement) -> Result {
		BsxDirective::rewrite(&self.clone(), key, element)
	}
}
