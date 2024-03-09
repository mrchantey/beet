# Core Api


| Description                        | Handler | Scheme   | Method | Address              | Payload1 | Payload2                                        |
| ---------------------------------- | ------- | -------- | ------ | -------------------- | -------- | ----------------------------------------------- |
| Create an entity and return its id | App     | Response | Create | `entity`             |          | `BeetEntityId`                                  |
| Create an entity with a given id   | App     | Response | Create | `entity:id`          |          | `Result` Fails if an entity with this id exists |
|                                    | App     | Publish  | Update | `entity:id/position` |          |                                                 |
|                                    | App     | Response | Delete | `entity`             |          | `BeetEntityId`                                  |
|                                    | App     | Publish  | Delete | `entity:id/position` |          |                                                 |