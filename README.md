# Base Rust Web API

A lightweight, multi-thread async Rust web API with a NestJS-like folder structure and a simple routing system. Controllers declare their own routes, and the router registers them at startup.

## Features

- Multi-thread async runtime (Tokio runtime on each execution thread)
- Strongly inspired on NestJS proven development patterns
- Request/Response primitives with automatic status lines
- Controller-first route declarations (NestJS-like)
- Domain-driven structure: controller, service, repo, DTO
- CLI tool for generating new entities, middlewares and managing DB.

## Project Structure

```
src/
  bin/
    scaffold_entity.rs
  domain/
    dog/
      controller.rs
      service.rs
      repo.rs
      dto.rs
      mod.rs
    mod.rs
  primitives/
    http/
      request.rs
      response.rs
      mod.rs
    mod.rs
  routing/
    init.rs
    mod.rs
  main.rs
```


## Configuration (.env)

You can override the default port and number of worker threads by creating a `.env` file in the project root:

```
# .env
PORT=80                # Server port (default: 8080)
CORES=4                # Number of worker threads (default: all available cores)
BCRYPT_COST=12         # bcrypt cost factor for password hashing (default: 12)

DB_HOST=localhost      # Postgres host (default: localhost)
DB_PORT=5432           # Postgres port (default: 5432)
DB_USER=postgres       # Postgres user (default: postgres)
DB_PASS=postgres       # Postgres password (default: postgres)
DB_NAME=postgres       # Postgres database name (default: postgres)
DB_MAX_CONNECTIONS=10  # Max DB pool connections (default: 10)
```

These values will be loaded automatically at startup.

## Running the Server

```bash
cargo run
```

The server listens on `127.0.0.1:8080`.

## Routing Flow

1. The router is initialized at startup via `init(init_routes())`.
2. Each controller exposes a `routes()` method returning `Route` definitions.
3. The router matches method/path and invokes the controller handler.
4. The controller returns a `Response`, which is written back to the client.

## Creating a New Entity

Use the scaffold CLI to generate a new domain entity:

```bash
cargo run --bin scaffold_entity -- Dog
```

This will:
- Create `src/domain/dog/` with controller/service/repo/dto files.
- Add the entity to `src/domain/mod.rs`.
- Register the controller routes in `src/routing/init.rs`.

## Creating a New Middleware

Use the scaffold CLI to generate a new middleware:

```bash
cargo run --bin scaffold_middleware -- Auth
```

This will:
- Create `src/middlewares/auth.rs`.
- Add it as `pub mod auth;` in `src/middlewares/mod.rs`.

## Adding Controller Routes

Routes are declared inside each controller’s `routes()` method using the `route!` macro:

```rust
Route::new("GET", &["dog"], vec![route!(DogController::get_all)])
```

Handlers receive:
- `&mut Request`
- `RouteParams` (path params like `:id` are available via `params.get("id")`)

## Middleware Support

Routes accept an array of functions (middlewares + final handler). Handlers are executed in order, and the last handler's `Response` is returned.

```rust
Route::new(
  "GET",
  &["dog"],
  vec![
    middleware!(DogMiddleware::log_request),
    middleware!(DogMiddleware::authorize),
    route!(DogController::get_all),
  ],
)
```

Middleware signature (See the section above to see how to use the cli to create a new middleware automatically) :

```rust
pub async fn log_request(
  request: &mut Request,
  params: &RouteParams,
  handlers: &mut Vec<Handler>,
) -> Response {
  // ...before
  // If we detect an error or auth failure we can
  // return prematurly the response object to the router without
  // executing the next functions
  let response = next_handler(request, params, handlers).await;
  // ...after
  response
}
```

IMPORTANT: use earlier handlers for middleware and put the main controller action last.


## Database Usage

To fetch data from the Postgres database, use the `db::query` function. It takes a SQL string and a vector of bind parameters (for SQL injection safety):

```rust
// Example: fetch a user by id
let user_id = 42;
let rows = db::query("SELECT * FROM users WHERE id = $1", vec![DbParam::Int32(user_id)]).await?;
for row in rows {
  let name: String = row.try_get("name")?;
  // ...
}
```

Bind parameters must implement the `Encode` and `Type` traits from sqlx. Use `?` placeholders for parameters in your SQL.

The connection pool is initialized automatically at startup.

## Database Migrations & Seeders

Database schema migrations and seed data are managed with SQL files and a CLI tool:

### Creating a Migration or Seeder

- To create a new migration:
  ```bash
  cargo run --bin db_cli -- migration:new
  ```
  You will be prompted for a name. Two files will be created in `src/db/migrations/`: an `_up.sql` (apply) and a `_down.sql` (undo) file, both prefixed with a timestamp for uniqueness.

- To create a new seeder:
  ```bash
  cargo run --bin db_cli -- seed:new
  ```
  You will be prompted for a name. Two files will be created in `src/db/seeders/`.

### Applying Migrations/Seeders

- To apply all pending migrations:
  ```bash
  cargo run --bin db_cli -- migrate
  ```
- To apply all pending seeders:
  ```bash
  cargo run --bin db_cli -- seed
  ```

### Undoing Migrations/Seeders

- To undo the last applied migration:
  ```bash
  cargo run --bin db_cli -- migrate:undo
  ```
- To undo the last applied seeder:
  ```bash
  cargo run --bin db_cli -- seed:undo
  ```

The framework automatically creates tables (`_migrations`, `_seeders`) to track which scripts have been applied. Each migration/seeder must have both an `_up.sql` and a `_down.sql` file for full support.

## Notes

- Responses automatically include `Content-Length` and `Connection: close` if not provided.
- The router is a singleton registry initialized before the server starts listening.
