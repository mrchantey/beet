//! `bx:cfg`: build-time exclusion of a document branch.
//!
//! Beet's default posture is that **structure is universal**: a document builds
//! whole in every binary, an unresolvable tag becomes an inert entity, and a
//! capability gap surfaces at *dispatch* through [`RequireFeatures`]. That is
//! the right default, and it is what lets a lean binary still enumerate a route
//! tree it cannot run.
//!
//! It has one boundary it cannot cross: **some nodes are effects, not
//! structure.** `<Template src>` reads a file. A node that DOES something at
//! build time has no inert form, so "build it anyway, just without behavior" is
//! not available to it. Either it runs or it does not exist.
//!
//! `bx:cfg` is that second option, and the whole mechanism: a condition on any
//! node, evaluated against this build's facts, that removes the node and its
//! entire subtree from the syntax tree *before* the tree becomes a world. Not
//! skipped, not inert, not spawned-then-emptied. Absent. Nothing downstream can
//! observe it, because pruning happens ahead of every walk that would: no
//! entity, no tag resolution, no directive, no `bx:ref` pin, no include read.
//!
//! ```html
//! <Fragment bx:cfg="feature:infra and feature:extra">
//!     <Route path="site"><Template src="infra/site.bsx"/></Route>
//! </Fragment>
//! ```
//!
//! # `bx:cfg` and `RequireFeatures` answer different questions
//!
//! | | question | when | a failure looks like |
//! |---|---|---|---|
//! | `bx:cfg` | does this branch EXIST in this build? | build | the branch is not there |
//! | [`RequireFeatures`] | may this branch RUN? | dispatch | a call fails naming the missing features |
//!
//! They compose, and the choice is not a matter of taste. Reach for
//! `RequireFeatures` by default: it keeps the structure visible and turns a
//! missing capability into a precise error at the moment someone asks for it.
//! Reach for `bx:cfg` when the branch's mere presence has a cost the lean build
//! must not pay, which in practice means it performs a build-time effect.
//!
//! # The condition grammar
//!
//! Deliberately total and tiny, because a gate is not a place for cleverness:
//!
//! ```text
//! condition := any
//! any       := all ("or" all)*
//! all       := unary ("and" unary)*
//! unary     := "!" unary | "(" any ")" | atom
//! atom      := namespace ":" argument
//! ```
//!
//! `!` binds tightest, then `and`, then `or`. There are no other operators and
//! no bare atoms: every atom names its namespace, so a condition says what KIND
//! of fact it consults and a reader never has to guess.
//!
//! # Namespaces are a seam
//!
//! An atom's namespace resolves through [`BsxConditions`], the same
//! register-a-handler shape as [`BsxTagResolvers`] and [`StyleResolver`]. Core
//! registers the two facts every build has:
//!
//! - `feature:<name>` / `feature:<crate>/<name>` — a compiled cargo feature,
//!   verified against the spawned [`CrateRegistration`] set through
//!   [`CrateCheck::feature_failures`], so this shares ONE requirement grammar
//!   with `<CrateCheck>` and [`RequireFeatures`] rather than inventing a second.
//! - `env:<KEY>` / `env:<KEY>=<value>` — an environment variable, set and
//!   non-empty, or equal to `value`.
//!
//! A downstream crate registers its own against whatever it knows (a stage, a
//! target, a licence tier) without core learning the concept.
//!
//! An **unknown namespace is a hard error**, not a false. A gate that silently
//! excludes a subtree because its condition was misspelled is the single worst
//! failure this mechanism could have, so it is the one case that refuses to
//! degrade.

use crate::prelude::*;
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
	Not(alloc::boxed::Box<BuildCondition>),
	/// `a and b and ..`, true when every member is.
	All(Vec<BuildCondition>),
	/// `a or b or ..`, true when any member is.
	Any(Vec<BuildCondition>),
}

impl BuildCondition {
	/// Parse a `bx:cfg` condition, ie `feature:infra and !env:CI`.
	pub fn parse(source: &str) -> Result<Self> {
		let tokens = tokenize(source)?;
		if tokens.is_empty() {
			bevybail!(
				"empty `bx:cfg` condition: expected something like `feature:infra`"
			);
		}
		let mut parser = ConditionParser { tokens: &tokens, pos: 0 };
		let condition = parser.parse_any()?;
		if parser.pos != tokens.len() {
			bevybail!(
				"trailing input in `bx:cfg=\"{source}\"`: expected `and`, `or` or end of condition"
			);
		}
		Ok(condition)
	}

