//! Post-build validation: the "type-checked feeling" for a no-code site.
//!
//! A compiler never sees a markup site, so three classes of mistake that Rust
//! would catch slip through to a blank page or a dead link. [`render_diagnostics`]
//! recovers them by walking a built render tree against the live route set and
//! [`RuleSet`], emitting a [`Diagnostic`] for each:
//!
//! - an **unknown uppercase tag** (`<Foo/>`) resolving to no component, resource,
//!   template or tag-resolver — surfaced as the [`TemplateError`] the build rides,
//!   or as a literal uppercase [`Element`] that slipped through;
//! - a **broken internal href** (`href="/.."`) with no matching route, the
//!   load-bearing replacement for codegen'd typed-route checking;
//! - an **unknown class** with no matching selector in the [`RuleSet`].
//!
//! Each check's severity is configurable through the [`RenderDiagnostics`]
//! resource (a site patches it like `PackageConfig`). The error-level diagnostics
//! gate `export-static` and `beet check`.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_ui::prelude::*;

/// The severity of a render [`Diagnostic`], also the per-check configuration in
/// [`RenderDiagnostics`]: a check set to [`Off`](Self::Off) emits nothing.
#[derive(
	Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Reflect, PartialOrd, Ord,
)]
pub enum DiagnosticSeverity {
	/// The check is disabled and emits no diagnostic.
	Off,
	/// A warning: logged, but does not fail a gated entry point.
	Warn,
	/// An error: logged and fails `export-static`/`beet check` with a non-zero exit.
	#[default]
	Error,
}

/// Which render check produced a [`Diagnostic`], so a consumer can group or
/// re-route by kind independent of the configured [`DiagnosticSeverity`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub enum DiagnosticKind {
	/// An uppercase tag resolving to no component/resource/template/tag-resolver,
	/// incl a build [`TemplateError`].
	UnknownTag,
	/// An internal `href="/.."` with no matching route in the route tree.
	BrokenHref,
	/// A `class`/[`Classes`] token with no matching selector in the [`RuleSet`].
	UnknownClass,
}

/// Configures the [`DiagnosticSeverity`] of each render check, overridable by a site like
/// [`PackageConfig`](beet_net::prelude::PackageConfig) (it is a reflected
/// resource, patchable from markup).
///
/// Defaults recover the typed-build guarantees: an unknown tag and a broken
/// internal href are hard [`Error`](DiagnosticSeverity::Error)s, while an unknown class is
/// a [`Warn`](DiagnosticSeverity::Warn) (a one-off marker class with no rule is sometimes
/// intentional; classes carry no rule-existence guarantee even in Rust).
#[derive(Debug, Clone, Resource, Reflect)]
#[reflect(Resource, Default)]
pub struct RenderDiagnostics {
	/// An uppercase tag resolving to nothing. Default [`Error`](DiagnosticSeverity::Error).
	pub unknown_tag: DiagnosticSeverity,
	/// An internal href with no matching route. Default [`Error`](DiagnosticSeverity::Error).
	pub broken_href: DiagnosticSeverity,
	/// A class with no matching selector. Default [`Warn`](DiagnosticSeverity::Warn).
	pub unknown_class: DiagnosticSeverity,
}

impl Default for RenderDiagnostics {
	fn default() -> Self {
		Self {
			unknown_tag: DiagnosticSeverity::Error,
			broken_href: DiagnosticSeverity::Error,
			unknown_class: DiagnosticSeverity::Warn,
		}
	}
}

impl RenderDiagnostics {
	/// The configured [`DiagnosticSeverity`] for a [`DiagnosticKind`].
	pub fn severity(&self, kind: DiagnosticKind) -> DiagnosticSeverity {
		match kind {
			DiagnosticKind::UnknownTag => self.unknown_tag,
			DiagnosticKind::BrokenHref => self.broken_href,
			DiagnosticKind::UnknownClass => self.unknown_class,
		}
	}
}

/// A single problem found by [`render_diagnostics`]: its [`DiagnosticKind`], the
/// resolved [`DiagnosticSeverity`], a human message, and the originating route path when
/// known (the entry-point drivers set it per route).
#[derive(Debug, Clone, Get)]
pub struct Diagnostic {
	/// The check that produced this diagnostic.
	pub kind: DiagnosticKind,
	/// The resolved severity (always [`Warn`](DiagnosticSeverity::Warn) or
	/// [`Error`](DiagnosticSeverity::Error); an [`Off`](DiagnosticSeverity::Off) check emits nothing).
	pub severity: DiagnosticSeverity,
	/// A concise, author-facing description of the problem.
	pub message: String,
	/// The route path this was found on, set by the per-route drivers.
	pub route: Option<SmolPath>,
}

