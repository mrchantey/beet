//! The native fallback backend: one `deno` child process per evaluation.
//!
//! Compiled when the `quickjs` feature is off, so a native build always has a
//! working [`Script`] as long as deno is installed. Slower than the embedded
//! engine (a process spawn per call), but it needs no C toolchain and no
//! recompile to change, which is why it is the dev-speed path.
//!
//! ## Isolation
//!
//! The child is `deno run --no-prompt --no-remote -q -`: deno denies every
//! permission by default, `--no-prompt` turns a denial into an immediate error
//! rather than an interactive prompt, and `--no-remote` blocks remote module
//! specifiers so a script cannot `import()` its way out. A script therefore
//! reaches no filesystem, no environment, no network and no subprocess; the
//! escape tests below assert each one.
//!
//! Note that `deno eval` is **not** usable here: it runs with implicit access to
//! all permissions, so it would silently produce an unsandboxed backend. The
//! program is fed to `deno run` on stdin instead, which restores the default
//! deny.
//!
//! ## Transport
//!
//! The [`ScriptRequest`] is embedded in the module source as a JSON string
//! literal and `JSON.parse`d at runtime, so no part of a script or its input is
//! ever spliced into JavaScript source. Events come back on stdout as
//! [`ScriptEvent`] JSON lines. Passing the request this way (rather than as an
//! argv or a second stdin write) sidesteps both the 128KB single-argument limit
//! on Linux and any ordering race with the module read.

use crate::prelude::*;
use beet_core::prelude::*;
use futures_lite::AsyncBufReadExt;
use futures_lite::AsyncReadExt;
use futures_lite::AsyncWriteExt;
use futures_lite::StreamExt;
use futures_lite::io::AsyncRead;
use futures_lite::io::BufReader;
use serde_json::Value as JsonValue;
use std::sync::Arc;

/// The message a missing `deno` produces, naming both ways forward.
const NOT_FOUND: &str = "`Script` needs the deno cli on this build: install it \
from https://deno.com, or enable the `quickjs` feature for the embedded engine \
(which needs no external runtime).";

/// This transport's `emit`: one protocol line per stdout write.
///
/// `writeSync` is not guaranteed to take the whole slice, so the write loops.
/// A short write would truncate a JSON line, which the reader then skips as
/// noise, silently losing the very event it was waiting for.
const EMIT: &str = r#"
const encoder = new TextEncoder();
const emit = (event) => {
	const bytes = encoder.encode(JSON.stringify(event) + "\n");
	let written = 0;
	while (written < bytes.length) {
		written += Deno.stdout.writeSync(bytes.subarray(written));
	}
};
"#;

/// Evaluate `request` in a sandboxed deno child, forwarding each console line to
/// `sink` as it arrives and returning the script's completion value (`None` when
/// it produced one).
///
/// Backs both [`Script::run`] and [`Script::run_console`]: the pure transform
/// ignores the console and needs the value, the console path is the reverse.
pub(crate) async fn run_deno<Sink>(
	request: ScriptRequest,
	sink: Sink,
) -> Result<Option<JsonValue>>
where
	Sink: FnMut(ConsoleStream, &str),
{
	let timeout = request.limits.timeout;
	let source = module_source(&request)?;

	let mut child = ChildProcess::new("deno")
		// see the module doc: `deno run` with no `--allow-*` denies everything,
		// where `deno eval` would grant everything.
		.with_args(["run", "--no-prompt", "--no-remote", "-q", "-"])
		// sandboxed scripts run in UTC: the child has no env access to read a
		// timezone from, so it is pinned here rather than inherited.
		.with_envs([("TZ", "UTC")])
		.with_not_found(NOT_FOUND)
		.spawn_piped()?;

	let mut stdin = child
		.take_stdin()
		.ok_or_else(|| bevyhow!("deno: child has no stdin"))?;
	let stdout = child
		.take_stdout()
		.ok_or_else(|| bevyhow!("deno: child has no stdout"))?;
	let stderr = child
		.take_stderr()
		.ok_or_else(|| bevyhow!("deno: child has no stderr"))?;

	// the deadline is the host's own, not the script's: a script cannot be
	// trusted to observe one. It covers the stdin write too, since a child that
	// wedges without draining its stdin would otherwise block here unbounded. On
	// expiry the child is killed outright (dropping the handle would too, but the
	// kill is explicit so the error is truthful).
	let exchange = async {
		// the module is the whole of stdin, so it must be closed for deno to run it.
		stdin
			.write_all(source.as_bytes())
			.await
			.map_err(|err| bevyhow!("deno: write module: {err}"))?;
		stdin
			.close()
			.await
			.map_err(|err| bevyhow!("deno: close stdin: {err}"))?;
		drop(stdin);
		read_events(stdout, stderr, sink).await
	};
	match async_ext::timeout(timeout, exchange).await {
		Ok(result) => result,
		Err(_) => {
			child.kill().ok();
			bevybail!("deno: script timed out")
		}
	}
}

