use anyhow::Result;
use crossterm::*;
use std::io::stdout;
use std::io::Write;

pub fn show_cursor() {
	let mut stdout = stdout();
	stdout.execute(cursor::Show).unwrap();
}


pub fn reset_cursor() {
	let mut stdout = stdout();
	stdout.execute(cursor::MoveTo(0, 0)).unwrap();
}

pub fn clear() -> Result<()> {
	let mut stdout = stdout();
	stdout
		.queue(terminal::Clear(terminal::ClearType::All))?
		// .queue(cursor::Hide)?
		.queue(cursor::MoveTo(0, 0))?;
	stdout.flush()?;
	Ok(())
}
