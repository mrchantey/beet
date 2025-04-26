# Beet Query

A high-as-a-kite level ORM for typesafe and DRY database interactions.

## Why another ORM?

sea-query and diesel are very powerful but often overkill for simple CRUD applications.
Instead `beet_query` provides a very high level and type safe query builder, and allows
for dropping down to raw sql when required.