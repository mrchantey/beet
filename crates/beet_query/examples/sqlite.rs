use sqlx::Row;
use sqlx::sqlite::SqlitePool;
use std::env;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	// Use in-memory sqlite for demonstration
	let db_url = env::var("DATABASE_URL")
		.unwrap_or_else(|_| "sqlite::memory:".to_string());
	let pool = SqlitePool::connect(&db_url).await?;

	// Create the users table if it doesn't exist
	let _res = sqlx::query("CREATE TABLE IF NOT EXISTS users (email TEXT)")
		.execute(&pool)
		.await?;

	// Insert a user
	sqlx::query("INSERT INTO users (email) VALUES (?1)")
		.bind("foo@example.com")
		.execute(&pool)
		.await?;

	// Query by email (prepared-like)
	let row = sqlx::query("SELECT * FROM users WHERE email = ?1")
		.bind("foo@example.com")
		.fetch_one(&pool)
		.await?;
	let value: String = row.try_get(0)?;
	println!("Prepared Query 1: {:?}", value);

	// Update the email
	sqlx::query("UPDATE users SET email = ?1 WHERE email = ?2")
		.bind("bar@example.com")
		.bind("foo@example.com")
		.execute(&pool)
		.await?;
	println!("Updated email from foo@example.com to bar@example.com");

	// Query by new email
	let row = sqlx::query("SELECT * FROM users WHERE email = ?1")
		.bind("bar@example.com")
		.fetch_one(&pool)
		.await?;
	let value: String = row.try_get(0)?;
	println!("Fresh Query: {:?}", value);

	// Query again with the same statement (simulating prepared reuse)
	let row = sqlx::query("SELECT * FROM users WHERE email = ?1")
		.bind("bar@example.com")
		.fetch_one(&pool)
		.await?;
	let value: String = row.try_get(0)?;
	println!("Prepared Query 2: {:?}", value);

	Ok(())
}
