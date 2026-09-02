//! The request/response wire a bridged script speaks, and the host-side
//! executor that answers it.
//!
//! One call in, one reply out, correlated by [`WorldCall::id`]: the script
//! awaits a promise, the host performs the operation against the live
//! [`World`], and the reply settles that promise. Every operation is served the
//! moment it is asked for, so a script reads its own writes.
use crate::prelude::*;
use beet_core::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Map as JsonMap;
use serde_json::Value as JsonValue;

/// One `world` call, as it left the sandbox.
///
/// ```json
/// { "id": 3, "op": "insert", "entity": "42v1", "component": "Name", "value": "ada" }
/// ```
///
/// The shapes are deliberately loose: a component is named by a string, a
/// component's value is arbitrary JSON. Which JSON a particular component
/// accepts is that component's business, not the wire's.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorldCall {
	/// Correlates this call with its [`WorldReply`]. Assigned by the shim,
	/// monotonic within one evaluation.
	pub id: u64,
	/// What the script asked for.
	#[serde(flatten)]
	pub op: WorldOp,
}

/// The operations a script can ask of the world.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum WorldOp {
	/// `await world.get(entity, component)`, replying with the component's JSON
	/// form, or with no value when the entity does not carry it.
	Get {
		/// The entity, in [`entity_id`](super::entity_id) form.
		entity: String,
		/// The component identifier.
		component: String,
	},
	/// `await world.entities(component)`, replying with the ids of every entity
	/// carrying it.
	Entities {
		/// The component identifier.
		component: String,
	},
	/// `await world.schema(component)`, replying with a loose structural
	/// description.
	Schema {
		/// The component identifier.
		component: String,
	},
	/// `await world.spawn(components)`, replying with the new entity's id.
	Spawn {
		/// The components to spawn it with, keyed by identifier.
		components: JsonMap<String, JsonValue>,
	},
	/// `await world.insert(entity, component, value)`, replying with no value.
	Insert {
		/// The entity, in [`entity_id`](super::entity_id) form.
		entity: String,
		/// The component identifier.
		component: String,
		/// The component's value.
		value: JsonValue,
	},
	/// `await world.remove(entity, component)`, replying with no value.
	Remove {
		/// The entity, in [`entity_id`](super::entity_id) form.
		entity: String,
		/// The component identifier.
		component: String,
	},
	/// `await world.despawn(entity)`, replying with no value.
	Despawn {
		/// The entity, in [`entity_id`](super::entity_id) form.
		entity: String,
	},
}

/// The host's answer to one [`WorldCall`].
///
/// ```json
/// { "status": "ok", "id": 3 }
/// { "status": "err", "id": 3, "message": "script may not write `Name`" }
/// ```
///
/// An [`Ok`](Self::Ok) with no value settles the promise with `undefined`,
/// which is how a void operation and an absent component both read in the
/// script. An [`Err`](Self::Err) *rejects* the promise, so a refused write is an
/// error the script can catch at the call site rather than a silent no-op.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum WorldReply {
	/// The operation succeeded.
	Ok {
		/// The call this answers.
		id: u64,
		/// What it produced, absent for a void operation.
		#[serde(default, skip_serializing_if = "Option::is_none")]
		value: Option<JsonValue>,
	},
	/// The operation failed, with the host's flattened message.
	Err {
		/// The call this answers.
		id: u64,
		/// Why it failed, as the script's `Error` message.
		message: String,
	},
}

impl WorldReply {
	/// The reply for a call that produced `value`, or failed.
	///
	/// A failure carries the error's first line only. A [`BevyError`] renders
	/// its host-side location and backtrace after the message, and none of that
	/// means anything inside the sandbox: the script is about to see this as an
	/// `Error` it may print or answer with. Every message the bridge raises is
	/// one line by construction.
	fn new(id: u64, result: Result<Option<JsonValue>>) -> Self {
		match result {
			Ok(value) => Self::Ok { id, value },
			Err(err) => Self::Err {
				id,
				message: err
					.to_string()
					.lines()
					.next()
					.unwrap_or_default()
					.to_string(),
			},
		}
	}
}

impl WorldCall {
	/// Perform this call against the live world under `exposure`.
	///
	/// Infallible by construction: a failure is a [`WorldReply::Err`] the script
	/// can catch, never a failure of the run. A script asking for something it
	/// may not have is the sandbox working, not the host breaking.
	pub fn execute(
		self,
		world: &mut World,
		exposure: &ScriptExposure,
	) -> WorldReply {
		WorldReply::new(self.id, self.op.execute(world, exposure))
	}
}

