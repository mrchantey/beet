//! `bx:cfg`: the one mechanism for excluding a region of a document from a
//! build, and the condition grammar it and its two siblings share.
//!
//! This is the markup twin of Rust's `#[cfg]`, and the analogy is exact enough
//! to lean on: `rsx!` needs nothing here because it is compiled and real
//! `#[cfg]` already applies, while a `.bsx` document is parsed at runtime and
//! has no compiler to do it. Same concept, same operators, two front ends.
//!
//! ```html
//! <Route path="site" bx:cfg="feature:infra && feature:extra">
//!     <Template src="infra/site.bsx"/>
//! </Route>
//! ```
//!
//! # Why exclusion needs a mechanism at all
//!
//! Beet's default posture is that **structure is universal**: a document builds
//! whole in every binary, an unresolvable tag becomes an inert entity, and a
//! lean binary keeps the tree's shape with only its behavior missing. That is
//! the right default and it is not going anywhere, because an inert entity
//! costs nothing.
//!
//! It has exactly one boundary it cannot cross: **some nodes are effects, not
//! structure.** `<Template src>` reads a file. A node that DOES something while
//! the tree is being built has no inert form, so "build it anyway, just without
//! behavior" is not available to it. Either it runs or it does not exist.
//!
//! `bx:cfg` is how such a branch is kept out of a build that must not run it.
//! An excluded branch is removed from the syntax tree *before* the build walk,
//! so nothing downstream can observe it: no entity, no tag resolution, no
//! directive, no `bx:ref` pin, no include read. Not skipped, not inert, not
//! spawned-then-emptied. Absent.
//!
//! # What is left behind, and why
//!
//! Exclusion is not silence. An excluded element leaves a single [`CfgExcluded`]
//! tombstone in its place: the condition, the tag, and the element's own `path`
//! if it declared one. It has no children, no components and no attributes, so
//! it performs nothing, but it carries enough for a host to answer "why is this
//! not here?".
//!
//! `beet_router` reads it: a tombstone that recorded a path becomes a route
//! that exists and refuses, so a lean binary still LISTS `/site` in `--help`
//! and dispatching it reports the condition that excluded it rather than
//! "unknown route". That is what lets this one mechanism cover the case a
//! separate dispatch-time gate used to.
//!
//! The honest limit, which is a property of the problem and not of this design:
//! routes *below* an excluded node cannot be listed, because the include that
//! defines them never ran. `/site` is reported as excluded; `/site/deploy` is
//! simply not there. Nothing could do better without running the effect that
//! exclusion exists to prevent.
//!
//! # The condition grammar
//!
//! Rust's operators and Rust's precedence, because the whole point is that this
//! reads as `#[cfg]`:
//!
//! ```text
//! condition := any
//! any       := all ("||" all)*
//! all       := unary ("&&" unary)*
//! unary     := "!" unary | "(" any ")" | atom
//! atom      := namespace ":" argument
//! ```
//!
//! `!` binds tightest, then `&&`, then `||`, and parentheses group:
//!
//! ```html
//! <foo bx:cfg="(feature:infra && feature:extra && !feature:lean) || feature:all"/>
//! ```
//!
//! There are no bare atoms. Every atom names its namespace, so a condition
//! always says what KIND of fact it consults and a reader never has to guess.
//!
//! # Namespaces are a seam
//!
//! An atom's namespace resolves through [`BsxConditions`], the same
//! register-a-handler shape as [`BsxTagResolvers`]. Core registers the facts
//! every build has:
//!
//! - `feature:<name>` / `feature:<crate>/<name>` — a compiled cargo feature,
//!   checked against the spawned [`CrateRegistration`] set.
//! - `version:<x.y.z>` / `version:<crate>@<x.y.z>` — a minimum compiled version.
//! - `env:<KEY>` / `env:<KEY>=<value>` — an environment variable, set and
//!   non-empty, or equal to `value`.
//!
//! A downstream crate registers its own against whatever it knows (a stage, a
//! deploy target, a licence tier) without core learning the concept.
//!
//! An **unknown namespace is a hard error**, not a false. A gate that silently
//! excluded a subtree because its condition was misspelled is the single worst
//! failure this mechanism could have, so it is the one case that refuses to
//! degrade.
//!
//! # The third consumer
//!
//! The same condition, asserted rather than applied, is
//! [`RequireCfg`](crate::prelude::RequireCfg): `<RequireCfg("feature:infra")/>`
//! fails the whole load when false, for an entry that cannot function at all
//! without something. It reports through [`BuildCondition::explain`], which
//! re-walks a false condition WITHOUT short-circuiting so the error names every
//! atom that failed rather than the first.
//!
//! So: one grammar, one namespace registry, three consumers. Prune the branch
//! (`bx:cfg`), report the branch ([`CfgExcluded`]), or refuse the document
//! ([`RequireCfg`](crate::prelude::RequireCfg)).

