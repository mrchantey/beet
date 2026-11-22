//! The one way a process asks its operator for something.
use crate::prelude::*;
use alloc::sync::Arc;
use std::sync::RwLock;

/// The seam every ask of the operator goes through: a passphrase, a token,
/// an mfa code, a url to open. [`ask`](Self::ask) hands an [`OperatorAsk`]
/// to the installed [`OperatorPromptProvider`], else the one this target
/// and launch imply:
///
/// - [`TerminalPrompt`] on a unix machine with a controlling terminal,
///   reading with echo off and printing notices to the tty;
/// - [`RefusalPrompt`] anywhere no person can answer: a `--headless` launch,
///   a pipe or CI with no terminal, a Worker, a browser. Its error names
///   what to pass instead ([`OperatorAsk::with_instead`]).
///
/// Another surface (ie a UI dialog) needs only a provider of its own,
/// [`install`](Self::install)ed at startup. No scene declares one: the
/// operator is a property of the process, not of a document.
///
/// ## A code is a person's
///
/// A one-time code ([`OperatorAskKind::Code`]) is the one thing an automated
/// process cannot supply, which is why a role's trust policy conditions on
/// it. So the seam asks a code only of an
/// [`interactive`](OperatorPromptProvider::interactive) provider, and the
/// refusal provider is not one: no flag, variable or configuration of it
/// answers a code.
pub struct OperatorPrompt;

impl OperatorPrompt {
	/// Answer every later ask through `provider`, ie a UI's dialog.
	pub fn install(provider: impl OperatorPromptProvider) {
		*Self::installed()
			.write()
			.unwrap_or_else(|err| err.into_inner()) = Some(Arc::new(provider));
	}

	/// Ask the operator, through the installed provider else the target's.
	pub async fn ask(ask: OperatorAsk) -> Result<String> {
		Self::ask_with(Self::provider().as_ref(), &ask).await
	}

	/// Ask through `provider`, a code refused when it is not interactive.
	pub async fn ask_with(
		provider: &dyn OperatorPromptProvider,
		ask: &OperatorAsk,
	) -> Result<String> {
		if matches!(ask.kind, OperatorAskKind::Code { .. })
			&& !provider.interactive()
		{
			return RefusalPrompt::new(format!(
				"{} answers without a person",
				provider.describe()
			))
			.ask(ask)
			.await;
		}
		provider.ask(ask).await
	}

	fn installed() -> &'static RwLock<Option<Arc<dyn OperatorPromptProvider>>> {
		static INSTALLED: RwLock<Option<Arc<dyn OperatorPromptProvider>>> =
			RwLock::new(None);
		&INSTALLED
	}

	/// The installed provider, else the one this target and launch imply.
	fn provider() -> Arc<dyn OperatorPromptProvider> {
		if let Some(provider) = Self::installed()
			.read()
			.unwrap_or_else(|err| err.into_inner())
			.clone()
		{
			return provider;
		}
		if BootstrapConfig::get().headless {
			return Arc::new(RefusalPrompt::new("this launch is headless"));
		}
		cfg_if! {
			if #[cfg(all(unix, not(target_arch = "wasm32")))] {
				match terminal_ext::tty_writer() {
					Ok(_) => Arc::new(TerminalPrompt),
					Err(err) => Arc::new(RefusalPrompt::new(format!(
						"there is no terminal to ask on ({err})"
					))),
				}
			} else {
				Arc::new(RefusalPrompt::new("this host has no terminal to ask on"))
			}
		}
	}
}

/// One ask of the operator: what is asked, the prompt shown, its kind, and
/// what to do instead where nobody can answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorAsk {
	/// What is asked, for an error, ie `a backup passphrase`.
	subject: SmolStr,
	/// The prompt or notice shown.
	message: String,
	/// What is asked for.
	kind: OperatorAskKind,
	/// What to pass or run instead when no person can answer.
	instead: Option<String>,
}

/// What an [`OperatorAsk`] asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatorAskKind {
	/// A secret line, typed without echo: a passphrase, a token.
	SecretLine,
	/// A one-time code of `digits` digits from an authenticator, typed
	/// without echo, which only a person can supply.
	Code {
		/// How many digits the code has.
		digits: usize,
	},
	/// Something to act on, ie "open this url and confirm this code",
	/// answered with an empty line once shown.
	Notice,
}

