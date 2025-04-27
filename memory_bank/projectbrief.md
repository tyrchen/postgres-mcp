# PostgreSQL MCP - Project Brief

## Overview
PostgreSQL MCP is a Rust-based implementation of the Model Context Protocol (MCP) for PostgreSQL databases. It provides a standardized interface for AI agents to interact with PostgreSQL databases through well-defined commands.

## Core Functionality
- **Connection Management**: Register, unregister, and manage database connections with connection pooling
- **Database Operations**: Execute SELECT queries, insert/update/delete records, create/drop tables and indexes, describe schemas
- **SQL Validation**: Built-in SQL parsing and validation to ensure only allowed operations are performed

## Technical Stack
- **Language**: Rust
- **Database**: PostgreSQL
- **Key Libraries**:
  - sqlx: PostgreSQL client
  - rmcp: Model Context Protocol implementation
  - tokio: Async runtime
  - sqlparser: SQL parsing and validation
  - clap: Command-line argument parsing

## Architecture
- MCP service that can run in two modes:
  - stdio mode: For direct communication through standard input/output
  - SSE (Server-Sent Events) mode: For web-based communication

## Implementation Details
- Uses connection pooling for efficient resource management
- Validates SQL queries before execution for security
- Supports multiple concurrent database connections
- Implements the complete MCP tool interface for PostgreSQL operations

## Development Status
The project is operational with a comprehensive test suite covering all major functionality.

## Target Use Cases
- AI agents that need to interact with PostgreSQL databases
- Database management tools that need standardized access to PostgreSQL
- Integration with other MCP-compatible systems