use crate::prelude::*;
use alloc::boxed::Box;
use alloc::sync::Arc;

/// A parsed `bx:cfg` condition. See the [module docs](self) for the grammar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildCondition {
	/// A `namespace:argument` fact, resolved through [`BsxConditions`].
	Atom {
		/// The namespace, ie `feature`.
		namespace: SmolStr,
		/// Everything after the first `:`, passed to the handler verbatim.
		argument: SmolStr,
	},
	/// `!condition`
	Not(Box<BuildCondition>),
	/// `a && b && ..`, true when every member is.
	All(Vec<BuildCondition>),
	/// `a || b || ..`, true when any member is.
	Any(Vec<BuildCondition>),
}

impl BuildCondition {
	/// Parse a condition, ie `feature:infra && !env:CI`.
	pub fn parse(source: &str) -> Result<Self> {
		let tokens = tokenize(source)?;
		if tokens.is_empty() {
			bevybail!(
				"empty cfg condition: expected something like `feature:infra`"
			);
		}
		let mut parser = ConditionParser {
			tokens: &tokens,
			pos: 0,
		};
		let condition = parser.parse_any()?;
		if parser.pos != tokens.len() {
			bevybail!(
				"trailing input in cfg condition `{source}`: expected `&&`, `||` or the end"
			);
		}
		Ok(condition)
	}

	/// Evaluate against this build's facts, resolving each atom's namespace
	/// through `conditions`. Short-circuits, so a false `&&` member stops the
	/// walk; use [`explain`](Self::explain) when the reason matters.
	pub fn evaluate(
		&self,
		conditions: &BsxConditions,
		cx: &ConditionCx,
	) -> Result<bool> {
		match self {
			Self::Atom { .. } => self.evaluate_atom(conditions, cx),
			Self::Not(inner) => Ok(!inner.evaluate(conditions, cx)?),
			Self::All(members) => {
				for member in members {
					if !member.evaluate(conditions, cx)? {
						return Ok(false);
					}
				}
				Ok(true)
			}
			Self::Any(members) => {
				for member in members {
					if member.evaluate(conditions, cx)? {
						return Ok(true);
					}
				}
				Ok(false)
			}
		}
	}

	/// Every atom that is false, as `namespace:argument` strings, walked WITHOUT
	/// short-circuiting.
	///
	/// The counterpart to [`evaluate`](Self::evaluate) for the case where a
	/// false condition is an error the user must act on: a load that fails
	/// should name every missing feature at once, not the first one and then
	/// another on the next run. Empty when the condition holds.
	///
	/// Under an `||` the members are reported together, since any one of them
	/// would have satisfied it and none did. Under a `!` the atoms that made the
	/// inner condition TRUE are the ones at fault, so the walk inverts with it.
	pub fn explain(
		&self,
		conditions: &BsxConditions,
		cx: &ConditionCx,
	) -> Result<Vec<String>> {
		let mut failures = Vec::new();
		self.collect_failures(conditions, cx, false, &mut failures)?;
		failures.sort();
		failures.dedup();
		Ok(failures)
	}

	/// Walk collecting the atoms responsible for the condition being false.
	/// `negated` tracks whether an odd number of `!` encloses this node, which
	/// flips which outcome counts as a failure.
	fn collect_failures(
		&self,
		conditions: &BsxConditions,
		cx: &ConditionCx,
		negated: bool,
		failures: &mut Vec<String>,
	) -> Result<()> {
		match self {
			Self::Atom {
				namespace,
				argument,
			} => {
				if self.evaluate_atom(conditions, cx)? == negated {
					failures.push(match negated {
						false => format!("{namespace}:{argument}"),
						true => format!("!{namespace}:{argument}"),
					});
				}
			}
			Self::Not(inner) => {
				inner.collect_failures(conditions, cx, !negated, failures)?
			}
			// `All` under no negation (and `Any` under one) fails member by
			// member, so only the members that actually failed are reported.
			Self::All(members) | Self::Any(members) => {
				for member in members {
					member.collect_failures(conditions, cx, negated, failures)?;
				}
			}
		}
		Ok(())
	}

	/// Resolve a single atom through its namespace handler.
	fn evaluate_atom(
		&self,
		conditions: &BsxConditions,
		cx: &ConditionCx,
	) -> Result<bool> {
		let Self::Atom {
			namespace,
			argument,
		} = self
		else {
			unreachable!("only called on an atom");
		};
		let Some(handler) = conditions.get(namespace) else {
			bevybail!(
				"unknown cfg namespace `{namespace}` (in `{namespace}:{argument}`). \
				Registered namespaces: {}",
				conditions.namespaces()
			);
		};
		handler(cx, argument)
	}
}