impl OperatorAsk {
	/// A secret line, `subject` named in errors, `message` the prompt.
	pub fn secret_line(
		subject: impl Into<SmolStr>,
		message: impl Into<String>,
	) -> Self {
		Self::new(subject, message, OperatorAskKind::SecretLine)
	}

	/// A one-time code of `digits` digits.
	pub fn code(
		subject: impl Into<SmolStr>,
		message: impl Into<String>,
		digits: usize,
	) -> Self {
		Self::new(subject, message, OperatorAskKind::Code { digits })
	}

	/// A notice to act on.
	pub fn notice(
		subject: impl Into<SmolStr>,
		message: impl Into<String>,
	) -> Self {
		Self::new(subject, message, OperatorAskKind::Notice)
	}

	fn new(
		subject: impl Into<SmolStr>,
		message: impl Into<String>,
		kind: OperatorAskKind,
	) -> Self {
		Self {
			subject: subject.into(),
			message: message.into(),
			kind,
			instead: None,
		}
	}

	/// Name what to pass or run instead where no person can answer, ie
	/// `pass the value with --value`.
	pub fn with_instead(mut self, instead: impl Into<String>) -> Self {
		self.instead = Some(instead.into());
		self
	}

	/// What is asked.
	pub fn subject(&self) -> &str { &self.subject }
	/// The prompt or notice shown.
	pub fn message(&self) -> &str { &self.message }
	/// What is asked for.
	pub fn kind(&self) -> OperatorAskKind { self.kind }
	/// What to do instead, where nobody can answer.
	pub fn instead(&self) -> Option<&str> { self.instead.as_deref() }
}

/// One way of answering an [`OperatorAsk`], see [`OperatorPrompt`].
pub trait OperatorPromptProvider: 'static + Send + Sync {
	/// What answers, for an error, ie `the terminal`.
	fn describe(&self) -> String;

	/// Whether a person answers, the one kind of provider a code is asked
	/// of.
	fn interactive(&self) -> bool;

	/// The answer, an empty string once a notice is shown.
	fn ask(&self, ask: &OperatorAsk) -> SendBoxedFuture<Result<String>>;
}

/// The controlling terminal: a secret or a code read from `/dev/tty` with
/// echo off, the prompt and a newline after the entry written to the tty, so
/// the line never reaches stdout or the scrollback; a notice printed there.
#[cfg(all(unix, not(target_arch = "wasm32")))]
pub struct TerminalPrompt;

#[cfg(all(unix, not(target_arch = "wasm32")))]
impl TerminalPrompt {
	/// Write `message` to the tty and read one line with echo off.
	fn read_secret_line(message: &str) -> Result<String> {
		use std::io::BufRead;
		use std::io::Write;
		use std::os::unix::io::AsRawFd;
		let mut tty = terminal_ext::tty_writer()?;
		tty.write_all(message.as_bytes())?;
		tty.flush()?;
		let input = std::fs::File::open("/dev/tty")?;
		let fd = input.as_raw_fd();
		// SAFETY: a zeroed termios is a valid out-param for tcgetattr, and the
		// restored attributes are the ones just read.
		let mut original: libc::termios = unsafe { core::mem::zeroed() };
		if unsafe { libc::tcgetattr(fd, &mut original) } != 0 {
			bevybail!("tcgetattr: {}", std::io::Error::last_os_error());
		}
		let mut quiet = original;
		quiet.c_lflag &= !libc::ECHO;
		if unsafe { libc::tcsetattr(fd, libc::TCSANOW, &quiet) } != 0 {
			bevybail!("tcsetattr: {}", std::io::Error::last_os_error());
		}
		let mut line = String::new();
		let read = std::io::BufReader::new(&input).read_line(&mut line);
		// echo comes back whatever the read did
		unsafe { libc::tcsetattr(fd, libc::TCSANOW, &original) };
		tty.write_all(b"\n")?;
		read?;
		line.trim_end_matches(['\r', '\n']).to_string().xok()
	}

	/// Write a notice to the tty.
	fn notify(message: &str) -> Result<String> {
		use std::io::Write;
		let mut tty = terminal_ext::tty_writer()?;
		tty.write_all(message.as_bytes())?;
		tty.write_all(b"\n")?;
		tty.flush()?;
		String::new().xok()
	}
}

