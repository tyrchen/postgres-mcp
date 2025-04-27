use anyhow::{Context, Result};
use rmcp::{ServiceExt, model::CallToolRequestParam, object, transport::TokioChildProcess};
use tokio::process::Command;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

// Default connection string - replace with your actual connection string
// For example, use environment variables: std::env::var("DATABASE_URL").unwrap_or_else(|_| TEST_DB_URL.to_string())
const TEST_DB_URL: &str = "postgres://postgres:postgres@localhost:5432/postgres";

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();

    tracing::info!("Starting basic_agent example...");

    // --- Start the postgres-mcp server process ---
    tracing::info!("Spawning postgres-mcp process in stdio mode...");
    let mut cmd = Command::new("postgres-mcp"); // Assumes postgres-mcp is in PATH
    cmd.arg("stdio");
    let transport = TokioChildProcess::new(&mut cmd).context("Failed to create child process")?;

    // --- Connect to the MCP service using ServiceExt ---
    tracing::info!("Connecting to MCP service...");
    let service = ().serve(transport).await.context("Failed to connect to MCP service")?;

    let server_info = service.peer_info();
    tracing::info!("Connected to server: {:#?}", server_info);

    let conn_id: String;

    // --- Register a database connection ---
    tracing::info!("Registering database connection: {}", TEST_DB_URL);
    match service
        .call_tool(CallToolRequestParam {
            name: "register".into(),
            arguments: Some(object!({ "conn_str": TEST_DB_URL })),
        })
        .await
    {
        Ok(result) => {
            conn_id = result.content[0]
                .raw
                .as_text()
                .context("Register result was not text")?
                .text
                .clone();
            tracing::info!(
                "Database connection registered successfully. Conn ID: {}",
                conn_id
            );
        }
        Err(e) => {
            tracing::error!("Failed to register database connection: {}", e);
            service.cancel().await?; // Ensure service is cancelled on error
            return Err(e).context("Registration failed");
        }
    }

    // --- Perform database operations ---
    let table_name = "mcp_basic_agent_test";

    // 1. Create Table
    tracing::info!("Creating table: {}", table_name);
    match service
        .call_tool(CallToolRequestParam {
            name: "create_table".into(),
            arguments: Some(object!({
                "conn_id": conn_id,
                "query": format!("CREATE TABLE IF NOT EXISTS {} (id SERIAL PRIMARY KEY, message TEXT)", table_name)
            })),
        })
        .await
    {
        Ok(result) => tracing::info!("Create table result: {:?}", result.content),
        Err(e) => tracing::error!("Create table failed: {}", e), // Continue even if create fails (might already exist)
    }

    // 2. Insert Data
    tracing::info!("Inserting data into table: {}", table_name);
    match service
        .call_tool(CallToolRequestParam {
            name: "insert".into(),
            arguments: Some(object!({
                "conn_id": conn_id,
                "query": format!("INSERT INTO {} (message) VALUES ($1), ($2)", table_name),
                // Note: Actual parameter binding isn't directly supported via this basic query string approach.
                // For parameterized queries, a different MCP tool or approach might be needed if developed.
                // This example inserts literal values. A more robust insert might construct the full query string.
                "query": format!("INSERT INTO {} (message) VALUES ('Hello from basic_agent!'), ('MCP rocks!')", table_name)
            })),
        })
        .await
    {
        Ok(result) => tracing::info!("Insert result: {:?}", result.content),
        Err(e) => tracing::error!("Insert failed: {}", e),
    }

    // 3. Query Data
    tracing::info!("Querying data from table: {}", table_name);
    match service
        .call_tool(CallToolRequestParam {
            name: "query".into(),
            arguments: Some(object!({
                "conn_id": conn_id,
                "query": format!("SELECT id, message FROM {}", table_name)
            })),
        })
        .await
    {
        Ok(result) => {
            if let Some(text_content) = result.content.first().and_then(|c| c.raw.as_text()) {
                tracing::info!("Query result: {}", text_content.text);
            } else {
                tracing::warn!(
                    "Query returned unexpected content format: {:?}",
                    result.content
                );
            }
        }
        Err(e) => tracing::error!("Query failed: {}", e),
    }

    // 4. Describe Table
    tracing::info!("Describing table: {}", table_name);
    match service
        .call_tool(CallToolRequestParam {
            name: "describe".into(),
            arguments: Some(object!({
                "conn_id": conn_id,
                "table": table_name
            })),
        })
        .await
    {
        Ok(result) => {
            if let Some(text_content) = result.content.first().and_then(|c| c.raw.as_text()) {
                tracing::info!("Describe result: {}", text_content.text);
            } else {
                tracing::warn!(
                    "Describe returned unexpected content format: {:?}",
                    result.content
                );
            }
        }
        Err(e) => tracing::error!("Describe failed: {}", e),
    }

    // 5. List Tables (Public Schema)
    tracing::info!("Listing tables in 'public' schema...");
    match service
        .call_tool(CallToolRequestParam {
            name: "list_tables".into(),
            arguments: Some(object!({
                "conn_id": conn_id,
                "schema": "public"
            })),
        })
        .await
    {
        Ok(result) => {
            if let Some(text_content) = result.content.first().and_then(|c| c.raw.as_text()) {
                tracing::info!("List tables result: {}", text_content.text);
            } else {
                tracing::warn!(
                    "List tables returned unexpected content format: {:?}",
                    result.content
                );
            }
        }
        Err(e) => tracing::error!("List tables failed: {}", e),
    }

    // 6. Drop Table
    tracing::info!("Dropping table: {}", table_name);
    match service
        .call_tool(CallToolRequestParam {
            name: "drop_table".into(),
            arguments: Some(object!({
                "conn_id": conn_id,
                "table": table_name
            })),
        })
        .await
    {
        Ok(result) => tracing::info!("Drop table result: {:?}", result.content),
        Err(e) => tracing::error!("Drop table failed: {}", e),
    }

    // --- Unregister the connection ---
    tracing::info!("Unregistering connection ID: {}", conn_id);
    match service
        .call_tool(CallToolRequestParam {
            name: "unregister".into(),
            arguments: Some(object!({ "conn_id": conn_id })),
        })
        .await
    {
        Ok(_) => tracing::info!("Connection unregistered successfully."),
        Err(e) => tracing::error!("Failed to unregister connection: {}", e),
    }

    // --- Shutdown ---
    tracing::info!("Shutting down basic_agent example...");
    service.cancel().await?;
    tracing::info!("Agent finished.");

    Ok(())
}
