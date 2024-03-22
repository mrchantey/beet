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
}

impl Default for DomSim {
	fn default() -> Self {
		Self {
			graph: DynGraph::new::<BeeNode>(forage()),
			auto_flowers: None,
			bees: 1,
			flowers: 1,
		}
	}
}


impl DomSim {
	pub fn with_graph<M>(mut self, graph: impl IntoBeetNode<M>) -> Self {
		self.graph = DynGraph::new::<BeeNode>(graph.into_beet_node());
		self
	}
	pub fn run(self) -> Result<()> {
		let (send, recv) = flume::unbounded();
		self.run_with_channel(send, recv)
	}
	pub fn from_url() -> Self {
		let mut this = Self::default();
		if let Some(bees) = SearchParams::get("bees") {
			this.bees = bees.parse().unwrap_or(1);
		}
		if let Some(flowers) = SearchParams::get("flowers") {
			this.flowers = flowers.parse().unwrap_or(1);
		}
		if let Some(auto_flowers) = SearchParams::get("auto-flowers") {
			let val: f64 = auto_flowers.parse().unwrap_or(1.0);
			this.auto_flowers = Some(Duration::from_secs_f64(val));
		}
		if let Ok(graph) = get_prefab_url_param() {
			this.graph = graph;
		}
		this
	}



	pub fn run_with_channel(
		self,
		send: Sender<DomSimMessage>,
		recv: Receiver<DomSimMessage>,
	) -> Result<()> {
		for _ in 0..self.bees {
			send.send(DomSimMessage::SpawnBee)?;
		}
		for _ in 0..self.flowers {
			send.send(DomSimMessage::SpawnFlower)?;
		}

		console_error_panic_hook::set_once();
		console_log::init_with_level(log::Level::Info).ok();
		setup_cool_ui(send.clone());
		let mut app = App::new();


		app /*-*/
			.add_plugins(BeetMinimalPlugin)
			.add_plugins(SteeringPlugin::default())
			.add_plugins(BeetPlugin::<BeeNode>::new(Relay::default()))
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

		frame.forget();
		Ok(())
	}
}

fn get_prefab_url_param() -> Result<DynGraph> {
	if let Some(tree) = SearchParams::get("graph") {
		let _bytes =
			general_purpose::STANDARD_NO_PAD.decode(tree.as_bytes())?;
		// let prefab = bincode::deserialize(&bytes)?;
		// Ok(prefab)
		todo!("deserialize graph")
	} else {
		anyhow::bail!("no tree param found");
	}
}


fn log_error(val: In<Result<()>>) {
	if let Err(e) = val.0 {
		log::error!("{e}");
	}
}
