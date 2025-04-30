use anyhow::Result;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

// currently unused, not nessecary?
pub struct CompileCheck;

// Static counter for generating unique file names
static COUNTER: AtomicUsize = AtomicUsize::new(0);

impl CompileCheck {
	pub fn string(code: &str) -> Result<()> {
		// Create temporary file with unique name using atomic counter
		let temp_dir = std::env::temp_dir();
		let counter = COUNTER.fetch_add(1, Ordering::SeqCst);
		let file_name = format!("compilecheck_{}.rs", counter);
		let file_path = temp_dir.join(&file_name);

		// Write code to temp file
		fs::write(&file_path, code)?;

		// Attempt compilation
		let output = Self::file(&file_path);
		// Clean up temp files
		let _ = fs::remove_file(&file_path);
		let _ = fs::remove_file(
			temp_dir.join(Path::new(&file_name).file_stem().unwrap()),
		);
		output
	}

	pub fn file(path: &Path) -> Result<()> {
		let output = Command::new("rustc").arg(&path).output()?;
		if output.status.success() {
			Ok(())
		} else {
			anyhow::bail!("{}", String::from_utf8_lossy(&output.stderr))
		}
	}
}
