use postgres_mcp::PgMcp;
use rmcp::ServiceExt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize the tracing subscriber with file and stdout logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .with_writer(std::io::stderr)
        .with_ansi(true)
        .init();

    tracing::info!("Starting Postgres MCP server");

    // Create an instance of our PostgresMcp router
    let service = PgMcp::new()
        .serve(rmcp::transport::stdio())
        .await
        .inspect_err(|e| {
            tracing::error!("serving error: {:?}", e);
        })?;

    service.waiting().await?;
    Ok(())
}
