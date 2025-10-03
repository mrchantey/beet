use crossterm::*;
use std::io::Write;
use std::io::stdout;

pub fn show_cursor() {
	let mut stdout = stdout();
	stdout.execute(cursor::Show).unwrap();
}


pub fn reset_cursor() {
	let mut stdout = stdout();
	stdout.execute(cursor::MoveTo(0, 0)).unwrap();
}

pub fn clear() -> std::io::Result<()> {
	let mut stdout = stdout();
	stdout
		.queue(terminal::Clear(terminal::ClearType::All))?
		// .queue(cursor::Hide)?
		.queue(cursor::MoveTo(0, 0))?;
	stdout.flush()?;
	Ok(())
}
