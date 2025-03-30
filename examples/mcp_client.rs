use anyhow::Result;
use rmcp::{ServiceExt, model::CallToolRequestParam, object, transport::TokioChildProcess};
use tokio::process::Command;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .with_writer(std::io::stderr)
        .with_ansi(true)
        .init();

    // Start server
    let mut cmd = Command::new("postgres-mcp");
    let service = ().serve(TokioChildProcess::new(&mut cmd)?).await?;

    // Initialize
    let server_info = service.peer_info();
    tracing::info!("Connected to server: {server_info:#?}");

    // List tools
    let tools = service.list_all_tools().await?;
    tracing::info!("Available tools: {tools:#?}");

    // Register a Postgres connection
    let tool_result = service
        .call_tool(CallToolRequestParam {
            name: "register".into(),
            arguments: Some(object!({
                "conn_str": "postgresql://postgres:postgres@localhost:5432/todo"
            })),
        })
        .await?;
    tracing::info!("Tool result for register: {tool_result:#?}");

    let id = tool_result.content[0].raw.as_text().unwrap().text.clone();

    // Create a table
    let tool_result = service
        .call_tool(CallToolRequestParam {
            name: "create_table".into(),
            arguments: Some(object!({
                "conn_id": id,
                "query": "CREATE TABLE IF NOT EXISTS users (id SERIAL PRIMARY KEY, name TEXT NOT NULL, email TEXT UNIQUE NOT NULL)"
            })),
        })
        .await?;
    tracing::info!("Tool result for create_table: {tool_result:#?}");

    // Insert some data
    let tool_result = service
        .call_tool(CallToolRequestParam {
            name: "insert".into(),
            arguments: Some(object!({
                "conn_id": id,
                "query": "INSERT INTO users (name, email) VALUES ('John Doe', 'john@example.com')"
            })),
        })
        .await?;
    tracing::info!("Tool result for insert: {tool_result:#?}");

    // Query the data
    let tool_result = service
        .call_tool(CallToolRequestParam {
            name: "query".into(),
            arguments: Some(object!({
                "conn_id": id,
                "query": "SELECT * FROM users"
            })),
        })
        .await?;
    tracing::info!("Tool result for query: {tool_result:#?}");

    // List tables in the public schema
    let tool_result = service
        .call_tool(CallToolRequestParam {
            name: "list_tables".into(),
            arguments: Some(object!({
                "conn_id": id,
                "schema": "public"
            })),
        })
        .await?;
    tracing::info!("Tool result for list_tables: {tool_result:#?}");

    // Describe the users table
    let tool_result = service
        .call_tool(CallToolRequestParam {
            name: "describe".into(),
            arguments: Some(object!({
                "conn_id": id,
                "table": "users"
            })),
        })
        .await?;
    tracing::info!("Tool result for describe: {tool_result:#?}");

    // Update some data
    let tool_result = service
        .call_tool(CallToolRequestParam {
            name: "update".into(),
            arguments: Some(object!({
                "conn_id": id,
                "query": "UPDATE users SET name = 'Jane Doe' WHERE email = 'john@example.com'"
            })),
        })
        .await?;
    tracing::info!("Tool result for update: {tool_result:#?}");

    // Delete some data
    let tool_result = service
        .call_tool(CallToolRequestParam {
            name: "delete".into(),
            arguments: Some(object!({
                "conn_id": id,
                "query": "DELETE FROM users WHERE email = 'john@example.com'"
            })),
        })
        .await?;
    tracing::info!("Tool result for delete: {tool_result:#?}");

    // Create an index
    let tool_result = service
        .call_tool(CallToolRequestParam {
            name: "create_index".into(),
            arguments: Some(object!({
                "conn_id": id,
                "query": "CREATE INDEX idx_users_email ON users (email)"
            })),
        })
        .await?;
    tracing::info!("Tool result for create_index: {tool_result:#?}");

    // Drop the index
    let tool_result = service
        .call_tool(CallToolRequestParam {
            name: "drop_index".into(),
            arguments: Some(object!({
                "conn_id": id,
                "index": "idx_users_email"
            })),
        })
        .await?;
    tracing::info!("Tool result for drop_index: {tool_result:#?}");

    // Drop the table
    let tool_result = service
        .call_tool(CallToolRequestParam {
            name: "drop_table".into(),
            arguments: Some(object!({
                "conn_id": id,
                "table": "users"
            })),
        })
        .await?;
    tracing::info!("Tool result for drop_table: {tool_result:#?}");

    // Unregister the connection
    let tool_result = service
        .call_tool(CallToolRequestParam {
            name: "unregister".into(),
            arguments: Some(object!({
                "conn_id": id
            })),
        })
        .await?;
    tracing::info!("Tool result for unregister: {tool_result:#?}");

    service.cancel().await?;

    Ok(())
}
