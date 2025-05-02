use libsql::Builder;

#[tokio::main]
async fn main() {
	let db = if let Ok(url) = std::env::var("LIBSQL_URL") {
		let token = std::env::var("LIBSQL_AUTH_TOKEN").unwrap_or_else(|_| {
			println!("LIBSQL_TOKEN not set, using empty token...");
			"".to_string()
		});

		Builder::new_remote(url, token).build().await.unwrap()
	} else {
		Builder::new_local(":memory:").build().await.unwrap()
	};

	let conn = db.connect().unwrap();

	conn.query("select 1; select 1;", ()).await.unwrap();

	conn.execute("CREATE TABLE IF NOT EXISTS users (email TEXT)", ())
		.await
		.unwrap();

	conn.execute("INSERT INTO users (email) VALUES (?1)", ["foo@example.com"])
		.await
		.unwrap();

	let mut query_stmt = conn
		.prepare("SELECT * FROM users WHERE email = ?1")
		.await
		.unwrap();

	let value = query_stmt
		.query(["foo@example.com"])
		.await
		.unwrap()
		.next()
		.await
		.unwrap()
		.unwrap()
		.get_value(0)
		.unwrap();

	println!("Prepared Query 1: {:?}", value);

	// Update the email to bar@example.com
	conn.execute("UPDATE users SET email = ?1 WHERE email = ?2", [
		"bar@example.com",
		"foo@example.com",
	])
	.await
	.unwrap();
	println!("Updated email from foo@example.com to bar@example.com");

	// fresh statement works
	let value = conn
		.query("SELECT * FROM users WHERE email = ?1", ["bar@example.com"])
		.await
		.unwrap()
		.next()
		.await
		.unwrap()
		.unwrap()
		.get_value(0)
		.unwrap();
	println!("Fresh Query: {:?}", value);

	query_stmt.reset();

	// Prepared statement is empty
	let value = query_stmt
		.query(["bar@example.com"])
		.await
		.unwrap()
		.next()
		.await
		.unwrap()
		.unwrap() // panics
		.get_value(0)
		.unwrap();
	println!("Prepared Query 2: {:?}", value);
}