impl Diagnostic {
	/// A diagnostic with no route context (the drivers attach it via
	/// [`Self::with_route`]).
	pub fn new(kind: DiagnosticKind, severity: DiagnosticSeverity, message: String) -> Self {
		Self {
			kind,
			severity,
			message,
			route: None,
		}
	}

	/// Tag this diagnostic with the route it was found on.
	pub fn with_route(mut self, route: SmolPath) -> Self {
		self.route = Some(route);
		self
	}

	/// Whether this is an [`Error`](DiagnosticSeverity::Error)-level diagnostic, ie one that
	/// gates `export-static`/`beet check`.
	pub fn is_error(&self) -> bool { self.severity == DiagnosticSeverity::Error }

	/// Log this diagnostic through the cross-platform `log` facade at the level
	/// matching its [`DiagnosticSeverity`].
	pub fn log(&self) {
		let route = self
			.route
			.as_ref()
			.map(|path| format!(" [{}]", path.with_leading_slash()))
			.unwrap_or_default();
		match self.severity {
			DiagnosticSeverity::Error => error!("render-diagnostic{route}: {}", self.message),
			DiagnosticSeverity::Warn => warn!("render-diagnostic{route}: {}", self.message),
			DiagnosticSeverity::Off => {}
		}
	}
}

/// Run the three render checks over the built tree rooted at `root`, returning a
/// [`Diagnostic`] per problem whose check is not [`Off`](DiagnosticSeverity::Off).
///
/// Walks the [`Element`]/[`Attribute`]/[`Classes`] tree with [`DiagnosticsQuery`]
/// (a [`SystemParam`], not ad-hoc world poking), validating internal hrefs
/// against `route_tree` and classes against `rule_set`. Any [`TemplateError`] on
/// the tree (eg the build's unresolved-tag bail) folds in as an
/// [`UnknownTag`](DiagnosticKind::UnknownTag) error, so a no-code author sees the
/// build failure loudly rather than as a blank page.
pub fn render_diagnostics(
	world: &mut World,
	root: Entity,
	route_tree: &RouteTree,
	rule_set: &RuleSet,
	config: &RenderDiagnostics,
) -> Vec<Diagnostic> {
	world.with_state::<DiagnosticsQuery, _>(|query| {
		query.collect(root, route_tree, rule_set, config)
	})
}

/// The render-tree traversal backing [`render_diagnostics`]: every [`Element`]
/// (with its attributes/classes) plus any [`TemplateError`] reachable from a
/// render root.
#[derive(SystemParam)]
pub struct DiagnosticsQuery<'w, 's> {
	elements: ElementQuery<'w, 's>,
	errors: Query<'w, 's, (Entity, &'static TemplateError)>,
	children: Query<'w, 's, &'static Children>,
}

impl DiagnosticsQuery<'_, '_> {
	/// Collect every diagnostic under `root` (see [`render_diagnostics`]).
	pub fn collect(
		&self,
		root: Entity,
		route_tree: &RouteTree,
		rule_set: &RuleSet,
		config: &RenderDiagnostics,
	) -> Vec<Diagnostic> {
		let mut out = Vec::new();
		// a build failure (eg the unresolved-uppercase-tag bail) rides
		// `TemplateError`; fold every one reachable from the root in as an
		// unknown-tag error so it is never silently dropped.
		self.collect_template_errors(root, config, &mut out);
		// then the per-element href/class/literal-uppercase-tag checks.
		for el in self.elements.iter_descendants_inclusive(root) {
			self.check_element(&el, route_tree, rule_set, config, &mut out);
		}
		out
	}

	/// Fold every [`TemplateError`] in `root`'s subtree into an
	/// [`UnknownTag`](DiagnosticKind::UnknownTag) error, the surfacing path for a
	/// build that bailed (eg on an unresolved tag).
	fn collect_template_errors(
		&self,
		root: Entity,
		config: &RenderDiagnostics,
		out: &mut Vec<Diagnostic>,
	) {
		let severity = config.severity(DiagnosticKind::UnknownTag);
		if severity == DiagnosticSeverity::Off {
			return;
		}
		let mut stack = vec![root];
		while let Some(entity) = stack.pop() {
			if let Ok((_, error)) = self.errors.get(entity) {
				out.push(Diagnostic::new(
					DiagnosticKind::UnknownTag,
					severity,
					format!("template build error: {}", error.error),
				));
			}
			if let Ok(children) = self.children.get(entity) {
				stack.extend(children.iter());
			}
		}
	}