/// What a cfg condition handler may consult: the world being built into, and
/// the [`CrateRegistration`] set collected from it once per build.
///
/// Read-only by construction. A condition answers a question about this build;
/// one that mutated the world would make the answer depend on evaluation order,
/// which short-circuiting does not promise.
pub struct ConditionCx<'a> {
	/// The world the document is building into.
	pub world: &'a World,
	/// Every spawned crate registration, the input to `feature:` and `version:`.
	pub registrations: &'a [CrateRegistration],
}

/// A cfg namespace handler: answers whether `argument` holds for this build, ie
/// `infra` under the `feature` namespace.
pub type BuildConditionFn =
	Arc<dyn Fn(&ConditionCx, &str) -> Result<bool> + Send + Sync>;

/// Maps a cfg atom namespace to its [`BuildConditionFn`].
///
/// Core registers `feature`, `version` and `env` (see the [module docs](self));
/// a downstream crate adds its own facts here, the same seam shape as
/// [`BsxTagResolvers`].
#[derive(Resource)]
pub struct BsxConditions(HashMap<SmolStr, BuildConditionFn>);

impl Default for BsxConditions {
	fn default() -> Self {
		let mut conditions = Self(HashMap::default());
		conditions.insert("feature", |cx, argument| {
			Ok(CrateRegistration::has_feature_item(
				cx.registrations,
				argument,
			))
		});
		conditions.insert("version", |cx, argument| {
			Ok(CrateRegistration::meets_version_item(
				cx.registrations,
				argument,
			))
		});
		// the process environment: set and non-empty, or equal to a value.
		conditions.insert("env", |_cx, argument| {
			Ok(match argument.split_once('=') {
				Some((key, expected)) => env_ext::var(key)
					.map(|value| value.as_str() == expected)
					.unwrap_or(false),
				None => env_ext::var(argument)
					.map(|value| !value.is_empty())
					.unwrap_or(false),
			})
		});
		conditions
	}
}

impl BsxConditions {
	/// Register a handler for `namespace`, replacing any existing one.
	pub fn insert(
		&mut self,
		namespace: impl Into<SmolStr>,
		handler: impl Fn(&ConditionCx, &str) -> Result<bool> + Send + Sync + 'static,
	) -> &mut Self {
		self.0.insert(namespace.into(), Arc::new(handler));
		self
	}

	/// The handler registered for `namespace`, if any.
	pub fn get(&self, namespace: &str) -> Option<BuildConditionFn> {
		self.0.get(namespace).cloned()
	}

	/// The registered namespaces as a sorted, comma-separated list, for the
	/// error a misspelled namespace raises.
	pub fn namespaces(&self) -> String {
		let mut names = self.0.keys().map(SmolStr::as_str).collect::<Vec<_>>();
		names.sort();
		names.join(", ")
	}

	/// A shallow copy of the handler map: the [`Arc`]s are cloned so evaluation
	/// can hold the handlers while the world is borrowed immutably.
	///
	/// Every consumer of a condition needs this, since [`ConditionCx`] borrows
	/// the world the seam itself lives in.
	pub fn clone_seam(&self) -> Self { Self(self.0.clone()) }
}

/// What a `bx:cfg` leaves where it excluded an element: the condition that was
/// false, the tag that is missing, and the element's `path` if it declared one.
///
/// Carries no behavior of its own and spawns no children, so the effect the
/// exclusion existed to prevent is still prevented. It exists so a host can
/// answer "why is this not here?" rather than leaving a hole: `beet_router`
/// turns a tombstone with a path into a route that exists and refuses, naming
/// the condition.
#[derive(Debug, Default, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component, Default)]
pub struct CfgExcluded {
	/// The condition source that evaluated false, ie `feature:infra`.
	pub condition: SmolStr,
	/// The tag that was excluded, ie `Route`.
	pub tag: SmolStr,
	/// The excluded element's own `path` attribute, empty if it had none. A
	/// route-shaped exclusion can therefore still be addressed and reported;
	/// anything else is simply gone.
	pub path: SmolStr,
}

impl CfgExcluded {
	/// The message a host reports when something reaches an excluded branch.
	pub fn message(&self) -> String {
		format!(
			"`<{}>` is excluded from this build: its `bx:cfg=\"{}\"` is false. \
			Rebuild with the features it names to get this branch back.",
			self.tag, self.condition
		)
	}
}

/// The `bx:cfg` attribute key.
const BUILD_CFG_KEY: &str = "bx:cfg";

/// The synthetic tag the prune pass emits in an excluded element's place,
/// resolving to [`CfgExcluded`] through the ordinary uppercase-tag path.
const TOMBSTONE_TAG: &str = "CfgExcluded";

