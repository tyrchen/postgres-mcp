# PostgreSQL MCP - Technical Context

## Codebase Structure

### Main Components
- **src/lib.rs**: Main library entry point, exports the core components
- **src/main.rs**: CLI application entry point, handles command-line arguments and server startup
- **src/pg.rs**: Core PostgreSQL functionality, handles database connections and operations
- **src/mcp.rs**: MCP protocol implementation, defines request/response structures and tool interfaces

### Key Files
```
postgres-mcp
├── src/
│   ├── lib.rs         # Library entry point
│   ├── main.rs        # CLI application entry
│   ├── pg.rs          # PostgreSQL functionality
│   └── mcp.rs         # MCP protocol implementation
├── tests/
│   └── mcp_test.rs    # Integration tests
├── fixtures/
│   └── migrations/    # Test database migrations
├── Cargo.toml         # Rust package definition
└── README.md          # Documentation
```

## Core Abstractions

### `PgMcp` (src/pg.rs, src/mcp.rs)
- Main service implementation that handles MCP protocol integration
- Implements ServerHandler trait for MCP protocol
- Provides tool methods for each PostgreSQL operation

### `Conns` (src/pg.rs)
- Connection pool manager
- Handles registration and unregistration of database connections
- Thread-safe storage of connection pools
- Methods for executing different types of SQL operations

### `Conn` (src/pg.rs)
- Represents a single database connection
- Contains connection ID, connection string, and connection pool

## Key Dependencies
- **sqlx**: SQL toolkit for Rust (async/await)
- **rmcp**: Model Context Protocol implementation
- **tokio**: Asynchronous runtime
- **sqlparser**: SQL parsing and validation
- **arc-swap**: Thread-safe reference swapping
- **axum**: Web framework for SSE mode

## Communication Protocol
The service implements the MCP protocol with two transport modes:
1. **stdio**: For direct integration with parent processes
2. **SSE**: For web-based clients

## Database Interaction
- Uses connection pooling for efficient resource management
- Validates SQL queries before execution using sqlparser
- Provides a structured interface for common database operations
- Handles connection management automatically

## Security Considerations
- SQL validation ensures only specific operations are allowed
- Each operation is validated against its expected SQL type
- Connection strings must be valid PostgreSQL connection strings
- No arbitrary SQL execution is allowed - only specific statements