	/// Run the per-element checks (literal uppercase tag, internal href, classes).
	fn check_element(
		&self,
		el: &ElementView,
		route_tree: &RouteTree,
		rule_set: &RuleSet,
		config: &RenderDiagnostics,
		out: &mut Vec<Diagnostic>,
	) {
		// an uppercase tag that survived as a literal element resolved to nothing.
		let tag_severity = config.severity(DiagnosticKind::UnknownTag);
		if tag_severity != DiagnosticSeverity::Off && is_uppercase_tag(el.tag()) {
			out.push(Diagnostic::new(
				DiagnosticKind::UnknownTag,
				tag_severity,
				format!(
					"unknown tag `<{}>`: no component, resource, template or tag-resolver registered",
					el.tag()
				),
			));
		}

		// an internal href (`/..`) must resolve to a route; external/anchor hrefs
		// are skipped.
		let href_severity = config.severity(DiagnosticKind::BrokenHref);
		if href_severity != DiagnosticSeverity::Off
			&& let Some(href) = el.attribute("href").and_then(|attr| attr.value.as_str().ok())
			&& let Some(segments) = internal_href_segments(href)
			&& route_tree.find(&segments).is_none()
		{
			out.push(Diagnostic::new(
				DiagnosticKind::BrokenHref,
				href_severity,
				format!("broken internal link `href=\"{href}\"`: no matching route"),
			));
		}

		// each class token must match a selector in the live rule set, except the
		// framework-emitted syntax-highlight classes: the `hl-<capture>` token spans
		// and the code-fence language hint on a `<code>` element (eg `rust`, `sh`).
		// The highlighter emits them; they may legitimately have no style rule
		// (a token falls back to the default text colour), and would otherwise warn
		// on every code block of every page.
		let class_severity = config.severity(DiagnosticKind::UnknownClass);
		if class_severity != DiagnosticSeverity::Off {
			let in_code = el.tag() == "code";
			for class in el.iter_classes() {
				if in_code || class.starts_with("hl-") {
					continue;
				}
				if !rule_set_has_class(rule_set, &class) {
					out.push(Diagnostic::new(
						DiagnosticKind::UnknownClass,
						class_severity,
						format!("unknown class `{class}`: no matching style rule"),
					));
				}
			}
		}
	}
}

/// Whether a tag resolves by name (a component/resource/template) rather than as
/// an HTML element: a capitalized tag, or a `path::to::X` whose final segment is
/// capitalized. Mirrors the BSX resolver's `is_uppercase_tag`.
fn is_uppercase_tag(tag: &str) -> bool {
	tag.rsplit("::")
		.next()
		.unwrap_or(tag)
		.starts_with(|ch: char| ch.is_uppercase())
}

/// The `/`-split segments of an *internal* href worth route-checking, or `None`
/// for an href to skip: an external scheme (`http(s)://`, `mailto:`, `tel:`, ..),
/// a fragment (`#anchor`), a protocol-relative (`//host`) or a non-rooted
/// relative href. A bare `/` yields the empty-segment root.
fn internal_href_segments(href: &str) -> Option<Vec<SmolStr>> {
	let href = href.trim();
	// only rooted, non-protocol-relative hrefs are internal links.
	if !href.starts_with('/') || href.starts_with("//") {
		return None;
	}
	// drop any query/fragment before matching the path.
	let path = href
		.split(['?', '#'])
		.next()
		.unwrap_or(href)
		.trim_start_matches('/')
		.trim_end_matches('/');
	if path.is_empty() {
		return Some(Vec::new());
	}
	path.split('/').map(SmolStr::from).collect::<Vec<_>>().xmap(Some)
}

/// Whether the [`RuleSet`] carries any selector matching the class `name`,
/// scanning every rule (incl the `:root` default) for a [`Class`](Selector::Class)
/// branch naming it.
fn rule_set_has_class(rule_set: &RuleSet, name: &str) -> bool {
	rule_set_classes(rule_set).any(|class| class == name)
}

/// Every class name a selector in the [`RuleSet`] references (incl the `:root`
/// default), the class vocabulary a `class="..."` autocomplete would offer.
///
/// The single source of truth shared by the unknown-class check
/// ([`rule_set_has_class`]) and the [`DiagnosticsManifest`](super::DiagnosticsManifest)
/// class catalog: both walk the same selectors, so the manifest can never list a
/// class the check rejects (or vice versa). Names repeat across rules; the caller
/// de-duplicates.
pub fn rule_set_classes(rule_set: &RuleSet) -> impl Iterator<Item = &str> {
	rule_set
		.iter()
		.flat_map(|rule| selector_classes(rule.selector()))
}

/// Every [`Class`](Selector::Class) name within a [`Selector`], recursing through
/// the `AnyOf`/`AllOf`/`Not`/`Descendant` combinators.
fn selector_classes(selector: &Selector) -> Vec<&str> {
	match selector {
		Selector::Class(class) => vec![class.as_str()],
		Selector::AnyOf(parts) | Selector::AllOf(parts) => {
			parts.iter().flat_map(|part| selector_classes(part)).collect()
		}
		Selector::Not(inner) => selector_classes(inner),
		Selector::Descendant {
			ancestor,
			descendant,
		} => selector_classes(ancestor)
			.into_iter()
			.chain(selector_classes(descendant))
			.collect(),
		_ => Vec::new(),
	}
}


#[cfg(test)]
mod test {
	use super::*;

