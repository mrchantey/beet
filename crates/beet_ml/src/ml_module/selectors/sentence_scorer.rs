use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::prelude::*;
use std::borrow::Cow;

/// This component is for use with the [`SentenceScorer`]. Add to either the agent or a child behavior.
#[derive(Debug, Clone, Component, PartialEq, Reflect)]
#[reflect(Component)]
pub struct Sentence(pub Cow<'static, str>);
impl Sentence {
	pub fn new(s: impl Into<Cow<'static, str>>) -> Self { Self(s.into()) }
}

/// This selector uses [`Bert`] to compare the [`SentenceOption`] attached to the agent
/// with those on child behaviors. It does
/// This should be used with the [`UtilitySelector`]
#[derive_action]
#[action(graph_role=GraphRole::Child, child_components=[Score])]
pub struct SentenceScorer;


fn sentence_scorer(
	mut commands: Commands,
	mut bert: ResMut<Bert>,
	sentences: Query<&Sentence>,
	started: Query<(&SentenceScorer, &ParentRoot, &Children), Added<Running>>,
) {
	for (_scorer, agent, children) in started.iter() {
		let Ok(parent) = sentences.get(agent.0) else {
			continue;
		};

		let children = children
			.iter()
			.filter_map(|e| sentences.get(*e).ok().map(|s| (e, s)))
			.collect::<Vec<_>>();

		let mut options = vec![parent.0.clone()];
		options.extend(children.iter().map(|c| c.1 .0.clone()));

		let embeddings = bert.get_embeddings(options).unwrap();
		let scores = embeddings.scores(0).unwrap();
		for score in scores {
			// subtract 1 because the first index is the agent
			let entity = *children[score.0 - 1].0;
			commands.entity(entity).insert(Score::Weight(score.1));
		}
	}
}



#[cfg(test)]
mod test {
	// use crate::ml_module::ml_plugin::MlPlugin;
	use crate::prelude::*;
	use anyhow::Result;
	use beet_ecs::prelude::*;
	use bevy::prelude::*;
	use sweet::*;

	#[test]
	fn works() -> Result<()> {
		pretty_env_logger::try_init().ok();

		let mut app = App::new();
		app.add_plugins(MlPlugin::default())
			.add_plugins(BeetSystemsPlugin::<
			(SentenceScorer, ScoreSelector),
			_,
		>::default());


		let entity = app
			.world_mut()
			.spawn(Sentence::new("destroy"))
			.with_children(|parent| {
				let id = parent.parent_entity();
				parent
					.spawn((
						ParentRoot(id),
						SentenceScorer,
						ScoreSelector {
							consume_scores: true,
						},
						Running,
					))
					.with_children(|parent| {
						parent.spawn(Sentence::new("heal"));
						parent.spawn(Sentence::new("kill"));
					});
			})
			.id();

		app.update();

		let tree = EntityTree::new_with_world(entity, app.world());

		let scores = tree.component_tree::<Score>(app.world());
		let heal_score = scores.children[0].children[0].value.unwrap();
		let kill_score = scores.children[0].children[1].value.unwrap();
		expect(kill_score).to_be_greater_than(heal_score)?;
		expect(heal_score.weight()).to_be_less_than(0.5)?;
		expect(kill_score.weight()).to_be_greater_than(0.5)?;

		expect(tree.component_tree(app.world())).to_be(
			Tree::new(None).with_child(
				Tree::new(Some(&Running)).with_leaf(None).with_leaf(None),
			),
		)?;
		app.update();
		expect(tree.component_tree(app.world())).to_be(
			Tree::new(None).with_child(
				Tree::new(Some(&Running))
					.with_leaf(None)
					.with_leaf(Some(&Running)),
			),
		)?;
		expect(tree.component_tree::<Score>(app.world())).to_be(
			Tree::new(None)
				.with_child(Tree::new(None).with_leaf(None).with_leaf(None)),
		)?;

		Ok(())
	}
}
