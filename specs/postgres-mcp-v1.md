# Postgres MCP

Postgres MCP is a MCP (Model Context Protocol) implementation for Postgres. It allows AI agents to interact with Postgres databases.

## APIs

### pg_mcp register <conn_str>

Register a new Postgres connection pool. AI agents can use this id to query the database.

```shell
pg_mcp register "postgres://postgres:postgres@localhost:5432/postgres"
123e4567-e89b-12d3-a456-426614174000
```

### pg_mcp unregister <conn_id>

Unregister a Postgres connection. The connection pool will be closed and the connection id can't be used again.

```shell
pg_mcp unregister 123e4567-e89b-12d3-a456-426614174000
```

### pg_mcp query <conn_id> <query_sql>

Query the database with a SQL statement. It must be a valid "SELECT" statement. We will use sqlparser to parse the statement, validate it is a valid "SELECT" statement, and then generate the SQL statement again. The newly generated SQL statement will be executed against the database.

```shell
pg_mcp query 123e4567-e89b-12d3-a456-426614174000 "SELECT * FROM users"
```

### pg_mcp insert <conn_id> <query_sql>

Insert a new row into the database. It must be a valid "INSERT" statement. We will use sqlparser to parse the statement, validate it is a valid "INSERT" statement, and then generate the SQL statement again. The newly generated SQL statement will be executed against the database.

```shell
pg_mcp insert 123e4567-e89b-12d3-a456-426614174000 "INSERT INTO users (name, email) VALUES ('John Doe', 'john.doe@example.com')"
```

### pg_mcp update <conn_id> <query_sql>

Update a row in the database. It must be a valid "UPDATE" statement. We will use sqlparser to parse the statement, validate it is a valid "UPDATE" statement, and then generate the SQL statement again. The newly generated SQL statement will be executed against the database.

### pg_mcp delete <conn_id> <table_name> <pk>

Delete a row in the database. We will generate the SQL statement and execute it against the database.

```shell
pg_mcp delete 123e4567-e89b-12d3-a456-426614174000 "users" "1"
```

### pg_mcp describe <conn_id> <table_name>

Describe a table in the database. We will generate the SQL statement and execute it against the database.

```shell
pg_mcp describe 123e4567-e89b-12d3-a456-426614174000 "users"
```

### pg_mcp create <conn_id> <create_sql>

Create a new table in the database. It must be a valid "CREATE TABLE" statement. We will use sqlparser to parse the statement, validate it is a valid "CREATE TABLE" statement, and then generate the SQL statement again. The newly generated SQL statement will be executed against the database.

```shell
pg_mcp create 123e4567-e89b-12d3-a456-426614174000 "CREATE TABLE users (id SERIAL PRIMARY KEY, name VARCHAR(255), email VARCHAR(255))"
```

### pg_mcp drop <conn_id> <table_name>

Drop a table in the database. We will generate the SQL statement and execute it against the database.

```shell
pg_mcp drop 123e4567-e89b-12d3-a456-426614174000 "users"
```

### pg_mcp create_index <conn_id> <create_index_sql>

Create an index on a table. It must be a valid "CREATE INDEX" statement. We will use sqlparser to parse the statement, validate it is a valid "CREATE INDEX" statement, and then generate the SQL statement again. The newly generated SQL statement will be executed against the database.

```shell
pg_mcp create_index 123e4567-e89b-12d3-a456-426614174000 "CREATE INDEX idx_users_name ON users (name)"
```

### pg_mcp drop_index <conn_id> <index_name>

Drop an index on a table. We will generate the SQL statement and execute it against the database.

## Implementations

### Data structure