	/// A world with the substrate + a `RuleSet` carrying a single `.page` rule,
	/// and a route tree exposing `/` and `/about`.
	fn diagnostics_world() -> World {
		(TemplatePlugin, DocumentPlugin).into_world()
	}

	/// A `RuleSet` with one `.page` class rule, the "known class" fixture.
	fn rule_set() -> RuleSet {
		RuleSet::default().with_rule(Rule::class("page"))
	}

	/// A route tree exposing `/` (home) and `/about`, the "known route" fixture.
	fn route_tree() -> RouteTree {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let root = world
			.spawn((Router, children![
				render_action::fixed_func_route("", || rsx! { <p>"home"</p> }),
				render_action::fixed_func_route("about", || rsx! { <p>"about"</p> }),
			]))
			.flush();
		world.entity(root).get::<RouteTree>().unwrap().clone()
	}

	/// Build `bundle` into a fresh render root and run the pass with the given
	/// `config`, returning the diagnostics.
	fn run(
		bundle: impl Bundle,
		config: &RenderDiagnostics,
	) -> Vec<Diagnostic> {
		let mut world = diagnostics_world();
		let tree = route_tree();
		let rules = rule_set();
		let root = world.spawn(bundle).flush();
		render_diagnostics(&mut world, root, &tree, &rules, config)
	}

	/// The diagnostics of a given [`DiagnosticKind`].
	fn of_kind(
		diagnostics: &[Diagnostic],
		kind: DiagnosticKind,
	) -> Vec<&Diagnostic> {
		diagnostics.iter().filter(|d| d.kind == kind).collect()
	}

	#[beet_core::test]
	fn unknown_tag_errors() {
		// an unregistered uppercase tag fails the BSX build (tags resolve at build
		// time, so this is a runtime parse, not a compile-time `rsx!`), riding
		// `TemplateError` onto the parse target — exactly the no-code failure mode.
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		let bytes = MediaBytes::new_bsx("<Nonexistent/>");
		let root = world.spawn_empty().id();
		world
			.entity_mut(root)
			.xtap(|entity| {
				// the build failure rides `TemplateError`, never a panic/return.
				MediaParser::new()
					.parse(ParseContext::new(entity, &bytes))
					.ok();
			})
			.flush();
		let tree = route_tree();
		let rules = rule_set();
		let diagnostics = render_diagnostics(
			&mut world,
			root,
			&tree,
			&rules,
			&RenderDiagnostics::default(),
		);
		let tag = of_kind(&diagnostics, DiagnosticKind::UnknownTag);
		tag.is_empty().xpect_false();
		tag.iter().all(|d| d.is_error()).xpect_true();
	}

	#[beet_core::test]
	fn broken_href_errors() {
		let diagnostics = run(
			rsx! { <a href="/nope">"x"</a> },
			&RenderDiagnostics::default(),
		);
		let href = of_kind(&diagnostics, DiagnosticKind::BrokenHref);
		href.len().xpect_eq(1);
		href[0].is_error().xpect_true();
	}