/// Whether any node in the tree carries a `bx:cfg`, so a document that declares
/// none skips the prune walk (and its clone) entirely.
pub(super) fn contains_build_cfg(nodes: &[BsxNode]) -> bool {
	nodes.iter().any(|node| match node {
		BsxNode::Element(el) => {
			el.attributes.iter().any(|attr| attr.key == BUILD_CFG_KEY)
				|| contains_build_cfg(&el.children)
		}
		_ => false,
	})
}

/// The syntax tree with every `bx:cfg`-excluded branch replaced by its
/// tombstone, evaluated against `world`'s registered conditions and crate
/// registrations.
///
/// Runs before the build walk, so exclusion is a property of the DOCUMENT
/// rather than of any entity: nothing downstream (ref collection, tag
/// resolution, includes) ever sees an excluded node's content.
pub(super) fn prune_build_cfg(
	nodes: &[BsxNode],
	world: &mut World,
) -> Result<Vec<BsxNode>> {
	// clone the seam out of the resource so the handlers may borrow the world.
	let conditions = world.get_resource_or_init::<BsxConditions>().clone_seam();
	let registrations =
		world.with_state::<Query<&CrateRegistration>, _>(|query| {
			query.iter().cloned().collect::<Vec<_>>()
		});
	let cx = ConditionCx {
		world,
		registrations: &registrations,
	};
	prune_nodes(nodes, &conditions, &cx)
}

fn prune_nodes(
	nodes: &[BsxNode],
	conditions: &BsxConditions,
	cx: &ConditionCx,
) -> Result<Vec<BsxNode>> {
	let mut kept = Vec::with_capacity(nodes.len());
	for node in nodes {
		let BsxNode::Element(el) = node else {
			kept.push(node.clone());
			continue;
		};
		if let Some(source) = build_cfg_attr(el)? {
			let condition = BuildCondition::parse(source)
				.map_err(|err| bevyhow!("in `<{} bx:cfg>`: {err}", el.tag))?;
			if !condition.evaluate(conditions, cx)? {
				debug!(
					"`bx:cfg` excluded `<{}>`: `{source}` is false in this build",
					el.tag
				);
				kept.extend(tombstones(el, source));
				continue;
			}
		}
		let mut el = el.clone();
		el.children = prune_nodes(&el.children, conditions, cx)?;
		kept.push(BsxNode::Element(el));
	}
	Ok(kept)
}

/// The nodes an excluded element is replaced by: one childless
/// `<CfgExcluded condition=".." tag=".." path=".."/>` per ADDRESSABLE node on
/// the excluded branch's frontier.
///
/// Emitted as synthetic elements rather than spawned directly so they resolve
/// through the same uppercase-tag path as any other component, which keeps the
/// build walk unaware that exclusion exists.
///
/// # Why the frontier rather than just the excluded node
///
/// A tombstone is only useful to a host if it can be addressed, which for
/// `beet_router` means the element declared a `path`. Excluding a `<Route>`
/// directly gives one; excluding a `<Fragment>` that GROUPS several routes
/// would give a pathless one and lose every route under it.
///
/// That would make the grouping form strictly worse than repeating the
/// condition on each route, which is a bad reason to write markup one way. So
/// the walk descends through the pathless nodes of an excluded branch and
/// leaves a tombstone at each addressable one it meets, stopping there rather
/// than going deeper. Both forms then behave identically and the author picks
/// on readability, which is the only thing that should decide it.
///
/// Reading attributes to find the frontier costs nothing and runs no effect:
/// this walk touches the syntax tree only. It is BUILDING an excluded node that
/// must not happen, and none of them is built.
fn tombstones(el: &BsxElement, condition: &str) -> Vec<BsxNode> {
	let mut out = Vec::new();
	collect_tombstones(el, condition, &mut out);
	// nothing addressable underneath: record the exclusion at the node itself,
	// so it is still visible as data even though no route can report it
	if out.is_empty() {
		out.push(tombstone(el, condition, None));
	}
	out
}

/// Walk an excluded branch collecting its addressable frontier: the outermost
/// `path`-declaring elements, each terminating its own descent.
fn collect_tombstones(
	el: &BsxElement,
	condition: &str,
	out: &mut Vec<BsxNode>,
) {
	if let Some(path) = path_attr(el) {
		out.push(tombstone(el, condition, Some(path)));
		return;
	}
	for child in &el.children {
		if let BsxNode::Element(child) = child {
			collect_tombstones(child, condition, out);
		}
	}
}

