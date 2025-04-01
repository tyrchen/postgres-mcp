use anyhow::Result;
use rmcp::{
    RoleClient, ServiceExt, model::CallToolRequestParam, object, service::RunningService,
    transport::TokioChildProcess,
};
use sqlx_db_tester::TestPg;
use tokio::process::Command;

type McpService = RunningService<RoleClient, ()>;

#[allow(dead_code)]
struct TestService {
    tdb: TestPg,
    conn_id: String,
    service: McpService,
}

const TEST_CONN_STR: &str = "postgres://postgres:postgres@localhost:5432/postgres";
async fn setup_service() -> Result<TestService> {
    // use TestPg
    let tdb = TestPg::new(
        TEST_CONN_STR.to_string(),
        std::path::Path::new("./fixtures/migrations"),
    );
    let url = tdb.url();

    let mut cmd = Command::new("postgres-mcp");
    let service = ().serve(TokioChildProcess::new(&mut cmd)?).await?;

    // Register a test connection
    let tool_result = service
        .call_tool(CallToolRequestParam {
            name: "register".into(),
            arguments: Some(object!({
                "conn_str": url
            })),
        })
        .await?;

    let conn_id = tool_result.content[0].raw.as_text().unwrap().text.clone();
    Ok(TestService {
        tdb,
        conn_id,
        service,
    })
}

async fn cleanup_service(service: McpService, conn_id: impl AsRef<str>) -> Result<()> {
    // Unregister the connection
    service
        .call_tool(CallToolRequestParam {
            name: "unregister".into(),
            arguments: Some(object!({
                "conn_id": conn_id.as_ref()
            })),
        })
        .await?;

    service.cancel().await?;
    Ok(())
}

#[tokio::test]
async fn test_connection_management() -> Result<()> {
    let test_service = setup_service().await?;
    let service = test_service.service;
    let conn_id = test_service.conn_id;

    // Test listing tables in public schema
    let tool_result = service
        .call_tool(CallToolRequestParam {
            name: "list_tables".into(),
            arguments: Some(object!({
                "conn_id": conn_id.as_str(),
                "schema": "public"
            })),
        })
        .await?;
    assert!(!tool_result.content.is_empty());

    cleanup_service(service, &conn_id).await?;
    Ok(())
}

#[tokio::test]
async fn test_table_operations() -> Result<()> {
    let test_service = setup_service().await?;
    let service = test_service.service;
    let conn_id = test_service.conn_id;

    // Create test table
    let create_result = service
        .call_tool(CallToolRequestParam {
            name: "create_table".into(),
            arguments: Some(object!({
                "conn_id": conn_id.as_str(),
                "query": "CREATE TABLE IF NOT EXISTS test_users (id SERIAL PRIMARY KEY, name TEXT NOT NULL, email TEXT UNIQUE NOT NULL)"
            })),
        })
        .await?;
    assert!(!create_result.content.is_empty());

    // Describe the table
    let describe_result = service
        .call_tool(CallToolRequestParam {
            name: "describe".into(),
            arguments: Some(object!({
                "conn_id": conn_id.as_str(),
                "table": "test_users"
            })),
        })
        .await?;
    assert!(!describe_result.content.is_empty());

    // Drop the table
    let drop_result = service
        .call_tool(CallToolRequestParam {
            name: "drop_table".into(),
            arguments: Some(object!({
                "conn_id": conn_id.as_str(),
                "table": "test_users"
            })),
        })
        .await?;
    assert!(!drop_result.content.is_empty());

    cleanup_service(service, &conn_id).await?;
    Ok(())
}

#[tokio::test]
async fn test_data_operations() -> Result<()> {
    let test_service = setup_service().await?;
    let service = test_service.service;
    let conn_id = test_service.conn_id;

    // Create test table
    service
        .call_tool(CallToolRequestParam {
            name: "create_table".into(),
            arguments: Some(object!({
                "conn_id": conn_id.as_str(),
                "query": "CREATE TABLE IF NOT EXISTS test_users (id SERIAL PRIMARY KEY, name TEXT NOT NULL, email TEXT UNIQUE NOT NULL)"
            })),
        })
        .await?;

    // Insert data
    let insert_result = service
        .call_tool(CallToolRequestParam {
            name: "insert".into(),
            arguments: Some(object!({
                "conn_id": conn_id.as_str(),
                "query": "INSERT INTO test_users (name, email) VALUES ('Test User', 'test@example.com')"
            })),
        })
        .await?;
    assert!(!insert_result.content.is_empty());

    // Query data
    let query_result = service
        .call_tool(CallToolRequestParam {
            name: "query".into(),
            arguments: Some(object!({
                "conn_id": conn_id.as_str(),
                "query": "SELECT * FROM test_users WHERE email = 'test@example.com'"
            })),
        })
        .await?;
    assert!(!query_result.content.is_empty());

    // Update data
    let update_result = service
        .call_tool(CallToolRequestParam {
            name: "update".into(),
            arguments: Some(object!({
                "conn_id": conn_id.as_str(),
                "query": "UPDATE test_users SET name = 'Updated User' WHERE email = 'test@example.com'"
            })),
        })
        .await?;
    assert!(!update_result.content.is_empty());

    // Delete data
    let delete_result = service
        .call_tool(CallToolRequestParam {
            name: "delete".into(),
            arguments: Some(object!({
                "conn_id": conn_id.as_str(),
                "query": "DELETE FROM test_users WHERE email = 'test@example.com'"
            })),
        })
        .await?;
    assert!(!delete_result.content.is_empty());

    // Drop the test table
    service
        .call_tool(CallToolRequestParam {
            name: "drop_table".into(),
            arguments: Some(object!({
                "conn_id": conn_id.as_str(),
                "table": "test_users"
            })),
        })
        .await?;

    cleanup_service(service, &conn_id).await?;
    Ok(())
}