	/// Evaluate against this build's facts, resolving each atom's namespace
	/// through `conditions`.
	pub fn evaluate(
		&self,
		conditions: &BsxConditions,
		cx: &ConditionCx,
	) -> Result<bool> {
		match self {
			Self::Atom {
				namespace,
				argument,
			} => {
				let Some(handler) = conditions.get(namespace) else {
					bevybail!(
						"unknown `bx:cfg` namespace `{namespace}` (in `{namespace}:{argument}`). \
						Registered namespaces: {}",
						conditions.namespaces()
					);
				};
				handler(cx, argument)
			}
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
}

/// What a `bx:cfg` condition handler may consult: the world being built into,
/// and the [`CrateRegistration`] set collected from it once per build.
///
/// Read-only by construction. A condition answers a question about this build;
/// one that mutated the world would make the answer depend on evaluation order,
/// which the grammar's short-circuiting does not promise.
pub struct ConditionCx<'a> {
	/// The world the document is building into.
	pub world: &'a World,
	/// Every spawned crate registration, the input to a `feature:` atom.
	pub registrations: &'a [CrateRegistration],
}

/// A `bx:cfg` namespace handler: answers whether `argument` holds for this
/// build, ie `infra` under the `feature` namespace.
pub type BuildConditionFn =
	Arc<dyn Fn(&ConditionCx, &str) -> Result<bool> + Send + Sync>;

/// Maps a `bx:cfg` atom namespace to its [`BuildConditionFn`].
///
/// Core registers `feature` and `env` (see the [module docs](self)); a
/// downstream crate adds its own facts here, the same seam shape as
/// [`BsxTagResolvers`].
#[derive(Resource)]
pub struct BsxConditions(HashMap<SmolStr, BuildConditionFn>);

impl Default for BsxConditions {
	fn default() -> Self {
		let mut conditions = Self(HashMap::default());
		// the compiled cargo features, through the one requirement grammar
		// `<CrateCheck>` and `RequireFeatures` already share.
		conditions.insert("feature", |cx, argument| {
			Ok(
				CrateCheck::feature_failures([argument], cx.registrations.iter())
					.is_empty(),
			)
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
		handler: impl Fn(&ConditionCx, &str) -> Result<bool>
		+ Send
		+ Sync
		+ 'static,
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
		let mut names =
			self.0.keys().map(SmolStr::as_str).collect::<Vec<_>>();
		names.sort();
		names.join(", ")
	}
}

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

/// The syntax tree with every `bx:cfg`-excluded branch removed, evaluated
/// against `world`'s registered conditions and crate registrations.
///
/// Runs before the build walk, so exclusion is a property of the DOCUMENT
/// rather than of any entity: nothing downstream (ref collection, tag
/// resolution, includes) ever sees an excluded node.
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

impl BsxConditions {
	/// A shallow copy of the handler map: the [`Arc`]s are cloned so evaluation
	/// can hold the handlers while the world is borrowed immutably.
	fn clone_seam(&self) -> Self { Self(self.0.clone()) }
}

/// The `bx:cfg` attribute key.
const BUILD_CFG_KEY: &str = "bx:cfg";

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
			let condition = BuildCondition::parse(source).map_err(|err| {
				bevyhow!("in `<{} bx:cfg=\"{source}\">`: {err}", el.tag)
			})?;
			if !condition.evaluate(conditions, cx)? {
				debug!(
					"`bx:cfg` excluded `<{}>`: `{source}` is false in this build",
					el.tag
				);
				continue;
			}
		}
		let mut el = el.clone();
		el.children = prune_nodes(&el.children, conditions, cx)?;
		kept.push(BsxNode::Element(el));
	}
	Ok(kept)
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
/// hyphenated env key, and a `KEY=value` comparison.
fn is_atom_char(ch: char) -> bool {
	ch.is_alphanumeric() || matches!(ch, '_' | '-' | '.' | '/' | ':' | '=' | '*' | '@')
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
			'&' | '|' => bevybail!(
				"`bx:cfg` spells its operators `and`, `or` and `!`, not `{ch}{ch}`"
			),
			ch if is_atom_char(ch) => {
				let end = rest
					.find(|ch: char| !is_atom_char(ch))
					.unwrap_or(rest.len());
				let (word, tail) = rest.split_at(end);
				rest = tail;
				tokens.push(match word {
					"and" => Token::And,
					"or" => Token::Or,
					"not" => bevybail!(
						"`bx:cfg` spells negation `!`, not `not`"
					),
					_ => Token::Atom(word.into()),
				});
			}
			ch => bevybail!("unexpected `{ch}` in a `bx:cfg` condition"),
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

	/// `all ("or" all)*`
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

	/// `unary ("and" unary)*`
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
			return Ok(BuildCondition::Not(alloc::boxed::Box::new(
				self.parse_unary()?,
			)));
		}
		if self.eat(&Token::Open) {
			let inner = self.parse_any()?;
			if !self.eat(&Token::Close) {
				bevybail!("unclosed `(` in a `bx:cfg` condition");
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
				bevybail!("unexpected `)` in a `bx:cfg` condition")
			}
			Some(Token::And) | Some(Token::Or) => bevybail!(
				"a `bx:cfg` operator needs a condition on both sides"
			),
			Some(Token::Bang) | Some(Token::Open) => unreachable!("eaten above"),
			None => bevybail!(
				"unexpected end of a `bx:cfg` condition: expected `feature:<name>`"
			),
		}
	}
}

