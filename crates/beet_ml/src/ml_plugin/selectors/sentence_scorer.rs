use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::ecs::schedule::SystemConfigs;
use bevy::prelude::*;
use forky_core::ResultTEExt;
use std::borrow::Cow;

/// This component is for use with the [`SentenceScorer`]. Add to either the agent or a child behavior.
#[derive(Debug, Clone, Component, PartialEq, Reflect)]
#[reflect(Component)]
pub struct Sentence(pub Cow<'static, str>);
impl Sentence {
	pub fn new(s: impl Into<Cow<'static, str>>) -> Self { Self(s.into()) }
}

/// Updates the [`Score`] of each child based on the similarity of its [`Sentence`] with the agent,
/// for use with [`ScoreSelector`]
#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[reflect(Component, ActionMeta)]
pub struct SentenceScorer {
	pub bert: Handle<Bert>,
}

impl SentenceScorer {
	pub fn new(bert: Handle<Bert>) -> Self { Self { bert } }
}

fn sentence_scorer(
	mut commands: Commands,
	mut berts: ResMut<Assets<Bert>>,
	sentences: Query<&Sentence>,
	// TODO double query, ie added running and added asset
	started: Query<(&SentenceScorer, &TargetAgent, &Children), With<Running>>,
) {
	for (scorer, agent, children) in started.iter() {
		let Some(bert) = berts.get_mut(&scorer.bert) else {
			continue;
		};

		let children = children.into_iter().cloned().collect::<Vec<_>>();
		//TODO: VERY EXPENSIVE
		bert.score_sentences(agent.0, children, &sentences)
			.ok_or(|e| log::error!("{e}"))
			.map(|scores| {
				for (entity, _, score) in scores {
					commands.entity(entity).insert(Score::Weight(score));
				}
			});
	}
}

impl ActionMeta for SentenceScorer {
	fn category(&self) -> ActionCategory { ActionCategory::ChildBehaviors }
}

impl ActionSystems for SentenceScorer {
	fn systems() -> SystemConfigs { sentence_scorer.in_set(TickSet) }
}

#[cfg(test)]
mod test {
	// use crate::ml_module::ml_plugin::MlPlugin;
	use crate::prelude::*;
	use anyhow::Result;
	use beet_ecs::prelude::*;
	use bevy::prelude::*;
	use sweet::*;

	fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
		commands
			.spawn(Sentence::new("destroy"))
			.with_children(|parent| {
				let id = parent.parent_entity();
				parent
					.spawn((
						TargetAgent(id),
						SentenceScorer::new(
							asset_server.load("default-bert.ron"),
						),
						ScoreSelector {
							consume_scores: true,
						},
						Running,
					))
					.with_children(|parent| {
						parent.spawn(Sentence::new("heal"));
						parent.spawn(Sentence::new("kill"));
					});
			});
	}


	#[test]
	fn works() -> Result<()> {
		pretty_env_logger::try_init().ok();

		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			AssetPlugin::default(),
			MlPlugin::default(),
			LifecyclePlugin,
		))
		.add_systems(Startup, setup)
		.finish();

		let entity = loop {
			app.update();
			let scorer = app
				.world_mut()
				.query::<&SentenceScorer>()
				.iter(app.world())
				.next()
				.unwrap();

			if app
				.world()
				.get_resource::<Assets<Bert>>()
				.unwrap()
				.get(&scorer.bert)
				.is_some()
			{
				break app
					.world_mut()
					.query_filtered::<Entity, (Without<Parent>, With<Sentence>)>(
					)
					.iter(app.world())
					.next()
					.unwrap();
			}
			std::thread::sleep(std::time::Duration::from_millis(1));
		};

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
		// why was this here?
		// expect(tree.component_tree::<Score>(app.world())).to_be(
		// 	Tree::new(None)
		// 		.with_child(Tree::new(None).with_leaf(None).with_leaf(None)),
		// )?;

		Ok(())
	}
}
