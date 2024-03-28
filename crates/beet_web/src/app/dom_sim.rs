use crate::prelude::*;
use anyhow::Result;
use base64::engine::general_purpose;
use base64::Engine;
use beet::prelude::*;
use bevy::ecs as bevy_ecs;
use bevy::prelude::*;
use bevy::utils::HashMap;
use flume::Receiver;
use flume::Sender;
use forky_web::AnimationFrame;
use forky_web::History;
use forky_web::SearchParams;
use std::time::Duration;
use web_sys::HtmlDivElement;

#[derive(Component)]
pub struct DomSimEntity;

#[derive(Clone, Default, Deref, DerefMut)]
pub struct DomSimElements(pub HashMap<Entity, HtmlDivElement>);

pub struct DomSim {
	pub graph: DynGraph,
	pub auto_flowers: Option<Duration>,
	pub bees: usize,
	pub flowers: usize,
	pub basic_ui: bool,
}

impl Default for DomSim {
	fn default() -> Self {
		Self {
			graph: DynGraph::new::<BeeNode>(forage()),
			auto_flowers: None,
			basic_ui: true,
			bees: 1,
			flowers: 1,
		}
	}
}


impl DomSim {
	pub fn with_node<M>(mut self, graph: impl IntoBeetBuilder<M>) -> Self {
		self.graph = DynGraph::new::<BeeNode>(graph.into_beet_builder());
		self
	}
	pub fn with_url_params<T: ActionTypes>(mut self) -> Self {
		if let Some(bees) = SearchParams::get("bees") {
			self.bees = bees.parse().unwrap_or(1);
		}
		if let Some(flowers) = SearchParams::get("flowers") {
			self.flowers = flowers.parse().unwrap_or(1);
		}
		if let Some(auto_flowers) = SearchParams::get("auto-flowers") {
			let val: f64 = auto_flowers.parse().unwrap_or(1.0);
			self.auto_flowers = Some(Duration::from_secs_f64(val));
		}
		if let Ok(Some(graph)) = get_graph_url_param::<T>() {
			self.graph = graph;
		}
		self
	}

	pub fn run_forever(self) -> Result<()> {
		let (send, recv) = flume::unbounded();
		self.run_with_channel(send, recv)?.forget();
		Ok(())
	}


	#[must_use]
	pub fn run_with_channel(
		self,
		send: Sender<DomSimMessage>,
		recv: Receiver<DomSimMessage>,
	) -> Result<AnimationFrame> {
		for _ in 0..self.bees {
			send.send(DomSimMessage::SpawnBee)?;
		}
		for _ in 0..self.flowers {
			send.send(DomSimMessage::SpawnFlower)?;
		}

		console_error_panic_hook::set_once();
		console_log::init_with_level(log::Level::Info).ok();
		if self.basic_ui {
			setup_ui(send.clone());
		}
		let mut app = App::new();


		app /*-*/
			.add_plugins(BeetMinimalPlugin)
			.add_plugins(DefaultBeetPlugins::<BeeNode>::new())
			.insert_resource(DomSimMessageSend(send))
			.insert_resource(DomSimMessageRecv(recv))
			.insert_resource(self.graph)
			.insert_non_send_resource(DomSimElements::default())
			.add_systems(Update,(
				message_handler.pipe(log_error),
				update_positions
			).chain()
			)
			.add_systems(Update, despawn_elements)
		/*-*/;

		if let Some(duration) = self.auto_flowers {
			app.insert_resource(AutoFlowers(Timer::new(
				duration,
				TimerMode::Repeating,
			)))
			.add_systems(Update, auto_flowers_spawn);
		}

		let frame = AnimationFrame::new(move || {
			app.update();
		});

		Ok(frame)
	}
}

pub fn get_graph_url_param<T: ActionTypes>() -> Result<Option<DynGraph>> {
	if let Some(tree) = SearchParams::get("graph") {
		let bytes = general_purpose::STANDARD_NO_PAD.decode(tree.as_bytes())?;
		let serde: DynGraphSerde<T> = bincode::deserialize(&bytes)?;
		Ok(Some(serde.into_dyn_graph()?))
	} else {
		Ok(None)
	}
}


const MAX_URL_LENGTH: usize = 1900;
pub fn set_graph_url_param<T: ActionTypes>(graph: &DynGraph) -> Result<()> {
	let serde = graph.into_serde::<T>();
	let val = bincode::serialize(&serde).unwrap();
	let val = general_purpose::STANDARD_NO_PAD.encode(val);
	if val.len() > MAX_URL_LENGTH {
		anyhow::bail!(
			"graph base64 length is too long: {} > {}",
			val.len(),
			MAX_URL_LENGTH
		);
	}
	History::set_param("graph", &val);
	Ok(())
}

fn log_error(val: In<Result<()>>) {
	if let Err(e) = val.0 {
		log::error!("{e}");
	}
}