/// One tombstone node.
fn tombstone(
	el: &BsxElement,
	condition: &str,
	path: Option<&str>,
) -> BsxNode {
	let mut attributes = vec![
		string_attribute("condition", condition),
		string_attribute("tag", &el.tag),
	];
	if let Some(path) = path {
		attributes.push(string_attribute("path", path));
	}
	BsxNode::Element(BsxElement {
		tag: TOMBSTONE_TAG.into(),
		tag_literal: None,
		attributes,
		children: Vec::new(),
		self_closing: true,
	})
}

/// An element's `path` string attribute, if it declares one.
fn path_attr(el: &BsxElement) -> Option<&str> {
	el.attributes.iter().find_map(|attr| match &attr.value {
		AttrValue::Str(value) if attr.key == "path" => Some(value.as_str()),
		_ => None,
	})
}

fn string_attribute(key: &str, value: &str) -> BsxAttribute {
	BsxAttribute {
		key: key.into(),
		value: AttrValue::Str(value.into()),
	}
}

/// An element's `bx:cfg` condition text, if it declares one.
fn build_cfg_attr(el: &BsxElement) -> Result<Option<&str>> {
	let Some(attr) = el.attributes.iter().find(|attr| attr.key == BUILD_CFG_KEY)
	else {
		return Ok(None);
	};
	match &attr.value {
		AttrValue::Str(source) => Ok(Some(source.as_str())),
		// a condition is answered from the build's facts, never from document
		// state, so there is nothing an expression could usefully say here.
		_ => bevybail!(
			"`bx:cfg` expects a quoted condition, ie `bx:cfg=\"feature:infra\"`"
		),
	}
}

// 💡 the condition grammar: a tokenizer and a three-level recursive descent.

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
	Open,
	Close,
	Bang,
	And,
	Or,
	Atom(SmolStr),
}

/// The characters an atom is made of: enough for `crate/feature`, a dotted or
/// hyphenated env key, a `crate@version`, and a `KEY=value` comparison.
fn is_atom_char(ch: char) -> bool {
	ch.is_alphanumeric()
		|| matches!(ch, '_' | '-' | '.' | '/' | ':' | '=' | '*' | '@')
}

fn tokenize(source: &str) -> Result<Vec<Token>> {
	let mut tokens = Vec::new();
	let mut rest = source;
	while let Some(ch) = rest.chars().next() {
		if ch.is_whitespace() {
			rest = &rest[ch.len_utf8()..];
			continue;
		}
		match ch {
			'(' => {
				tokens.push(Token::Open);
				rest = &rest[1..];
			}
			')' => {
				tokens.push(Token::Close);
				rest = &rest[1..];
			}
			'!' => {
				tokens.push(Token::Bang);
				rest = &rest[1..];
			}
			'&' | '|' => {
				let (token, pair) = match ch {
					'&' => (Token::And, "&&"),
					_ => (Token::Or, "||"),
				};
				if !rest.starts_with(pair) {
					bevybail!(
						"cfg conditions spell their operators `&&`, `||` and `!`; \
						found a single `{ch}`"
					);
				}
				tokens.push(token);
				rest = &rest[2..];
			}
			ch if is_atom_char(ch) => {
				let end = rest
					.find(|ch: char| !is_atom_char(ch))
					.unwrap_or(rest.len());
				let (word, tail) = rest.split_at(end);
				rest = tail;
				// the keyword spellings of the operators are a common reach;
				// name the right one rather than parsing them as atoms
				match word {
					"and" | "or" | "not" => bevybail!(
						"cfg conditions use Rust's operators: `&&`, `||` and `!`, \
						not `{word}`"
					),
					_ => tokens.push(Token::Atom(word.into())),
				}
			}
			ch => bevybail!("unexpected `{ch}` in a cfg condition"),
		}
	}
	Ok(tokens)
}

struct ConditionParser<'a> {
	tokens: &'a [Token],
	pos: usize,
}

