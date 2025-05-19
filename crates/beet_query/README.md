# Beet Query

A high level wrapper for sea-query with libsql and limbo backends.

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