	#[beet_core::test]
	fn valid_href_passes() {
		// `/about` is a real route, so no broken-href diagnostic.
		run(
			rsx! { <a href="/about">"about"</a> },
			&RenderDiagnostics::default(),
		)
		.xmap(|d| of_kind(&d, DiagnosticKind::BrokenHref).len())
		.xpect_eq(0);
		// the root `/` resolves too.
		run(rsx! { <a href="/">"home"</a> }, &RenderDiagnostics::default())
			.xmap(|d| of_kind(&d, DiagnosticKind::BrokenHref).len())
			.xpect_eq(0);
	}

	#[beet_core::test]
	fn external_href_skipped() {
		// an external scheme, a fragment and a mailto are never route-checked.
		for href in ["https://example.com/nope", "#anchor", "mailto:a@b.c"] {
			run(
				rsx! { <a href=href>"x"</a> },
				&RenderDiagnostics::default(),
			)
			.xmap(|d| of_kind(&d, DiagnosticKind::BrokenHref).len())
			.xpect_eq(0);
		}
	}

	#[beet_core::test]
	fn unknown_class_warns() {
		let diagnostics = run(
			rsx! { <div class="not-a-real-class"/> },
			&RenderDiagnostics::default(),
		);
		let class = of_kind(&diagnostics, DiagnosticKind::UnknownClass);
		class.len().xpect_eq(1);
		// unknown-class defaults to a warning, not an error.
		(class[0].severity == DiagnosticSeverity::Warn).xpect_true();
	}

	#[beet_core::test]
	fn known_class_passes() {
		run(
			rsx! { <div class="page"/> },
			&RenderDiagnostics::default(),
		)
		.xmap(|d| of_kind(&d, DiagnosticKind::UnknownClass).len())
		.xpect_eq(0);
	}

	#[beet_core::test]
	fn syntax_highlight_classes_skipped() {
		// the code-fence language hint (`<code class="rust">`) and the `hl-<capture>`
		// token spans are framework-emitted by the syntax highlighter and may have no
		// style rule; they must NOT warn, else every code-heavy page floods the
		// console (and the tui).
		let diagnostics = run(
			rsx! {
				<pre><code class="rust">
					<span class="hl-function.macro">"vec!"</span>
				</code></pre>
			},
			&RenderDiagnostics::default(),
		);
		of_kind(&diagnostics, DiagnosticKind::UnknownClass)
			.len()
			.xpect_eq(0);
	}

	#[beet_core::test]
	fn clean_tree_silent() {
		let diagnostics = run(
			rsx! { <div class="page"><a href="/about">"ok"</a></div> },
			&RenderDiagnostics::default(),
		);
		diagnostics.is_empty().xpect_true();
	}

	#[beet_core::test]
	fn unknown_class_override_to_error() {
		let config = RenderDiagnostics {
			unknown_class: DiagnosticSeverity::Error,
			..default()
		};
		let class = run(rsx! { <div class="zzz"/> }, &config)
			.xmap(|d| of_kind(&d, DiagnosticKind::UnknownClass)
				.into_iter()
				.cloned()
				.collect::<Vec<_>>());
		class.len().xpect_eq(1);
		class[0].is_error().xpect_true();
	}

	#[beet_core::test]
	fn unknown_class_off_suppresses() {
		let config = RenderDiagnostics {
			unknown_class: DiagnosticSeverity::Off,
			..default()
		};
		run(rsx! { <div class="zzz"/> }, &config)
			.xmap(|d| of_kind(&d, DiagnosticKind::UnknownClass).len())
			.xpect_eq(0);
	}

	#[beet_core::test]
	fn template_error_surfaces_as_error() {
		// a `TemplateError` placed directly on a built root surfaces loudly.
		let mut world = diagnostics_world();
		let tree = route_tree();
		let rules = rule_set();
		let root = world
			.spawn((
				Element::new("div"),
				TemplateError::new(bevyhow!("boom")),
			))
			.flush();
		let diagnostics = render_diagnostics(
			&mut world,
			root,
			&tree,
			&rules,
			&RenderDiagnostics::default(),
		);
		let tag = of_kind(&diagnostics, DiagnosticKind::UnknownTag);
		tag.len().xpect_eq(1);
		tag[0].is_error().xpect_true();
		tag[0].message.as_str().xpect_contains("boom");
	}
}