#[cfg(all(unix, not(target_arch = "wasm32")))]
impl OperatorPromptProvider for TerminalPrompt {
	fn describe(&self) -> String { "the terminal".into() }

	fn interactive(&self) -> bool { true }

	fn ask(&self, ask: &OperatorAsk) -> SendBoxedFuture<Result<String>> {
		let ask = ask.clone();
		Box::pin(async move {
			// the read blocks its thread until the operator answers, so it
			// runs on the blocking pool rather than the caller's
			async_ext::unblock(move || match ask.kind {
				OperatorAskKind::Notice => Self::notify(&ask.message),
				_ => Self::read_secret_line(&ask.message),
			})
			.await
		})
	}
}

/// Nobody to ask: every ask is an error naming what was asked, why nobody
/// can answer, and what to pass instead.
#[derive(Debug, Clone)]
pub struct RefusalPrompt {
	/// Why nobody can answer, ie `this launch is headless`.
	reason: String,
}

impl RefusalPrompt {
	/// A refusal because of `reason`.
	pub fn new(reason: impl Into<String>) -> Self {
		Self {
			reason: reason.into(),
		}
	}

	/// The error `ask` is refused with.
	pub fn refusal(&self, ask: &OperatorAsk) -> BevyError {
		let what = match ask.kind {
			OperatorAskKind::Notice => "needs a person to act on it",
			OperatorAskKind::Code { .. } => {
				"is typed by a person, never passed or piped"
			}
			OperatorAskKind::SecretLine => "is typed by the operator",
		};
		match &ask.instead {
			Some(instead) => {
				bevyhow!(
					"{} {what}, and {}: {instead}",
					ask.subject,
					self.reason
				)
			}
			None => bevyhow!("{} {what}, and {}", ask.subject, self.reason),
		}
	}
}

impl OperatorPromptProvider for RefusalPrompt {
	fn describe(&self) -> String { format!("nobody ({})", self.reason) }

	fn interactive(&self) -> bool { false }

	fn ask(&self, ask: &OperatorAsk) -> SendBoxedFuture<Result<String>> {
		let refusal = self.refusal(ask);
		Box::pin(async move { Err(refusal) })
	}
}

#[cfg(test)]
mod test {
	use super::*;

	/// A provider answering every ask with `answer`, as a scripted one
	/// would, but declaring no person behind it.
	struct Scripted;

	impl OperatorPromptProvider for Scripted {
		fn describe(&self) -> String { "a script".into() }
		fn interactive(&self) -> bool { false }
		fn ask(&self, _ask: &OperatorAsk) -> SendBoxedFuture<Result<String>> {
			Box::pin(async { Ok("123456".to_string()) })
		}
	}

	/// The refusal names what was asked, why nobody answers and what to do
	/// instead, per kind.
	#[crate::test]
	async fn refuses_with_what_to_do() {
		let refusal = RefusalPrompt::new("this launch is headless");
		[
			OperatorAsk::secret_line("a backup passphrase", "passphrase: ")
				.with_instead("run this interactively, without `--headless`"),
			OperatorAsk::code("an mfa code", "code: ", 6),
			OperatorAsk::notice("a sign in", "open https://example.com"),
		]
		.iter()
		.map(|ask| refusal.refusal(ask).to_string())
		.map(|err| err.lines().next().unwrap_or_default().to_string())
		.collect::<Vec<_>>()
		.join("\n")
		.xpect_snapshot();
		refusal
			.ask(&OperatorAsk::secret_line("x", "x: "))
			.await
			.unwrap_err();
	}

	/// A code is never asked of a provider no person stands behind, however
	/// it would answer.
	#[crate::test]
	async fn a_code_needs_a_person() {
		OperatorPrompt::ask_with(
			&Scripted,
			&OperatorAsk::code("an mfa code", "code: ", 6),
		)
		.await
		.unwrap_err()
		.to_string()
		.xpect_contains("is typed by a person")
		.xpect_contains("a script answers without a person");
		// anything else it may answer
		OperatorPrompt::ask_with(
			&Scripted,
			&OperatorAsk::secret_line("a value", "value: "),
		)
		.await
		.unwrap()
		.xpect_eq("123456");
	}
}