impl ConditionParser<'_> {
	fn peek(&self) -> Option<&Token> { self.tokens.get(self.pos) }

	fn eat(&mut self, token: &Token) -> bool {
		if self.peek() == Some(token) {
			self.pos += 1;
			true
		} else {
			false
		}
	}

	/// `all ("||" all)*`
	fn parse_any(&mut self) -> Result<BuildCondition> {
		let mut members = vec![self.parse_all()?];
		while self.eat(&Token::Or) {
			members.push(self.parse_all()?);
		}
		Ok(match members.len() {
			1 => members.pop().unwrap(),
			_ => BuildCondition::Any(members),
		})
	}

	/// `unary ("&&" unary)*`
	fn parse_all(&mut self) -> Result<BuildCondition> {
		let mut members = vec![self.parse_unary()?];
		while self.eat(&Token::And) {
			members.push(self.parse_unary()?);
		}
		Ok(match members.len() {
			1 => members.pop().unwrap(),
			_ => BuildCondition::All(members),
		})
	}

	/// `"!" unary | "(" any ")" | atom`
	fn parse_unary(&mut self) -> Result<BuildCondition> {
		if self.eat(&Token::Bang) {
			return Ok(BuildCondition::Not(Box::new(self.parse_unary()?)));
		}
		if self.eat(&Token::Open) {
			let inner = self.parse_any()?;
			if !self.eat(&Token::Close) {
				bevybail!("unclosed `(` in a cfg condition");
			}
			return Ok(inner);
		}
		match self.peek() {
			Some(Token::Atom(word)) => {
				let word = word.clone();
				self.pos += 1;
				parse_atom(&word)
			}
			Some(Token::Close) => {
				bevybail!("unexpected `)` in a cfg condition")
			}
			Some(Token::And) | Some(Token::Or) => {
				bevybail!("a cfg operator needs a condition on both sides")
			}
			Some(Token::Bang) | Some(Token::Open) => {
				unreachable!("eaten above")
			}
			None => bevybail!(
				"unexpected end of a cfg condition: expected `feature:<name>`"
			),
		}
	}
}

