# PostgreSQL MCP - System Patterns

## Design Patterns

### Command Pattern
- Each MCP tool method (register, query, insert, etc.) follows the command pattern
- Request objects contain all necessary parameters for an operation
- Operations are executed through a consistent interface

### Connection Pool Pattern
- Database connections are managed through connection pools
- Connections are identified by unique IDs
- Thread-safe access to connection pools using ArcSwap

### Request-Response Pattern
- All operations follow a clear request-response pattern
- Requests contain operation parameters
- Responses contain operation results or error information

### Validation-Execution Pattern
- SQL statements are first validated for correct type and syntax
- Only after validation is execution performed
- Clear error messages are returned for validation failures

## Code Organization

### Resource Management
- Connections are treated as resources with explicit lifecycle
- Registration creates the resource
- Unregistration removes the resource
- All operations require valid resource identifiers

### Error Handling
- Operations return `Result<String, Error>` for uniform error handling
- MCP errors are converted to appropriate protocol errors
- Descriptive error messages are provided for debugging

### Transport Independence
- Core functionality is independent of transport mechanism
- Same operations work with stdio or SSE transport
- Transport-specific code is isolated in main.rs

## System Interactions

### Client-Server Model
- PostgreSQL MCP acts as a server for client requests
- Clients connect through stdio or SSE
- Server processes requests and returns responses

### Database Interaction
- SQL validation ensures safety before execution
- Operations are mapped to specific SQL statement types
- Query results are converted to JSON for consistent return format

## Testing Strategy

### Integration Testing
- Each operation is tested through the MCP interface
- Tests use a real PostgreSQL database (via TestPg)
- Complete workflow testing (create → query → update → delete)

### Operation Isolation
- Tests for different operations are kept separate
- Each test manages its own resources (tables, data)
- Test database is reset between test runs
