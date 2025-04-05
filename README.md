# Postgres MCP

Postgres MCP is a Model Context Protocol (MCP) implementation for PostgreSQL databases. It provides a standardized interface for AI agents to interact with PostgreSQL databases through a set of well-defined commands.

## Features

- **Connection Management**
  - Register and unregister database connections
  - Support for multiple concurrent database connections
  - Connection pooling for efficient resource management

- **Database Operations**
  - Execute SELECT queries
  - Insert new records
  - Update existing records
  - Delete records
  - Create and drop tables
  - Create and drop indexes
  - Describe table structures
  - List tables in a schema

- **SQL Validation**
  - Built-in SQL parser for validating statements
  - Support for PostgreSQL-specific syntax
  - Safety checks to ensure only allowed operations are performed

## Installation

```bash
cargo install postgres-mcp
```

## Usage

### Configuration

Add the following to your MCP configuration file:

```json
{
  "mcpServers": {
    "postgres": {
      "command": "postgres-mcp",
      "args": ["stdio"]
    }
  }
}
```

or run it in SSE mode:

First, start the `postgres-mcp` server in SSE mode:

```bash
postgres-mcp sse
```

Then, configure the MCP config file to use the SSE mode:

```json
{
  "mcpServers": {
    "postgres": {
      "url": "http://localhost:3000/sse"
    }
  }
}
```

Once you started the `postgres-mcp` server, you should see the status of the MCP config is green, like this (cursor):

![mcp-status](./docs/images/mcp-status.jpg)

And then you could interact with it via the agent, like this (cursor):

![mcp](./docs/images/mcp.jpg)

### Commands

#### Register a Database Connection

```bash
pg_mcp register "postgres://postgres:postgres@localhost:5432/postgres"
# Returns a connection ID (UUID)
```

#### Unregister a Connection

```bash
pg_mcp unregister <connection_id>
```

#### Execute a SELECT Query

```bash
pg_mcp query <connection_id> "SELECT * FROM users"
```

#### Insert Data

```bash
pg_mcp insert <connection_id> "INSERT INTO users (name, email) VALUES ('John Doe', 'john.doe@example.com')"
```

#### Update Data

```bash
pg_mcp update <connection_id> "UPDATE users SET name = 'Jane Doe' WHERE id = 1"
```

#### Delete Data

```bash
pg_mcp delete <connection_id> "users" "1"
```

#### Create a Table

```bash
pg_mcp create <connection_id> "CREATE TABLE users (id SERIAL PRIMARY KEY, name VARCHAR(255), email VARCHAR(255))"
```

#### Drop a Table

```bash
pg_mcp drop <connection_id> "users"
```

#### Create an Index

```bash
pg_mcp create_index <connection_id> "CREATE INDEX idx_users_name ON users (name)"
```

#### Drop an Index

```bash
pg_mcp drop_index <connection_id> "idx_users_name"
```

#### Describe a Table

```bash
pg_mcp describe <connection_id> "users"
```

## Dependencies

- Rust 1.70 or later
- PostgreSQL 12 or later
- Required Rust crates:
  - anyhow: 1.0
  - arc-swap: 1.7
  - sqlx: 0.8 (with "runtime-tokio", "tls-rustls-aws-lc-rs", "postgres" features)
  - rmcp: 0.1 (with "server", "transport-sse-server", "transport-io" features)
  - schemars: 0.8
  - sqlparser: 0.55
  - tokio: 1.44

## Development

To build from source:

```bash
git clone https://github.com/yourusername/postgres-mcp.git
cd postgres-mcp
cargo build --release
```

## License

MIT license. See [LICENSE.md](LICENSE.md) for details.

## Contributing

Contributions are welcome! Please open an issue or submit a pull request.
