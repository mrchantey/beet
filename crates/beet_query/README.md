# Beet Query

A high level wrapper for sea-query with libsql and limbo backends.

## Why not sea-orm?

- Simplicity: sea-orm is very powerful but can be a little intimidating for sql newbies, particularly when it comes to its approach to relations.
- Libsql: sea-orm depends on sqlx which [doesnt yet support](https://github.com/launchbadge/sqlx/issues/2674) zero dependency databases like limbo




```rust
#[derive(Table)]
struct User{
	id: u32,
	email: String,
	#[field(foreign)]
	posts: Vec<Post::Id>
}


#[derive(Table)]
struct Post{
	id: u32,
	title: String,
	body: String
}

User::select((UserId,UserEmail))-> (UserId,UserEmail)

```