/// This request as a runnable deno module: the shared runner, with the request
/// and this transport's `emit` prepended.
fn module_source(request: &ScriptRequest) -> Result<String> {
	format!("{}\n{EMIT}\n{JS_RUNNER}", request.to_js_prelude()?).xok()
}

/// Read protocol events until the terminal one, streaming console lines to
/// `sink` on the way.
///
/// stderr is drained concurrently into `diagnostics` rather than read at the
/// end. A child that writes more than a pipe buffer to stderr (`Deno.stderr`
/// needs no permission, so a script can do this deliberately) would otherwise
/// block on the write, never exit, never close stdout, and wedge this loop until
/// the deadline.
async fn read_events<Out, Err, Sink>(
	stdout: Out,
	stderr: Err,
	mut sink: Sink,
) -> Result<Option<JsonValue>>
where
	Out: Unpin + AsyncRead,
	Err: Unpin + AsyncRead,
	Sink: FnMut(ConsoleStream, &str),
{
	let diagnostics = Arc::new(RwLock::new(String::new()));
	let events = async {
		let mut lines = BufReader::new(stdout).lines();
		while let Some(line) = lines.next().await {
			let line = line.map_err(|err| bevyhow!("deno: read: {err}"))?;
			if let Some(result) = protocol::apply_event(&line, &mut sink) {
				return result.map_err(|err| bevyhow!("deno: {err}"));
			}
		}
		// stdout closed with no terminal event, so the runner never got to emit
		// one: deno itself failed, and whatever it said is on stderr.
		let message = diagnostics
			.read()
			.map(|message| message.trim().to_string())
			.unwrap_or_default();
		bevybail!("deno: exited without a result: {message}")
	};
	// never resolves, so the race is decided by the event stream alone; draining
	// is the point, not the result.
	let drain_stderr = drain(stderr, diagnostics.clone());
	futures_lite::future::or(events, drain_stderr).await
}

