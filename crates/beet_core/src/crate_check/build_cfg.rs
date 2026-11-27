//! [`Cfg`]: the one mechanism for excluding a region of a document from a
//! binary, and the condition grammar it and its two siblings share.
//!
//! This is the markup twin of Rust's `#[cfg]`, and the analogy is exact enough
//! to lean on: `rsx!` needs nothing here because it is compiled and real
//! `#[cfg]` already applies, while a document is spawned at runtime and has no
//! compiler to do it. Same concept, same operators, two front ends.
//!
//! ```html
//! <Route path="site" bx:cfg="feature:infra && feature:extra">
//!     <Instance document="infra/site.bsx"/>
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
//! structure.** An [`Instance`] reads a document. A node that DOES something
//! while the tree is being built has no inert form, so "build it anyway, just
//! without behavior" is not available to it. Either it runs or it does not
//! exist.
//!
//! `bx:cfg` is how such a branch is kept out of a binary that must not run it.
//! The body keeps the branch whole, a `Cfg` component on its root, so one body
//! is valid for every target; a binary answering it false leaves the branch
//! out as the body spawns (`beet_scene`'s `DynamicTemplate::reflect`), so nothing downstream can
//! observe it: no entity, no template expanded, no document read. Not
//! skipped, not inert, not spawned-then-emptied. Absent.
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
//! that exists and refuses, claiming its whole subtree, so a lean binary still
//! LISTS `/site` in `--help` and dispatching `/site` or `/site/deploy` reports
//! the condition that excluded it rather than "unknown route". That is what
//! lets this one mechanism cover the case a separate dispatch-time gate used
//! to.
//!
//! The honest limit, which is a property of the problem and not of this design:
//! routes *below* an excluded node cannot be listed, because the include that
//! defines them never ran. Nothing could do better without running the effect
//! that exclusion exists to prevent.
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
//! An atom's namespace resolves through [`BsxConditions`]. Core registers the
//! facts every build has:
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

/// The condition as it parses back, a compound member of another kind in
/// parentheses: `feature:a && (feature:b || !env:CI)`.
impl core::fmt::Display for BuildCondition {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		let member = |f: &mut core::fmt::Formatter<'_>,
		              member: &BuildCondition,
		              compound: bool|
		 -> core::fmt::Result {
			match compound {
				true => write!(f, "({member})"),
				false => write!(f, "{member}"),
			}
		};
		let join = |f: &mut core::fmt::Formatter<'_>,
		            members: &[BuildCondition],
		            separator: &str,
		            is_other: fn(&BuildCondition) -> bool|
		 -> core::fmt::Result {
			for (i, item) in members.iter().enumerate() {
				if i > 0 {
					f.write_str(separator)?;
				}
				member(f, item, is_other(item))?;
			}
			Ok(())
		};
		match self {
			Self::Atom {
				namespace,
				argument,
			} => write!(f, "{namespace}:{argument}"),
			Self::Not(inner) => {
				f.write_str("!")?;
				member(f, inner, !matches!(**inner, Self::Atom { .. }))
			}
			Self::All(members) => {
				join(f, members, " && ", |item| matches!(item, Self::Any(_)))
			}
			Self::Any(members) => {
				join(f, members, " || ", |item| matches!(item, Self::All(_)))
			}
		}
	}
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
			// a group that holds has nothing at fault, whichever members are
			// false (an `||` satisfied by one member is satisfied); a group that
			// fails is walked member by member, so an `&&` names only the
			// members that failed and an `||` names every member, since any
			// one would have done.
			Self::All(members) | Self::Any(members) => {
				if self.evaluate(conditions, cx)? != negated {
					return Ok(());
				}
				for member in members {
					member
						.collect_failures(conditions, cx, negated, failures)?;
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
/// a downstream crate adds its own facts here.
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
			CrateRegistration::meets_version_item(cx.registrations, argument)
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
#[cfg_attr(feature = "serde", derive(Serialize))]
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

/// Excludes the entity and everything under it from a binary that does not
/// meet a condition, `bx:cfg="feature:infra"` in markup. Kept in the body and
/// answered when it spawns, where a false condition leaves a [`CfgExcluded`]
/// in the branch's place.
#[derive(Debug, Default, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component, Default)]
pub struct Cfg {
	/// The condition, ie `feature:infra && !env:CI`.
	pub cfg: SmolStr,
	/// The tag it was authored on, which a tombstone reports.
	#[reflect(default)]
	pub tag: SmolStr,
}

impl Cfg {
	/// The directive it is authored as.
	pub const DIRECTIVE: &'static str = "bx:cfg";
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
		parse(
			"(feature:infra && feature:extra && !feature:lean) || feature:all",
		)
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

	/// `explain` reports EVERY false atom, which is the whole reason it exists:
	/// a load that fails should name everything missing at once.
	#[crate::test]
	fn explain_reports_every_failure_without_short_circuiting() {
		let mut world = World::new();
		world.init_resource::<BsxConditions>();
		let conditions = world.resource::<BsxConditions>();
		let registrations = [CrateRegistration::new(
			"test_bin",
			Version::parse("1.2.3").unwrap(),
		)
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
			.xpect_eq(vec!["feature:a".to_string(), "feature:b".to_string()]);
		// a satisfied condition explains nothing, an `||` one member satisfies
		// included: the false member is not at fault
		BuildCondition::parse("feature:infra")
			.unwrap()
			.explain(conditions, &cx)
			.unwrap()
			.xpect_empty();
		BuildCondition::parse("feature:a || feature:infra")
			.unwrap()
			.explain(conditions, &cx)
			.unwrap()
			.xpect_empty();
		// an `||` nobody satisfies names every member
		BuildCondition::parse("feature:a || feature:b")
			.unwrap()
			.explain(conditions, &cx)
			.unwrap()
			.xpect_eq(vec!["feature:a".to_string(), "feature:b".to_string()]);
		// under a `!`, the atom at fault is the one that was TRUE
		BuildCondition::parse("!feature:infra")
			.unwrap()
			.explain(conditions, &cx)
			.unwrap()
			.xpect_eq(vec!["!feature:infra".to_string()]);
	}
}
