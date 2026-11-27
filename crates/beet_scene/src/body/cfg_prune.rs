//! Pruning a body to this binary: every branch whose [`Cfg`] is unmet left
//! out, its tombstones in its place.
use alloc::borrow::Cow;
use beet_core::prelude::*;
use bevy::reflect::TypePath;

/// A body to prune to one binary.
pub(super) struct CfgPrune<'a> {
	body: &'a SceneBody,
}

impl<'a> CfgPrune<'a> {
	/// Prune `body`.
	pub fn new(body: &'a SceneBody) -> Self { Self { body } }

	/// The body for `world`'s binary: every branch whose [`Cfg`] is false
	/// replaced by its tombstones, evaluated against the registered
	/// conditions and the crate registrations, with the keys left out. A body
	/// declaring no condition is answered as it stands.
	pub fn prune(
		&self,
		world: &World,
	) -> Result<(Cow<'a, SceneBody>, ExcludedKeys)> {
		if !self
			.body
			.entities
			.values()
			.any(|components| Self::cfg(components).is_some())
		{
			return Ok((Cow::Borrowed(self.body), default()));
		}
		// cloned out so the handlers may borrow the world
		let conditions = world
			.get_resource::<BsxConditions>()
			.map(BsxConditions::clone_seam)
			.unwrap_or_default();
		let registrations = world
			.try_query::<&CrateRegistration>()
			.map(|mut query| query.iter(world).cloned().collect::<Vec<_>>())
			.unwrap_or_default();
		let cx = ConditionCx {
			world,
			registrations: &registrations,
		};
		let children = self.owned();
		let mut body = SceneBody {
			resources: self.body.resources.clone(),
			entities: default(),
		};
		let mut excluded = ExcludedKeys::default();
		// in document order, so a branch is left out before anything in it is
		// answered
		for (key, components) in &self.body.entities {
			if excluded.contains_key(key) {
				continue;
			}
			let Some(cfg) = Self::cfg(components) else {
				body.entities.insert(key.clone(), components.clone());
				continue;
			};
			let condition = BuildCondition::parse(&cfg.cfg)
				.map_err(|err| bevyhow!("in `<{} bx:cfg>`: {err}", cfg.tag))?;
			if condition.evaluate(&conditions, &cx)? {
				body.entities.insert(key.clone(), components.clone());
				continue;
			}
			debug!(
				"`bx:cfg` excluded `<{}>`: `{}` is false in this binary",
				cfg.tag, cfg.cfg
			);
			for key in Self::branch(key, &children) {
				excluded.insert(key, cfg.cfg.clone());
			}
			// a branch's first tombstone stands at its root, the rest beside it
			let parent = components
				.get(ChildOf::type_path())
				.ok()
				.cloned()
				.unwrap_or_else(|| key.reference());
			for (index, tombstone) in self
				.tombstones(key, &cfg, &children)
				.into_iter()
				.enumerate()
			{
				let standing = match index {
					0 => key.clone(),
					index => EntityKey::parse(&format!("{key}/#cfg.{index}"))?,
				};
				let mut entity =
					EntityBody::default().with_typed(&tombstone)?;
				if index > 0 || components.contains(ChildOf::type_path()) {
					entity.insert(ChildOf::type_path(), parent.clone());
				}
				body.entities.insert(standing, entity);
			}
		}
		Ok((Cow::Owned(body), excluded))
	}

	/// The condition an entity's components carry, by type path or the name a
	/// document lowered with no registry wrote.
	fn cfg(components: &EntityBody) -> Option<Cfg> {
		let value = components
			.get(Cfg::type_path())
			.or_else(|_| components.get("Cfg"))
			.ok()?;
		Cfg {
			cfg: Self::str_field(value, "cfg").unwrap_or_default(),
			tag: Self::str_field(value, "tag").unwrap_or_default(),
		}
		.xsome()
	}

	/// The tombstones a branch leaves: one per addressable entity on its
	/// frontier, the outermost ones holding a `path`, each ending its own
	/// descent, else one for the branch itself.
	///
	/// A tombstone is only useful to a host if it can be addressed, which for
	/// `beet_router` means the entity declared a `path`. Excluding a `<Route>`
	/// directly gives one; excluding a `<Fragment>` that groups several routes
	/// would give a pathless one and lose every route under it. Reading the
	/// body to find the frontier runs no effect: it is spawning an excluded
	/// entity that must not happen, and none is.
	fn tombstones(
		&self,
		key: &EntityKey,
		cfg: &Cfg,
		children: &HashMap<EntityKey, Vec<EntityKey>>,
	) -> Vec<CfgExcluded> {
		let mut found = Vec::new();
		let mut stack = vec![key.clone()];
		while let Some(next) = stack.pop() {
			match self.addressed(&next) {
				Some((tag, path)) => found.push(CfgExcluded {
					condition: cfg.cfg.clone(),
					tag: match next == *key && !cfg.tag.is_empty() {
						true => cfg.tag.clone(),
						false => tag,
					},
					path,
				}),
				None => stack.extend(
					children
						.get(&next)
						.into_iter()
						.flat_map(|children| children.iter().rev().cloned()),
				),
			}
		}
		if found.is_empty() {
			found.push(CfgExcluded {
				condition: cfg.cfg.clone(),
				tag: cfg.tag.clone(),
				path: default(),
			});
		}
		found
	}

	/// The component of the entity keyed `key` declaring a `path`, by its
	/// short name, and the path, if any does.
	fn addressed(&self, key: &EntityKey) -> Option<(SmolStr, SmolStr)> {
		self.body
			.entities
			.get(key)?
			.iter()
			.find_map(|(name, value)| {
				Self::str_field(value, "path")
					.map(|path| (TypeName::new(name.as_str()).short(), path))
			})
	}

	/// The children of each entity, in document order.
	fn owned(&self) -> HashMap<EntityKey, Vec<EntityKey>> {
		let mut children = HashMap::<EntityKey, Vec<EntityKey>>::default();
		for (key, components) in &self.body.entities {
			for owner in [ChildOf::type_path(), AttributeOf::type_path()] {
				if let Some(owner) =
					components.get(owner).ok().and_then(EntityKey::of_reference)
				{
					children.entry(owner).or_default().push(key.clone());
				}
			}
		}
		children
	}

	/// The key `root` and every key under it.
	fn branch(
		root: &EntityKey,
		children: &HashMap<EntityKey, Vec<EntityKey>>,
	) -> Vec<EntityKey> {
		let mut found = Vec::new();
		let mut stack = vec![root.clone()];
		while let Some(key) = stack.pop() {
			stack.extend(children.get(&key).into_iter().flatten().cloned());
			found.push(key);
		}
		found
	}

	/// The string `field` of a component's value.
	fn str_field(value: &Value, field: &str) -> Option<SmolStr> {
		value
			.as_map()
			.ok()?
			.get(field)
			.ok()?
			.as_str()
			.ok()
			.map(SmolStr::new)
	}
}

/// The keys an exclusion took with it, each to the condition that excluded it.
/// A reference to one binds to an entity nothing builds, the one SILENT outcome
/// of exclusion: the site's analytics store was declared under an excluded
/// `<Stack>`, so the served binary bound `StoreRef($analytics)` to nothing and
/// dropped every event with no error anywhere. The reflection names it instead.
pub(super) type ExcludedKeys = HashMap<EntityKey, SmolStr>;

#[cfg(test)]
mod test {
	use super::CfgPrune;
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// An exclusion reports the keys it took with it, so a `$name` that
	/// reaches for one can be named rather than silently binding to nothing.
	#[beet_core::test]
	fn prune_records_the_keys_it_excludes() {
		let mut world = World::new();
		world.init_resource::<BsxConditions>();
		let body = BsxParser::body(
			&world,
			r#"<div>
				<Stack bx:cfg="feature:nope">
					<Store bx:ref="analytics"/>
				</Stack>
				<Store bx:ref="kept"/>
			</div>"#,
		)
		.unwrap();
		let (pruned, excluded) = CfgPrune::new(&body).prune(&world).unwrap();
		excluded[&EntityKey::parse("analytics").unwrap()]
			.as_str()
			.xpect_eq("feature:nope");
		excluded
			.contains_key(&EntityKey::parse("kept").unwrap())
			.xpect_false();
		pruned
			.entities
			.contains_key(&EntityKey::parse("kept").unwrap())
			.xpect_true();
	}

