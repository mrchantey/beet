//! `vault/backup`: the identity file, passphrase-encrypted for a stick or
//! paper.

use crate::prelude::*;
use beet_core::prelude::*;

/// Request params for [`VaultBackup`], surfaced in `--help`.
#[derive(Reflect)]
struct BackupParams {
	/// The file to write; defaults to `beet-identity-<date>.age` in the
	/// current directory.
	out: Option<String>,
	/// Also print the armored text as a QR code, for paper.
	qr: bool,
	/// Refuse rather than prompt, for a runner with no terminal.
	headless: bool,
}

/// Back up the identity file: prompt for a passphrase twice on the terminal
/// (never echoed, never on argv) and write the file passphrase-encrypted as
/// armored age text, for a USB stick in a drawer; `--qr` also prints it as
/// a QR code for paper. Anyone with the passphrase and the file has the
/// identity, so the passphrase lives in a head. `vault/restore-identity`
/// is the inverse; `age -d <file>` on any laptop is the escape hatch.
///
/// ```sh
/// beet vault/backup --qr
/// beet vault/backup --out=/media/stick/keys.txt.age
/// ```
#[action]
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(
	PathPartial = PathPartial::new("backup"),
	ParamsPartial = ParamsPartial::new::<BackupParams>()
)]
pub async fn VaultBackup(cx: ActionContext<Request>) -> Result<Response> {
	let params = cx.input.parse_params::<BackupParams>()?;
	let identities = AgeIdentityFile::require()?;
	if params.headless {
		bevybail!(
			"a backup passphrase is typed on a terminal: run this \
			interactively, without `--headless`"
		);
	}
	let passphrase = terminal_ext::read_secret_line("passphrase: ")?;
	if passphrase.is_empty() {
		bevybail!("an empty passphrase protects nothing");
	}
	if terminal_ext::read_secret_line("passphrase (again): ")? != passphrase {
		bevybail!("the passphrases differ");
	}
	let ciphertext = AgePassphrase::new(passphrase)
		.encrypt(identities.to_string().as_bytes())?;
	let out = params.out.unwrap_or_else(|| {
		format!("beet-identity-{}.age", Timestamp::now().format_date())
	});
	fs_ext::write_private(&out, &ciphertext)?;
	let mut text = format!(
		"wrote `{out}` ({} identities). Keep it on a stick in a drawer and \
		remember the passphrase: nothing else can open it.\n",
		identities.len()
	);
	if params.qr {
		text.push_str(&qr_text(&ciphertext)?);
	}
	Response::ok_text(text).xok()
}

/// The armored backup as a QR code in unicode half-blocks.
#[cfg(feature = "qrcode")]
fn qr_text(armored: &str) -> Result<String> {
	qrcode::QrCode::new(armored.as_bytes())?
		.render::<qrcode::render::unicode::Dense1x2>()
		.quiet_zone(true)
		.build()
		.xmap(|qr| format!("\n{qr}\n"))
		.xok()
}

/// The QR print was not compiled in.
#[cfg(not(feature = "qrcode"))]
fn qr_text(_armored: &str) -> Result<String> {
	"(the qr print is not compiled into this binary: enable the `qrcode` \
	feature)\n"
		.to_string()
		.xok()
}
