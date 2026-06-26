//! An agent equipped with a calculator toolset, so the same request/response
//! routes that served a CLI or HTTP calculator become an agent's tools.
//!
//! ```bsx
//! <div {RepeatWhileFunctionCallOutput} {CreateThread}>
//!   <div bx:ref="thread" {Thread} {Behavior}>
//!     <CreateActor name="System" kind="System">
//!       <CreatePost text="You are great at using a calculator.."/>
//!     </CreateActor>
//!     <CreateActor name="Agent" kind="Agent" {ModelStreamer{provider:Ollama, size:Large}}>
//!       <CalculatorToolset/>
//!     </CreateActor>
//!   </div>
//! </div>
//! ```
use crate::beet::prelude::*;
use beet_core::prelude::*;

/// Markup-resolvable alias for a `Sequence<(), Outcome>` behaviour: runs its
/// children in order. A bare generic `{Sequence}` tag does not resolve by name,
/// so this non-generic marker stands in (it `#[require]`s the sequence).
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[require(Sequence<(), ()>)]
pub struct Behavior;

/// The two operands a calculator tool takes.
#[derive(Reflect, serde::Deserialize, serde::Serialize)]
struct BinaryOp {
	/// The first operand.
	a: f64,
	/// The second operand.
	b: f64,
}

/// Add two numbers.
#[action(pure, route = "add")]
#[derive(Component, Reflect)]
#[reflect(Component)]
fn CalcAdd(cx: ActionContext<BinaryOp>) -> String {
	format!("{}", cx.a + cx.b)
}

/// Subtract the second number from the first.
#[action(pure, route = "subtract")]
#[derive(Component, Reflect)]
#[reflect(Component)]
fn CalcSubtract(cx: ActionContext<BinaryOp>) -> String {
	format!("{}", cx.a - cx.b)
}

/// Multiply two numbers.
#[action(pure, route = "multiply")]
#[derive(Component, Reflect)]
#[reflect(Component)]
fn CalcMultiply(cx: ActionContext<BinaryOp>) -> String {
	format!("{}", cx.a * cx.b)
}

/// Divide the first number by the second.
#[action(pure, route = "divide")]
#[derive(Component, Reflect)]
#[reflect(Component)]
fn CalcDivide(cx: ActionContext<BinaryOp>) -> String {
	format!("{}", cx.a / cx.b)
}

/// `<CalculatorToolset/>` — equips the enclosing agent with the four calculator
/// tools, nested as children so the thread query discovers them.
#[template]
pub fn CalculatorToolset() -> impl Bundle {
	children![CalcAdd, CalcSubtract, CalcMultiply, CalcDivide]
}

/// Registers the `Behavior` marker and the calculator toolset, so a thread
/// `main.bsx` declaring `<CalculatorToolset/>` resolves.
pub struct AgentExamplesPlugin;

impl Plugin for AgentExamplesPlugin {
	fn build(&self, app: &mut App) {
		app.register_type::<Behavior>()
			.register_type::<CalcAdd>()
			.register_type::<CalcSubtract>()
			.register_type::<CalcMultiply>()
			.register_type::<CalcDivide>()
			.register_template::<CalculatorToolset>();
	}
}