impl WorldOp {
	/// Perform this operation, producing its reply value.
	fn execute(
		self,
		world: &mut World,
		exposure: &ScriptExposure,
	) -> Result<Option<JsonValue>> {
		match self {
			Self::Get { entity, component } => WorldRead::get(
				world,
				entity_id::decode(&entity)?,
				&component,
				exposure,
			),
			Self::Entities { component } => {
				WorldRead::entities(world, &component, exposure).map(Some)
			}
			Self::Schema { component } => {
				WorldRead::schema(world, &component, exposure).map(Some)
			}
			Self::Spawn { components } => {
				WorldWrite::spawn(world, &components, exposure)?
					.xmap(entity_id::encode)
					.xmap(JsonValue::String)
					.xmap(Some)
					.xok()
			}
			Self::Insert {
				entity,
				component,
				value,
			} => WorldWrite::insert(
				world,
				entity_id::decode(&entity)?,
				&component,
				&value,
				exposure,
			)
			.map(|_| None),
			Self::Remove { entity, component } => WorldWrite::remove(
				world,
				entity_id::decode(&entity)?,
				&component,
				exposure,
			)
			.map(|_| None),
			Self::Despawn { entity } => {
				WorldWrite::despawn(world, entity_id::decode(&entity)?)
					.map(|_| None)
			}
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use crate::scripting::dynamic::test_support::*;
	use beet_core::prelude::*;

	/// Serve one call against a fresh world, the shape every bridged operation
	/// takes.
	fn serve(world: &mut World, call: &str) -> WorldReply {
		serde_json::from_str::<WorldCall>(call)
			.unwrap()
			.execute(world, &ScriptExposure::default())
	}

	/// The headline capability: a write lands before the read that follows it,
	/// because both are served against the same live world.
	#[beet_core::test]
	fn a_read_sees_the_write_before_it() {
		let mut world = test_world();
		let entity = world.spawn_empty().id();
		serve(
			&mut world,
			&format!(
				r#"{{"id":0,"op":"insert","entity":"{entity}","component":"Name","value":"ada"}}"#
			),
		);
		serve(
			&mut world,
			&format!(
				r#"{{"id":1,"op":"get","entity":"{entity}","component":"Name"}}"#
			),
		)
		.xpect_eq(WorldReply::Ok {
			id: 1,
			value: Some(serde_json::json!("ada")),
		});
	}

	#[beet_core::test]
	fn a_spawn_replies_with_a_usable_id() {
		let mut world = test_world();
		let WorldReply::Ok {
			value: Some(serde_json::Value::String(id)),
			..
		} = serve(
			&mut world,
			r#"{"id":0,"op":"spawn","components":{"Name":"ada"}}"#,
		)
		else {
			panic!("spawn did not reply with an id");
		};
		only_name(&mut world).xpect_eq(Name::new("ada"));
		serve(
			&mut world,
			&format!(
				r#"{{"id":1,"op":"get","entity":"{id}","component":"Name"}}"#
			),
		)
		.xpect_eq(WorldReply::Ok {
			id: 1,
			value: Some(serde_json::json!("ada")),
		});
	}

	/// A component the entity does not carry replies with no value, which the
	/// shim settles as `undefined`.
	#[beet_core::test]
	fn an_absent_component_replies_with_no_value() {
		let mut world = test_world();
		let entity = world.spawn_empty().id();
		serve(
			&mut world,
			&format!(
				r#"{{"id":0,"op":"get","entity":"{entity}","component":"Name"}}"#
			),
		)
		.xpect_eq(WorldReply::Ok { id: 0, value: None });
	}

	#[beet_core::test]
	fn a_void_operation_replies_with_no_value() {
		let mut world = test_world();
		let entity = world.spawn(Name::new("ada")).id();
		serve(
			&mut world,
			&format!(r#"{{"id":4,"op":"despawn","entity":"{entity}"}}"#),
		)
		.xpect_eq(WorldReply::Ok { id: 4, value: None });
		entity_count(&world).xpect_eq(0);
	}

	/// A refusal is an error reply carrying the call's id, so the shim rejects
	/// exactly the promise that asked.
	#[beet_core::test]
	fn a_failure_replies_with_the_calls_id() {
		let mut world = test_world();
		let WorldReply::Err { id, message } = serve(
			&mut world,
			r#"{"id":9,"op":"get","entity":"nope","component":"Name"}"#,
		) else {
			panic!("a malformed entity should not have succeeded");
		};
		id.xpect_eq(9);
		message.xpect_contains("is not an entity id");
	}

	/// The wire tags are written by hand in the shared JS shim, so they are
	/// pinned here the way `ScriptEvent`'s are.
	#[beet_core::test]
	fn wire_tags_are_stable() {
		serde_json::to_string(&WorldCall {
			id: 1,
			op: WorldOp::Remove {
				entity: "42v1".to_string(),
				component: "Name".to_string(),
			},
		})
		.unwrap()
		.xpect_eq(
			r#"{"id":1,"op":"remove","entity":"42v1","component":"Name"}"#,
		);
		serde_json::to_string(&WorldReply::Ok { id: 1, value: None })
			.unwrap()
			.xpect_eq(r#"{"status":"ok","id":1}"#);
		serde_json::to_string(&WorldReply::Err {
			id: 1,
			message: "boom".to_string(),
		})
		.unwrap()
		.xpect_eq(r#"{"status":"err","id":1,"message":"boom"}"#);
	}
}