/// Append everything `reader` produces to `sink` as it arrives, then park.
async fn drain<Read, T>(mut reader: Read, sink: Arc<RwLock<String>>) -> T
where
	Read: Unpin + AsyncRead,
{
	let mut buffer = [0u8; 4096];
	while let Ok(read) = reader.read(&mut buffer).await
		&& read > 0
	{
		if let Ok(mut sink) = sink.write() {
			sink.push_str(&String::from_utf8_lossy(&buffer[..read]));
		}
	}
	async_ext::yield_forever().await
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	async fn transforms_its_input() {
		Script::<i64, i64>::new("input + 1").run(41).await.unwrap().xpect_eq(42);
	}

	#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
	struct Player {
		name: String,
		score: i64,
	}

	/// Structured values survive the crossing intact, not just scalars.
	#[beet_core::test]
	async fn round_trips_a_struct() {
		Script::<Player, Player>::new("input.score += 10; input")
			.run(Player {
				name: "ada".to_string(),
				score: 5,
			})
			.await
			.unwrap()
			.xpect_eq(Player {
				name: "ada".to_string(),
				score: 15,
			});
	}

	/// The prelude escapes every `<` for the iframe transport's sake; that must
	/// not corrupt the payload the runner actually parses.
	#[beet_core::test]
	async fn angle_brackets_survive_the_crossing() {
		Script::<(), String>::new(r#""a </script> b <!-- c " + (1 < 2)"#)
			.run(())
			.await
			.unwrap()
			.xpect_eq("a </script> b <!-- c true".to_string());
	}

	#[beet_core::test]
	async fn awaits_an_async_script() {
		Script::<(), i64>::new("Promise.resolve(20).then(value => value * 2)")
			.run(())
			.await
			.unwrap()
			.xpect_eq(40);
	}

	#[beet_core::test]
	async fn splits_the_console_streams() {
		Script::<(), ()>::new(r#"console.log("out"); console.error("err")"#)
			.run_captured(())
			.await
			.unwrap()
			.xpect_eq("out\n".to_string());
	}

	/// A script that floods stderr cannot wedge the exchange. `Deno.stderr` needs
	/// no permission, so a script can fill the pipe buffer deliberately; reading
	/// stderr only after stdout closed would leave the child blocked on its write
	/// and the terminal event never arriving.
	#[beet_core::test]
	async fn a_stderr_flood_does_not_wedge_the_exchange() {
		Script::<(), i64>::new(
			r#"const noise = new TextEncoder().encode("x".repeat(1024));
			for (let i = 0; i < 512; i++) Deno.stderr.writeSync(noise);
			7"#,
		)
		.run(())
		.await
		.unwrap()
		.xpect_eq(7);
	}

	#[beet_core::test]
	async fn script_errors_propagate() {
		Script::<(), ()>::new(r#"throw new Error("boom")"#)
			.run(())
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("boom");
	}

	/// The child holds no authority the script can reach: each denial is deno's
	/// default-deny answering, not a beet-side check.
	///
	/// Asserted on the `--allow-*` flag name rather than the prose, which varies
	/// with the environment (a `LD_LIBRARY_PATH` in scope changes the wording of
	/// the subprocess denial), and on `NotCapable` so a denial is never confused
	/// with some other failure.
	#[beet_core::test]
	async fn denies_every_ambient_authority() {
		let denied = [
			(r#"Deno.readTextFileSync("/etc/passwd")"#, "--allow-read"),
			(r#"Deno.env.get("HOME")"#, "--allow-env"),
			(
				r#"Deno.writeTextFileSync("/tmp/beet-escape", "x")"#,
				"--allow-write",
			),
			(r#"fetch("https://example.com")"#, "--allow-net"),
			(
				r#"new Deno.Command("sh", { args: ["-c", "echo escaped"] }).outputSync()"#,
				"--allow-run",
			),
		];
		for (source, expected) in denied {
			Script::<(), ()>::new(source)
				.run(())
				.await
				.unwrap_err()
				.to_string()
				.xpect_contains("NotCapable")
				.xpect_contains(expected);
		}
	}

	/// `--no-remote` blocks the other way out: pulling in code from the network.
	#[beet_core::test]
	async fn blocks_remote_imports() {
		Script::<(), ()>::new(r#"import("https://deno.land/std/version.ts")"#)
			.run(())
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("--no-remote");
	}

	/// A runaway script is killed at the deadline and the host survives it.
	#[beet_core::test]
	async fn runaway_scripts_are_killed_at_the_deadline() {
		Script::<(), ()>::new("while (true) {}")
			.with_limits(ScriptLimits {
				timeout: Duration::from_millis(500),
				..default()
			})
			.run(())
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("timed out");
		Script::<i64, i64>::new("input + 1")
			.run(1)
			.await
			.unwrap()
			.xpect_eq(2);
	}

	/// Sandboxed scripts run in UTC, so a date renders the same wherever the
	/// host happens to be.
	#[beet_core::test]
	async fn runs_in_utc() {
		Script::<(), String>::new("new Date(0).toISOString()")
			.run(())
			.await
			.unwrap()
			.xpect_eq("1970-01-01T00:00:00.000Z".to_string());
	}
}