```rust

#[derive(Debug, Clone)]
pub(crate) struct Conn {
  pub(crate) id: String,
  pub(crate) conn_str: String,
  pub(crate) pool: PgPool,
}

pub(crate) struct Conns {
  pub(crate) inner: Arc<ArcSwap<HashMap<String, Conn>>>,
}

#[derive(Debug, FromRow, Serialize, Deserialize)]
struct JsonRow(serde_json::Value);

impl Conns {
  fn new() -> Self {
    ...
  }

  // return the conn id (uuid) if success
  async fn register(&self, conn_str: String) -> Result<String, Error> {
    let mut conns = self.inner.load();
    // use arc_swap to update the inner map
    ...
  }

  fn unregister(&self, id: String) -> Result<(), Error> {
    let mut conns = self.inner.load();
    // use arc_swap to update the inner map
    ...
  }

  // return the result as a json string (serialize Vec<JsonRow>)
  async fn query(&self, id: &str, query: &str) -> Result<String, Error> { ... }

  // return "success, rows_affected: <rows_affected>" if success
  async fn insert(&self, id: &str, query: &str) -> Result<String, Error> { ... }

  // return "success, rows_affected: <rows_affected>" if success
  async fn update(&self, id: &str, query: &str) -> Result<String, Error> { ... }

  // return "success, rows_affected: <rows_affected>" if success
  async fn delete(&self, id: &str, table: &str, pk: &str) -> Result<String, Error> { ... }

  // return "success" if success
  async fn create_table(&self, id: &str, query: &str) -> Result<String, Error> { ... }

  // return "success" if success
  async fn drop_table(&self, id: &str, table: &str) -> Result<String, Error> { ... }

  // return "success" if success
  async fn create_index(&self, id: &str, query: &str) -> Result<String, Error> { ... }

  // return "success" if success
  async fn drop_index(&self, id: &str, index: &str) -> Result<String, Error> { ... }

  // return the result as a json string (serialize Vec<JsonRow>), table name is "schema.table" or "table" if public schema
  async fn describe(&self, id: &str, table: &str) -> Result<String, Error> { ... }

  // return the result as a json string (serialize Vec<JsonRow>)
  async fn list_tables(&self, id: &str) -> Result<String, Error> { ... }

}

```

### Folder structure

src/
├── main.rs: The main entry point of the server
├── lib.rs: core data structure (Conn, Conns) and imports
├── pg.rs: core postgres related logic, e.g. implementation of Conns
└── mcp.rs: MCP related data structure andserver implementation

### Dependencies

anyhow: 1.0

arc-swap: 1.7

sqlx: 0.8 with "runtime-tokio", "tls-rustls-aws-lc-rs", "postgres" features

Example:

```rust
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect("postgres://postgres:password@localhost/test").await?;

    let row: (i64,) = sqlx::query_as("SELECT $1")
        .bind(150_i64)
        .fetch_one(&pool).await?;

    assert_eq!(row.0, 150);

    Ok(())
}
```

rmcp: 0.1 with "server", "transport-sse-server", "transport-io" features

Example:

```rust
use anyhow::Result;
use tracing_subscriber::{self, EnvFilter};
use rmcp::{
    ServerHandler, ServiceExt, transport::stdio
    model::{ServerCapabilities, ServerInfo},
    schemars, tool, Error as McpError, model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SumRequest {
    #[schemars(description = "the left hand side number")]
    pub a: i32,
    pub b: i32,
}
#[derive(Debug, Clone)]
pub struct Calculator;
#[tool(tool_box)]
impl Calculator {
    #[tool(description = "Calculate the sum of two numbers")]
    fn sum(&self, #[tool(aggr)] SumRequest { a, b }: SumRequest) -> Result<CallToolResult, McpError> {
        Ok(CallToolResult::new(Content::Text((a + b).to_string())))
    }

    #[tool(description = "Calculate the sum of two numbers")]
    fn sub(
        &self,
        #[tool(param)]
        #[schemars(description = "the left hand side number")]
        a: i32,
        #[tool(param)]
        #[schemars(description = "the left hand side number")]
        b: i32,
    ) -> String {
        (a - b).to_string()
    }
}

#[tool(tool_box)]
impl ServerHandler for Calculator {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("A simple calculator".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

/// to test mcp server, run `npx @modelcontextprotocol/inspector cargo run`
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize the tracing subscriber with file and stdout logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::DEBUG.into()))
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    tracing::info!("Starting MCP server");

    // Create an instance of our Calculator router
    let service = Calculator::new().serve(stdio()).await.inspect_err(|e| {
        tracing::error!("serving error: {:?}", e);
    })?;

    service.waiting().await?;
    Ok(())
}
```

schemars: 0.8

Example:

```rust
use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MyStruct {
    #[serde(rename = "myNumber")]
    pub my_int: i32,
    pub my_bool: bool,
    #[serde(default)]
    pub my_nullable_enum: Option<MyEnum>,
}

#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(untagged)]
pub enum MyEnum {
    StringNewType(String),
    StructVariant { floats: Vec<f32> },
}

let schema = schema_for!(MyStruct);
println!("{}", serde_json::to_string_pretty(&schema).unwrap());
```

sqlparser: 0.55

Doc: https://docs.rs/sqlparser/latest/sqlparser/

Example:

```rust
use sqlparser::dialect::PostgreSqlDialect;
use sqlparser::parser::Parser;

let dialect = PostgreSqlDialect {};

let sql = "SELECT a, b, 123, myfunc(b) \
           FROM table_1 \
           WHERE a > b AND b < 100 \
           ORDER BY a DESC, b";

let ast = Parser::parse_sql(&dialect, sql).unwrap();
```

tokio: 1.44