/// `namespace:argument`, split at the FIRST `:` so an argument may contain more.
fn parse_atom(word: &str) -> Result<BuildCondition> {
	let Some((namespace, argument)) = word.split_once(':') else {
		bevybail!(
			"`bx:cfg` atoms are `namespace:argument`, ie `feature:infra`; got `{word}`"
		);
	};
	if namespace.is_empty() || argument.is_empty() {
		bevybail!(
			"`bx:cfg` atoms are `namespace:argument`, ie `feature:infra`; got `{word}`"
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

	/// Parse `source` into the condition AST.
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
	fn parses_atoms_and_operators() {
		parse("feature:infra").xpect_eq(atom("feature", "infra"));
		// the argument keeps everything after the first `:`, so `crate/feature`
		// and a `KEY=value` env comparison both survive.
		parse("feature:beet_esp/alvik")
			.xpect_eq(atom("feature", "beet_esp/alvik"));
		parse("env:CI=true").xpect_eq(atom("env", "CI=true"));
		parse("!feature:infra").xpect_eq(BuildCondition::Not(Box::new(atom(
			"feature", "infra",
		))));
		// `and` binds tighter than `or`
		parse("a:1 or b:2 and c:3").xpect_eq(BuildCondition::Any(vec![
			atom("a", "1"),
			BuildCondition::All(vec![atom("b", "2"), atom("c", "3")]),
		]));
		// ..and parens override that
		parse("(a:1 or b:2) and c:3").xpect_eq(BuildCondition::All(vec![
			BuildCondition::Any(vec![atom("a", "1"), atom("b", "2")]),
			atom("c", "3"),
		]));
	}

	#[crate::test]
	fn rejects_malformed_conditions() {
		// a bare atom has no namespace to resolve, so it is not a condition
		BuildCondition::parse("infra").xpect_err();
		BuildCondition::parse("").xpect_err();
		BuildCondition::parse("feature:").xpect_err();
		BuildCondition::parse(":infra").xpect_err();
		BuildCondition::parse("feature:a and").xpect_err();
		BuildCondition::parse("(feature:a").xpect_err();
		BuildCondition::parse("feature:a feature:b").xpect_err();
		// the near-miss spellings name the right one rather than parsing as an atom
		BuildCondition::parse("feature:a && feature:b").xpect_err();
		BuildCondition::parse("not feature:a").xpect_err();
	}

	/// Build `markup` into a world whose primary registration compiles `infra`,
	/// returning the world and the built root, or the build error.
	fn try_build(markup: &str) -> Result<(World, Entity)> {
		let mut world = (TemplatePlugin, DocumentPlugin).into_world();
		world.init_resource::<BsxConditions>();
		{
			let registry = world.resource_mut::<AppTypeRegistry>();
			registry.write().register::<PackageConfig>();
		}
		world.spawn(
			CrateRegistration::new("test_bin", "0.1.0")
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
		child_tags(&world, root).xpect_eq(vec![
			"div".to_string(),
			"p".to_string(),
		]);
	}

	/// The property the whole mechanism exists for: an excluded branch leaves NO
	/// entity, so a build-time EFFECT inside it never runs. Contrast the
	/// unregistered-tag path, which spawns an inert entity and builds children,
	/// and `RequireFeatures`, which builds the subtree whole.
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
		world.entity(root).get::<Children>().unwrap().len().xpect_eq(1);
		world
			.resource::<PackageConfig>()
			.title
			.as_str()
			.xpect_eq("Applied");
		// unmet: no entity, and the declaration never applied
		let (world, root) = build(&markup("feature:lambda"));
		world.entity(root).get::<Children>().is_none().xpect_true();
		world
			.get_resource::<PackageConfig>()
			.map(|config| config.title.to_string())
			.unwrap_or_default()
			.xpect_not_eq("Applied");
	}

	#[crate::test]
	fn operators_compose() {
		let (world, root) = build(
			r#"<a bx:cfg="feature:infra and !feature:lambda"/>
			   <b bx:cfg="feature:infra and feature:lambda"/>
			   <i bx:cfg="feature:lambda or feature:infra"/>
			   <u bx:cfg="!(feature:infra or feature:lambda)"/>"#,
		);
		child_tags(&world, root)
			.xpect_eq(vec!["a".to_string(), "i".to_string()]);
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
				world
					.entity(attr)
					.get::<Attribute>()
					.unwrap()
					.to_string()
			})
			.collect::<Vec<_>>();
		attrs.xpect_eq(vec!["id".to_string()]);
	}
}