/// `namespace:argument`, split at the FIRST `:` so an argument may contain more.
fn parse_atom(word: &str) -> Result<BuildCondition> {
	let Some((namespace, argument)) = word.split_once(':') else {
		bevybail!(
			"cfg atoms are `namespace:argument`, ie `feature:infra`; got `{word}`"
		);
	};
	if namespace.is_empty() || argument.is_empty() {
		bevybail!(
			"cfg atoms are `namespace:argument`, ie `feature:infra`; got `{word}`"
		);
	}
	Ok(BuildCondition::Atom {
		namespace: namespace.into(),
		argument: argument.into(),
	})
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	fn parse(source: &str) -> BuildCondition {
		BuildCondition::parse(source).unwrap()
	}

	fn atom(namespace: &str, argument: &str) -> BuildCondition {
		BuildCondition::Atom {
			namespace: namespace.into(),
			argument: argument.into(),
		}
	}

	#[crate::test]
	fn parses_rust_operators_with_rust_precedence() {
		parse("feature:infra").xpect_eq(atom("feature", "infra"));
		// the argument keeps everything after the first `:`, so `crate/feature`,
		// `crate@version` and a `KEY=value` env comparison all survive
		parse("feature:beet_esp/alvik")
			.xpect_eq(atom("feature", "beet_esp/alvik"));
		parse("version:beet_esp@0.5.9")
			.xpect_eq(atom("version", "beet_esp@0.5.9"));
		parse("env:CI=true").xpect_eq(atom("env", "CI=true"));
		parse("!feature:infra")
			.xpect_eq(BuildCondition::Not(Box::new(atom("feature", "infra"))));
		// `&&` binds tighter than `||`
		parse("a:1 || b:2 && c:3").xpect_eq(BuildCondition::Any(vec![
			atom("a", "1"),
			BuildCondition::All(vec![atom("b", "2"), atom("c", "3")]),
		]));
		// ..and parens override that
		parse("(a:1 || b:2) && c:3").xpect_eq(BuildCondition::All(vec![
			BuildCondition::Any(vec![atom("a", "1"), atom("b", "2")]),
			atom("c", "3"),
		]));
		// the shape from the docs, whole
		parse("(feature:infra && feature:extra && !feature:lean) || feature:all")
			.xpect_eq(BuildCondition::Any(vec![
				BuildCondition::All(vec![
					atom("feature", "infra"),
					atom("feature", "extra"),
					BuildCondition::Not(Box::new(atom("feature", "lean"))),
				]),
				atom("feature", "all"),
			]));
	}

	#[crate::test]
	fn rejects_malformed_conditions() {
		// a bare atom has no namespace to resolve, so it is not a condition
		BuildCondition::parse("infra").xpect_err();
		BuildCondition::parse("").xpect_err();
		BuildCondition::parse("feature:").xpect_err();
		BuildCondition::parse(":infra").xpect_err();
		BuildCondition::parse("feature:a &&").xpect_err();
		BuildCondition::parse("(feature:a").xpect_err();
		BuildCondition::parse("feature:a feature:b").xpect_err();
		// a single `&` is a typo for `&&`, not an operator of its own
		BuildCondition::parse("feature:a & feature:b").xpect_err();
		// the keyword spellings name the right operator rather than parsing as atoms
		BuildCondition::parse("feature:a and feature:b").xpect_err();
		BuildCondition::parse("not feature:a").xpect_err();
	}

	/// Build `markup` into a world whose primary registration compiles `infra`
	/// at `1.2.3`, returning the world and the built root, or the build error.
	fn try_build(markup: &str) -> Result<(World, Entity)> {
		let mut world = (TemplatePlugin, DocumentPlugin).into_world();
		world.init_resource::<BsxConditions>();
		{
			let registry = world.resource_mut::<AppTypeRegistry>();
			let mut registry = registry.write();
			registry.register::<PackageConfig>();
			registry.register::<CfgExcluded>();
		}
		world.spawn(
			CrateRegistration::new("test_bin", "1.2.3")
				.with_feature("infra")
				.with_skip_prefix(),
		);
		let nodes = BsxNode::parse_document(markup, &BsxParseConfig::bsx())?;
		let root = world
			.spawn_template(BsxTemplate::container(
				nodes,
				BsxTemplateRegistry::default(),
			))?
			.id();
		world.flush();
		Ok((world, root))
	}

	fn build(markup: &str) -> (World, Entity) { try_build(markup).unwrap() }

	/// The child element tags directly under the built root.
	fn child_tags(world: &World, root: Entity) -> Vec<String> {
		world
			.entity(root)
			.get::<Children>()
			.map(|children| {
				children
					.iter()
					.filter_map(|child| {
						world
							.entity(child)
							.get::<Element>()
							.map(|el| el.tag().to_string())
					})
					.collect()
			})
			.unwrap_or_default()
	}

	#[crate::test]
	fn keeps_a_met_branch_and_drops_an_unmet_one() {
		let (world, root) = build(
			r#"<div bx:cfg="feature:infra"/><span bx:cfg="feature:lambda"/><p/>"#,
		);
		child_tags(&world, root)
			.xpect_eq(vec!["div".to_string(), "p".to_string()]);
	}

	/// The property the whole mechanism exists for: an excluded branch leaves no
	/// content, so a build-time EFFECT inside it never runs. Contrast the
	/// unregistered-tag path, which spawns an inert entity and builds children.
	///
	/// The positive control is the point: the identical markup under a MET
	/// condition does apply the effect, so the absence below is the gate working
	/// rather than the declaration never having worked.
	#[crate::test]
	fn an_excluded_branch_runs_no_build_time_effect() {
		let markup = |condition: &str| {
			format!(
				r#"<div bx:cfg="{condition}"><PackageConfig title="Applied"/><span/></div>"#
			)
		};
		// met: the branch builds and its resource declaration lands
		let (world, root) = build(&markup("feature:infra"));
		child_tags(&world, root).xpect_eq(vec!["div".to_string()]);
		world
			.resource::<PackageConfig>()
			.title
			.as_str()
			.xpect_eq("Applied");
		// unmet: the declaration never applied
		let (world, root) = build(&markup("feature:lambda"));
		world
			.get_resource::<PackageConfig>()
			.map(|config| config.title.to_string())
			.unwrap_or_default()
			.xpect_not_eq("Applied");
		// ..and what is there is a childless tombstone, not the element
		let children = world.entity(root).get::<Children>().unwrap();
		children.len().xpect_eq(1);
		let tombstone = children[0];
		world.entity(tombstone).contains::<Element>().xpect_false();
		world.entity(tombstone).get::<Children>().is_none().xpect_true();
		let excluded = world.entity(tombstone).get::<CfgExcluded>().unwrap();
		excluded.tag.as_str().xpect_eq("div");
		excluded.condition.as_str().xpect_eq("feature:lambda");
	}

	/// A route-shaped exclusion keeps its path, which is what lets a host report
	/// `/site` as excluded rather than unknown.
	#[crate::test]
	fn a_tombstone_keeps_the_excluded_path() {
		let (world, root) =
			build(r#"<Route path="site" bx:cfg="feature:lambda"><b/></Route>"#);
		let tombstone = world.entity(root).get::<Children>().unwrap()[0];
		let excluded = world.entity(tombstone).get::<CfgExcluded>().unwrap();
		excluded.path.as_str().xpect_eq("site");
		excluded.tag.as_str().xpect_eq("Route");
		excluded.message().contains("feature:lambda").xpect_true();
	}

	/// Gating a GROUP is equivalent to gating each member: a `<Fragment>` that
	/// wraps several routes leaves a tombstone per route, not one for itself.
	///
	/// Without this the grouping form would silently lose every route under it,
	/// which would make it strictly worse than repeating the condition and let
	/// an implementation detail decide how markup is written.
	#[crate::test]
	fn gating_a_group_is_the_same_as_gating_each_member() {
		let paths = |markup: &str| {
			let (world, root) = build(markup);
			let mut paths = world
				.entity(root)
				.get::<Children>()
				.unwrap()
				.iter()
				.filter_map(|child| {
					world
						.entity(child)
						.get::<CfgExcluded>()
						.map(|excluded| excluded.path.to_string())
				})
				.collect::<Vec<_>>();
			paths.sort();
			paths
		};
		let grouped = paths(
			r#"<Fragment bx:cfg="feature:lambda">
				<Route path="site"><Template src="infra/site.bsx"/></Route>
				<Route path="mail"><Template src="infra/mail.bsx"/></Route>
			</Fragment>"#,
		);
		let each = paths(
			r#"<Route path="site" bx:cfg="feature:lambda"><Template src="infra/site.bsx"/></Route>
			   <Route path="mail" bx:cfg="feature:lambda"><Template src="infra/mail.bsx"/></Route>"#,
		);
		grouped.xpect_eq(vec!["mail".to_string(), "site".to_string()]);
		grouped.xpect_eq(each);
	}

	/// An excluded branch with nothing addressable under it still records the
	/// exclusion, just with no path for a host to report at.
	#[crate::test]
	fn a_pathless_exclusion_is_still_recorded() {
		let (world, root) =
			build(r#"<Fragment bx:cfg="feature:lambda"><div/><span/></Fragment>"#);
		let children = world.entity(root).get::<Children>().unwrap();
		children.len().xpect_eq(1);
		let excluded = world.entity(children[0]).get::<CfgExcluded>().unwrap();
		excluded.tag.as_str().xpect_eq("Fragment");
		excluded.path.is_empty().xpect_true();
	}

	#[crate::test]
	fn operators_compose() {
		let (world, root) = build(
			r#"<a bx:cfg="feature:infra && !feature:lambda"/>
			   <b bx:cfg="feature:infra && feature:lambda"/>
			   <i bx:cfg="feature:lambda || feature:infra"/>
			   <u bx:cfg="!(feature:infra || feature:lambda)"/>
			   <s bx:cfg="version:1.0.0 && !version:9.0.0"/>"#,
		);
		child_tags(&world, root).xpect_eq(vec![
			"a".to_string(),
			"i".to_string(),
			"s".to_string(),
		]);
	}

	/// Exclusion is recursive: a met branch still prunes inside itself.
	#[crate::test]
	fn prunes_nested_branches() {
		let (world, root) = build(
			r#"<div bx:cfg="feature:infra"><span bx:cfg="feature:lambda"/><p/></div>"#,
		);
		let outer = world.entity(root).get::<Children>().unwrap()[0];
		child_tags(&world, outer).xpect_eq(vec!["p".to_string()]);
	}

	/// A misspelled namespace is the one failure that must not degrade: it is
	/// indistinguishable from a false condition by outcome, and silently
	/// dropping the subtree is exactly the bug this refuses to have.
	#[crate::test]
	fn an_unknown_namespace_is_a_hard_error() {
		let error = try_build(r#"<div bx:cfg="featrue:infra"/>"#)
			.err()
			.unwrap()
			.to_string();
		// it names the typo AND the namespaces that do exist
		error.contains("featrue").xpect_true();
		error.contains("feature").xpect_true();
	}

	/// `bx:cfg` is a structural directive, so it never reaches the element as an
	/// html attribute.
	#[crate::test]
	fn is_not_an_html_attribute() {
		let (world, root) = build(r#"<div bx:cfg="feature:infra" id="kept"/>"#);
		let el = world.entity(root).get::<Children>().unwrap()[0];
		let attrs = world
			.entity(el)
			.get::<Attributes>()
			.unwrap()
			.iter()
			.map(|attr| {
				world.entity(attr).get::<Attribute>().unwrap().to_string()
			})
			.collect::<Vec<_>>();
		attrs.xpect_eq(vec!["id".to_string()]);
	}

	/// `explain` reports EVERY false atom, which is the whole reason it exists:
	/// a load that fails should name everything missing at once.
	#[crate::test]
	fn explain_reports_every_failure_without_short_circuiting() {
		let mut world = World::new();
		world.init_resource::<BsxConditions>();
		let conditions = world.resource::<BsxConditions>();
		let registrations = [CrateRegistration::new("test_bin", "1.2.3")
			.with_feature("infra")
			.with_skip_prefix()];
		let cx = ConditionCx {
			world: &world,
			registrations: &registrations,
		};
		// two of the three `&&` members fail, and both are named
		BuildCondition::parse("feature:infra && feature:a && feature:b")
			.unwrap()
			.explain(conditions, &cx)
			.unwrap()
			.xpect_eq(vec![
				"feature:a".to_string(),
				"feature:b".to_string(),
			]);
		// a satisfied condition explains nothing
		BuildCondition::parse("feature:infra")
			.unwrap()
			.explain(conditions, &cx)
			.unwrap()
			.xpect_empty();
		// under a `!`, the atom at fault is the one that was TRUE
		BuildCondition::parse("!feature:infra")
			.unwrap()
			.explain(conditions, &cx)
			.unwrap()
			.xpect_eq(vec!["!feature:infra".to_string()]);
	}
}
