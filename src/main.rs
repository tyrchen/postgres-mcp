use anyhow::bail;
use arc_swap::ArcSwap;
use rmcp::{
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    schemars, tool,
    transport::stdio,
    Error as McpError, ServerHandler, ServiceExt,
};
use serde::{Deserialize, Serialize};
use sqlparser::dialect::PostgreSqlDialect;
use sqlparser::parser::Parser;
use sqlx::{
    postgres::{PgPool, PgPoolOptions},
    prelude::FromRow,
};
use std::collections::HashMap;
use std::sync::Arc;
use tracing_subscriber::{self, EnvFilter};

#[allow(dead_code)]
#[derive(Debug, Clone)]
struct Conn {
    id: String,
    conn_str: String,
    pool: PgPool,
}

#[derive(Debug, Clone)]
struct Conns {
    inner: Arc<ArcSwap<HashMap<String, Conn>>>,
}

#[derive(Debug, FromRow, Serialize, Deserialize)]
struct JsonRow(serde_json::Value);

impl Conns {
    fn new() -> Self {
        Self {
            inner: Arc::new(ArcSwap::from_pointee(HashMap::new())),
        }
    }

    async fn register(&self, id: String, conn_str: String) -> Result<(), anyhow::Error> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&conn_str)
            .await?;

        let conn = Conn {
            id: id.clone(),
            conn_str,
            pool,
        };
        let mut conns = self.inner.load().as_ref().clone();
        conns.insert(id, conn);
        self.inner.store(Arc::new(conns));
        Ok(())
    }

    fn unregister(&self, id: &str) -> Result<(), anyhow::Error> {
        let mut conns = self.inner.load().as_ref().clone();
        conns.remove(id);
        self.inner.store(Arc::new(conns));
        Ok(())
    }

    fn get(&self, id: &str) -> Result<Conn, anyhow::Error> {
        let conns = self.inner.load();
        conns
            .get(id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Connection not found"))
    }

    async fn query(&self, id: &str, query: &str) -> Result<String, anyhow::Error> {
        let dialect = PostgreSqlDialect {};
        let ast = Parser::parse_sql(&dialect, &query)?;

        // Validate it's a SELECT statement
        if ast.len() != 1 || !matches!(ast[0], sqlparser::ast::Statement::Query(_)) {
            bail!("Only SELECT queries are allowed");
        }

        // TODO: add limit if not present in ast

        let conn = self.get(id)?;
        let result: Vec<JsonRow> = sqlx::query_as(&query).fetch_all(&conn.pool).await?;
        Ok(serde_json::to_string(&result)?)
    }

    async fn insert(&self, id: &str, query: &str) -> Result<u64, anyhow::Error> {
        let dialect = PostgreSqlDialect {};
        let ast = Parser::parse_sql(&dialect, &query)?;

        // Validate it's an INSERT statement
        if ast.len() != 1 || !matches!(ast[0], sqlparser::ast::Statement::Insert(_)) {
            return Err(anyhow::anyhow!("Only INSERT statements are allowed"));
        }

        let conn = self.get(id)?;
        let result = sqlx::query(&query).execute(&conn.pool).await?;
        Ok(result.rows_affected())
    }

    async fn update(&self, id: &str, query: &str) -> Result<u64, anyhow::Error> {
        let dialect = PostgreSqlDialect {};
        let ast = Parser::parse_sql(&dialect, &query)?;

        // Validate it's an UPDATE statement
        if ast.len() != 1 || !matches!(ast[0], sqlparser::ast::Statement::Update { .. }) {
            return Err(anyhow::anyhow!("Only UPDATE statements are allowed"));
        }

        let conn = self.get(id)?;
        let result = sqlx::query(&query).execute(&conn.pool).await?;
        Ok(result.rows_affected())
    }

    async fn delete(&self, id: &str, table_name: &str, pk: &str) -> Result<u64, anyhow::Error> {
        let conn = self.get(id)?;
        let query = format!("DELETE FROM {} WHERE id = $1", table_name);
        let result = sqlx::query(&query).bind(pk).execute(&conn.pool).await?;

        Ok(result.rows_affected())
    }

    async fn create_table(&self, id: &str, create_sql: &str) -> Result<u64, anyhow::Error> {
        let dialect = PostgreSqlDialect {};
        let ast = Parser::parse_sql(&dialect, &create_sql)?;

        // Validate it's a CREATE TABLE statement
        if ast.len() != 1 || !matches!(ast[0], sqlparser::ast::Statement::CreateTable(_)) {
            return Err(anyhow::anyhow!("Only CREATE TABLE statements are allowed"));
        }

        let conn = self.get(id)?;
        let result = sqlx::query(&create_sql).execute(&conn.pool).await?;
        Ok(result.rows_affected())
    }

    async fn drop_table(&self, id: &str, table_name: &str) -> Result<u64, anyhow::Error> {
        let conn = self.get(id)?;
        let query = "DROP TABLE IF EXISTS $1";
        let result = sqlx::query(&query)
            .bind(table_name)
            .execute(&conn.pool)
            .await?;
        Ok(result.rows_affected())
    }

    async fn create_index(&self, id: &str, create_index_sql: &str) -> Result<u64, anyhow::Error> {
        let dialect = PostgreSqlDialect {};
        let ast = Parser::parse_sql(&dialect, &create_index_sql)?;

        // Validate it's a CREATE INDEX statement
        if ast.len() != 1 || !matches!(ast[0], sqlparser::ast::Statement::CreateIndex(_)) {
            return Err(anyhow::anyhow!("Only CREATE INDEX statements are allowed"));
        }

        let conn = self.get(id)?;
        let result = sqlx::query(&create_index_sql).execute(&conn.pool).await?;
        Ok(result.rows_affected())
    }

    async fn drop_index(&self, id: &str, index_name: &str) -> Result<u64, anyhow::Error> {
        let conn = self.get(id)?;
        let query = "DROP INDEX IF EXISTS $1";
        let result = sqlx::query(&query)
            .bind(index_name)
            .execute(&conn.pool)
            .await?;
        Ok(result.rows_affected())
    }

    async fn list_tables(&self, id: &str, schema: &str) -> Result<String, anyhow::Error> {
        let conn = self.get(id)?;
        let query = r#"
            SELECT
                t.table_name,
                obj_description(format('%s.%s', t.table_schema, t.table_name)::regclass::oid) as description,
                pg_stat_get_tuples_inserted(format('%s.%s', t.table_schema, t.table_name)::regclass::oid) as total_rows
            FROM information_schema.tables t
            WHERE
                t.table_schema = $1
                AND t.table_type = 'BASE TABLE'
            ORDER BY t.table_name"#;
        let result: Vec<JsonRow> = sqlx::query_as(query)
            .bind(schema)
            .fetch_all(&conn.pool)
            .await?;
        Ok(serde_json::to_string(&result)?)
    }

    async fn describe(&self, id: &str, table_name: &str) -> Result<String, anyhow::Error> {
        let conn = self.get(id)?;
        let (schema, table) = table_name.split_once('.').unwrap_or(("public", table_name));
        let query = r#"SELECT
                c.column_name,
                c.data_type,
                c.is_nullable,
                c.column_default,
                col_description(format('%s.%s', c.table_schema, c.table_name)::regclass::oid, c.ordinal_position) as description
            FROM information_schema.columns c
            WHERE
                c.table_schema = $1 AND
                c.table_name = $2
            ORDER BY c.ordinal_position"#;
        let result: Vec<JsonRow> = sqlx::query_as(&query)
            .bind(schema)
            .bind(table)
            .fetch_all(&conn.pool)
            .await?;
        Ok(serde_json::to_string(&result)?)
    }
}