#[tokio::test]
async fn test_index_operations() -> Result<()> {
    let test_service = setup_service().await?;
    let service = test_service.service;
    let conn_id = test_service.conn_id;

    // Create test table
    service
        .call_tool(CallToolRequestParam {
            name: "create_table".into(),
            arguments: Some(object!({
                "conn_id": conn_id.as_str(),
                "query": "CREATE TABLE IF NOT EXISTS test_users (id SERIAL PRIMARY KEY, name TEXT NOT NULL, email TEXT UNIQUE NOT NULL)"
            })),
        })
        .await?;

    // Create index
    let create_index_result = service
        .call_tool(CallToolRequestParam {
            name: "create_index".into(),
            arguments: Some(object!({
                "conn_id": conn_id.as_str(),
                "query": "CREATE INDEX idx_test_users_email ON test_users (email)"
            })),
        })
        .await?;
    assert!(!create_index_result.content.is_empty());

    // Drop index
    let drop_index_result = service
        .call_tool(CallToolRequestParam {
            name: "drop_index".into(),
            arguments: Some(object!({
                "conn_id": conn_id.as_str(),
                "index": "idx_test_users_email"
            })),
        })
        .await?;
    assert!(!drop_index_result.content.is_empty());

    // Drop the test table
    service
        .call_tool(CallToolRequestParam {
            name: "drop_table".into(),
            arguments: Some(object!({
                "conn_id": conn_id.as_str(),
                "table": "test_users"
            })),
        })
        .await?;

    cleanup_service(service, &conn_id).await?;
    Ok(())
}

#[tokio::test]
async fn test_type_operations() -> Result<()> {
    let test_service = setup_service().await?;
    let service = test_service.service;
    let conn_id = test_service.conn_id;

    // Create enum type
    let create_type_result = service
        .call_tool(CallToolRequestParam {
            name: "create_type".into(),
            arguments: Some(object!({
                "conn_id": conn_id.as_str(),
                "query": "CREATE TYPE user_role AS ENUM ('admin', 'user', 'guest')"
            })),
        })
        .await?;
    assert!(!create_type_result.content.is_empty());

    // Create a table using the new type
    let create_table_result = service
        .call_tool(CallToolRequestParam {
            name: "create_table".into(),
            arguments: Some(object!({
                "conn_id": conn_id.as_str(),
                "query": "CREATE TABLE test_users (id SERIAL PRIMARY KEY, name TEXT NOT NULL, role user_role NOT NULL)"
            })),
        })
        .await?;
    assert!(!create_table_result.content.is_empty());

    // Insert data using the enum type
    let insert_result = service
        .call_tool(CallToolRequestParam {
            name: "insert".into(),
            arguments: Some(object!({
                "conn_id": conn_id.as_str(),
                "query": "INSERT INTO test_users (name, role) VALUES ('Test Admin', 'admin'), ('Test User', 'user'), ('Test Guest', 'guest')"
            })),
        })
        .await?;
    assert!(!insert_result.content.is_empty());

    // Query data to verify enum type works
    let query_result = service
        .call_tool(CallToolRequestParam {
            name: "query".into(),
            arguments: Some(object!({
                "conn_id": conn_id.as_str(),
                "query": "SELECT * FROM test_users WHERE role = 'admin'"
            })),
        })
        .await?;
    assert!(!query_result.content.is_empty());

    // Drop the test table
    service
        .call_tool(CallToolRequestParam {
            name: "drop_table".into(),
            arguments: Some(object!({
                "conn_id": conn_id.as_str(),
                "table": "test_users"
            })),
        })
        .await?;

    cleanup_service(service, &conn_id).await?;
    Ok(())
}

#[tokio::test]
async fn test_schema_operations() -> Result<()> {
    let test_service = setup_service().await?;
    let service = test_service.service;
    let conn_id = test_service.conn_id;

    // Create a test schema
    let schema_name = "test_schema_ops";
    let create_result = service
        .call_tool(CallToolRequestParam {
            name: "create_schema".into(),
            arguments: Some(object!({
                "conn_id": conn_id.as_str(),
                "schema_name": schema_name
            })),
        })
        .await?;
    assert!(
        create_result.content[0]
            .raw
            .as_text()
            .unwrap()
            .text
            .contains("success")
    );

    cleanup_service(service, &conn_id).await?;
    Ok(())
}