	/// Build `markup` into a world whose primary registration compiles `infra`
	/// at `1.2.3`, returning the world and the built root, or the build error.
	fn try_build(markup: &str) -> Result<(World, Entity)> {
		let mut world = (ScenePlugin, DocumentPlugin).into_world();
		world.init_resource::<BsxConditions>();
		{
			let registry = world.resource_mut::<AppTypeRegistry>();
			let mut registry = registry.write();
			registry.register::<PackageConfig>();
			registry.register::<CfgExcluded>();
		}
		world.spawn(
			CrateRegistration::new(
				"test_bin",
				Version::parse("1.2.3").unwrap(),
			)
			.with_feature("infra")
			.with_skip_prefix(),
		);
		let body = BsxParser::body(&world, markup)?;
		let root = world.spawn_template(body)?.id();
		world.flush();
		Ok((world, root))
	}

	/// Build `markup` as the children of a root element.
	fn build(markup: &str) -> (World, Entity) {
		try_build(&format!("<main>{markup}</main>")).unwrap()
	}

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

	#[beet_core::test]
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
	#[beet_core::test]
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
		world
			.entity(tombstone)
			.get::<Children>()
			.is_none()
			.xpect_true();
		let excluded = world.entity(tombstone).get::<CfgExcluded>().unwrap();
		excluded.tag.as_str().xpect_eq("div");
		excluded.condition.as_str().xpect_eq("feature:lambda");
	}

	/// A route-shaped exclusion keeps its path, which is what lets a host report
	/// `/site` as excluded rather than unknown.
	#[beet_core::test]
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
	#[beet_core::test]
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
				<Route path="site"><Instance document="infra/site.bsx"/></Route>
				<Route path="mail"><Instance document="infra/mail.bsx"/></Route>
			</Fragment>"#,
		);
		let each = paths(
			r#"<Route path="site" bx:cfg="feature:lambda"><Instance document="infra/site.bsx"/></Route>
			   <Route path="mail" bx:cfg="feature:lambda"><Instance document="infra/mail.bsx"/></Route>"#,
		);
		grouped.xpect_eq(vec!["mail".to_string(), "site".to_string()]);
		grouped.xpect_eq(each);
	}

	/// An excluded branch with nothing addressable under it still records the
	/// exclusion, just with no path for a host to report at.
	#[beet_core::test]
	fn a_pathless_exclusion_is_still_recorded() {
		let (world, root) = build(
			r#"<Fragment bx:cfg="feature:lambda"><div/><span/></Fragment>"#,
		);
		let children = world.entity(root).get::<Children>().unwrap();
		children.len().xpect_eq(1);
		let excluded = world.entity(children[0]).get::<CfgExcluded>().unwrap();
		excluded.tag.as_str().xpect_eq("Fragment");
		excluded.path.is_empty().xpect_true();
	}

	#[beet_core::test]
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
	#[beet_core::test]
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
	#[beet_core::test]
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
	#[beet_core::test]
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
}