#[derive(Debug, Clone)]
pub struct PostgresMCP {
    conns: Conns,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RegisterRequest {
    #[schemars(description = "Postgres connection string")]
    pub conn_str: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct QueryRequest {
    #[schemars(description = "Connection ID")]
    pub conn_id: String,
    #[schemars(description = "SQL query")]
    pub query: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DeleteRequest {
    #[schemars(description = "Connection ID")]
    pub conn_id: String,
    #[schemars(description = "Table name")]
    pub table_name: String,
    #[schemars(description = "Primary key")]
    pub pk: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct TableRequest {
    #[schemars(description = "Connection ID")]
    pub conn_id: String,
    #[schemars(description = "Table name")]
    pub table_name: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CreateRequest {
    #[schemars(description = "Connection ID")]
    pub conn_id: String,
    #[schemars(description = "SQL create statement")]
    pub create_sql: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct IndexRequest {
    #[schemars(description = "Connection ID")]
    pub conn_id: String,
    #[schemars(description = "SQL create index statement")]
    pub create_index_sql: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DropIndexRequest {
    #[schemars(description = "Connection ID")]
    pub conn_id: String,
    #[schemars(description = "Index name")]
    pub index_name: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListTablesRequest {
    #[schemars(description = "Connection ID")]
    pub conn_id: String,
    #[schemars(description = "Schema name")]
    pub schema: String,
}

#[tool(tool_box)]
impl PostgresMCP {
    pub fn new() -> Self {
        Self {
            conns: Conns::new(),
        }
    }

    #[tool(description = "Register a new Postgres connection")]
    async fn register(
        &self,
        #[tool(aggr)] req: RegisterRequest,
    ) -> Result<CallToolResult, McpError> {
        let id = uuid::Uuid::new_v4().to_string();
        if let Err(e) = self.conns.register(id.clone(), req.conn_str).await {
            return Err(McpError::internal_error(e.to_string(), None));
        }
        Ok(CallToolResult::success(vec![Content::text(id)]))
    }

    #[tool(description = "Unregister a Postgres connection")]
    async fn unregister(
        &self,
        #[tool(aggr)] req: QueryRequest,
    ) -> Result<CallToolResult, McpError> {
        if let Err(e) = self.conns.unregister(&req.conn_id) {
            return Err(McpError::internal_error(e.to_string(), None));
        }
        Ok(CallToolResult::success(vec![Content::text(
            "success".to_string(),
        )]))
    }

    #[tool(description = "Execute a SELECT query")]
    async fn query(&self, #[tool(aggr)] req: QueryRequest) -> Result<CallToolResult, McpError> {
        match self.conns.query(&req.conn_id, &req.query).await {
            Ok(result) => Ok(CallToolResult::success(vec![Content::text(result)])),
            Err(e) => Err(McpError::internal_error(e.to_string(), None)),
        }
    }

    #[tool(description = "Execute an INSERT statement")]
    async fn insert(&self, #[tool(aggr)] req: QueryRequest) -> Result<CallToolResult, McpError> {
        match self.conns.insert(&req.conn_id, &req.query).await {
            Ok(result) => Ok(CallToolResult::success(vec![Content::text(
                result.to_string(),
            )])),
            Err(e) => Err(McpError::internal_error(e.to_string(), None)),
        }
    }

    #[tool(description = "Execute an UPDATE statement")]
    async fn update(&self, #[tool(aggr)] req: QueryRequest) -> Result<CallToolResult, McpError> {
        match self.conns.update(&req.conn_id, &req.query).await {
            Ok(result) => Ok(CallToolResult::success(vec![Content::text(
                result.to_string(),
            )])),
            Err(e) => Err(McpError::internal_error(e.to_string(), None)),
        }
    }

    #[tool(description = "Delete a row by primary key")]
    async fn delete(&self, #[tool(aggr)] req: DeleteRequest) -> Result<CallToolResult, McpError> {
        match self
            .conns
            .delete(&req.conn_id, &req.table_name, &req.pk)
            .await
        {
            Ok(result) => Ok(CallToolResult::success(vec![Content::text(
                result.to_string(),
            )])),
            Err(e) => Err(McpError::internal_error(e.to_string(), None)),
        }
    }

    #[tool(description = "Describe a table")]
    async fn describe(&self, #[tool(aggr)] req: TableRequest) -> Result<CallToolResult, McpError> {
        match self.conns.describe(&req.conn_id, &req.table_name).await {
            Ok(result) => Ok(CallToolResult::success(vec![Content::text(result)])),
            Err(e) => Err(McpError::internal_error(e.to_string(), None)),
        }
    }

    #[tool(description = "Create a new table")]
    async fn create(&self, #[tool(aggr)] req: CreateRequest) -> Result<CallToolResult, McpError> {
        match self.conns.create_table(&req.conn_id, &req.create_sql).await {
            Ok(result) => Ok(CallToolResult::success(vec![Content::text(
                result.to_string(),
            )])),
            Err(e) => Err(McpError::internal_error(e.to_string(), None)),
        }
    }

    #[tool(description = "Drop a table")]
    async fn drop(&self, #[tool(aggr)] req: TableRequest) -> Result<CallToolResult, McpError> {
        match self.conns.drop_table(&req.conn_id, &req.table_name).await {
            Ok(result) => Ok(CallToolResult::success(vec![Content::text(
                result.to_string(),
            )])),
            Err(e) => Err(McpError::internal_error(e.to_string(), None)),
        }
    }

    #[tool(description = "Create an index")]
    async fn create_index(
        &self,
        #[tool(aggr)] req: IndexRequest,
    ) -> Result<CallToolResult, McpError> {
        match self
            .conns
            .create_index(&req.conn_id, &req.create_index_sql)
            .await
        {
            Ok(result) => Ok(CallToolResult::success(vec![Content::text(
                result.to_string(),
            )])),
            Err(e) => Err(McpError::internal_error(e.to_string(), None)),
        }
    }

    #[tool(description = "Drop an index")]
    async fn drop_index(
        &self,
        #[tool(aggr)] req: DropIndexRequest,
    ) -> Result<CallToolResult, McpError> {
        match self.conns.drop_index(&req.conn_id, &req.index_name).await {
            Ok(result) => Ok(CallToolResult::success(vec![Content::text(
                result.to_string(),
            )])),
            Err(e) => Err(McpError::internal_error(e.to_string(), None)),
        }
    }

    #[tool(description = "List tables")]
    async fn list_tables(
        &self,
        #[tool(aggr)] req: ListTablesRequest,
    ) -> Result<CallToolResult, McpError> {
        match self.conns.list_tables(&req.conn_id, &req.schema).await {
            Ok(result) => Ok(CallToolResult::success(vec![Content::text(result)])),
            Err(e) => Err(McpError::internal_error(e.to_string(), None)),
        }
    }
}

#[tool(tool_box)]
impl ServerHandler for PostgresMCP {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("Postgres MCP server for database operations".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Initialize the tracing subscriber
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::DEBUG.into()))
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    tracing::info!("Starting Postgres MCP server");

    // Create an instance of our PostgresMCP router
    let service = PostgresMCP::new().serve(stdio()).await.inspect_err(|e| {
        tracing::error!("serving error: {:?}", e);
    })?;

    service.waiting().await?;
    Ok(())
}
