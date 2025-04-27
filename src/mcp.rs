use crate::pg::PgMcpError;
use crate::{Conns, PgMcp};
use anyhow::Result;
use rmcp::{
    Error as McpError, ServerHandler,
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    schemars, tool,
};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct RegisterRequest {
    #[schemars(description = "Postgres connection string")]
    pub conn_str: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct UnregisterRequest {
    #[schemars(description = "Connection ID to unregister")]
    pub conn_id: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct QueryRequest {
    #[schemars(description = "Connection ID")]
    pub conn_id: String,
    #[schemars(
        description = "Single SQL query, could return multiple rows. Caller should properly limit the number of rows returned."
    )]
    pub query: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct InsertRequest {
    #[schemars(description = "Connection ID")]
    pub conn_id: String,
    #[schemars(
        description = "Single SQL insert statement, but multiple rows for the same table are allowed"
    )]
    pub query: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct UpdateRequest {
    #[schemars(description = "Connection ID")]
    pub conn_id: String,
    #[schemars(
        description = "Single SQL update statement, could update multiple rows for the same table based on the WHERE clause"
    )]
    pub query: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct DeleteRequest {
    #[schemars(description = "Connection ID")]
    pub conn_id: String,
    #[schemars(
        description = "Single SQL delete statement, could delete multiple rows for the same table based on the WHERE clause"
    )]
    pub query: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CreateTableRequest {
    #[schemars(description = "Connection ID")]
    pub conn_id: String,
    #[schemars(description = "Single SQL create table statement")]
    pub query: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct DropTableRequest {
    #[schemars(description = "Connection ID")]
    pub conn_id: String,
    #[schemars(
        description = "Table name. Format: schema.table. If schema is not provided, it will use the current schema."
    )]
    pub table: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CreateIndexRequest {
    #[schemars(description = "Connection ID")]
    pub conn_id: String,
    #[schemars(description = "SingleSQL create index statement")]
    pub query: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct DropIndexRequest {
    #[schemars(description = "Connection ID")]
    pub conn_id: String,
    #[schemars(description = "Index name")]
    pub index: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct DescribeRequest {
    #[schemars(description = "Connection ID")]
    pub conn_id: String,
    #[schemars(description = "Table name")]
    pub table: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ListTablesRequest {
    #[schemars(description = "Connection ID")]
    pub conn_id: String,
    #[schemars(description = "Schema name")]
    pub schema: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CreateSchemaRequest {
    #[schemars(description = "Connection ID")]
    pub conn_id: String,
    #[schemars(description = "Schema name")]
    pub name: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CreateTypeRequest {
    #[schemars(description = "Connection ID")]
    pub conn_id: String,
    #[schemars(description = "Single SQL create type statement")]
    pub query: String,
}

// Helper function to map PgMcpError to McpError
fn map_pg_error(e: PgMcpError) -> McpError {
    match e {
        PgMcpError::ConnectionNotFound(id) => McpError::internal_error(
            format!("Invalid Argument: Connection not found for ID: {}", id),
            None,
        ),
        PgMcpError::ValidationFailed {
            kind,
            query,
            details,
        } => McpError::internal_error(
            format!(
                "Invalid Argument: SQL validation failed for query '{}': {} - {}",
                query, kind, details
            ),
            None,
        ),
        PgMcpError::DatabaseError {
            operation,
            underlying,
        } => McpError::internal_error(
            format!("Database operation '{}' failed: {}", operation, underlying),
            None,
        ),
        PgMcpError::SerializationError(se) => {
            McpError::internal_error(format!("Result serialization failed: {}", se), None)
        }
        PgMcpError::ConnectionError(ce) => {
            McpError::internal_error(format!("Database connection failed: {}", ce), None)
        }
        PgMcpError::InternalError(ie) => {
            McpError::internal_error(format!("Internal error: {}", ie), None)
        }
    }
}

#[tool(tool_box)]
impl PgMcp {
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
        let id = self
            .conns
            .register(req.conn_str)
            .await
            .map_err(map_pg_error)?;
        Ok(CallToolResult::success(vec![Content::text(id)]))
    }

    #[tool(description = "Unregister a Postgres connection")]
    async fn unregister(
        &self,
        #[tool(aggr)] req: UnregisterRequest,
    ) -> Result<CallToolResult, McpError> {
        self.conns.unregister(req.conn_id).map_err(map_pg_error)?;
        Ok(CallToolResult::success(vec![Content::text(
            "success".to_string(),
        )]))
    }

    #[tool(description = "Execute a SELECT query")]
    async fn query(&self, #[tool(aggr)] req: QueryRequest) -> Result<CallToolResult, McpError> {
        let result = self
            .conns
            .query(&req.conn_id, &req.query)
            .await
            .map_err(map_pg_error)?;
        Ok(CallToolResult::success(vec![Content::text(result)]))
    }

    #[tool(description = "Execute an INSERT statement")]
    async fn insert(&self, #[tool(aggr)] req: InsertRequest) -> Result<CallToolResult, McpError> {
        let result = self
            .conns
            .insert(&req.conn_id, &req.query)
            .await
            .map_err(map_pg_error)?;
        Ok(CallToolResult::success(vec![Content::text(result)]))
    }

    #[tool(description = "Execute an UPDATE statement")]
    async fn update(&self, #[tool(aggr)] req: UpdateRequest) -> Result<CallToolResult, McpError> {
        let result = self
            .conns
            .update(&req.conn_id, &req.query)
            .await
            .map_err(map_pg_error)?;
        Ok(CallToolResult::success(vec![Content::text(result)]))
    }

    #[tool(description = "Delete a row from a table")]
    async fn delete(&self, #[tool(aggr)] req: DeleteRequest) -> Result<CallToolResult, McpError> {
        let result = self
            .conns
            .delete(&req.conn_id, &req.query)
            .await
            .map_err(map_pg_error)?;
        Ok(CallToolResult::success(vec![Content::text(result)]))
    }

    #[tool(description = "Create a new table")]
    async fn create_table(
        &self,
        #[tool(aggr)] req: CreateTableRequest,
    ) -> Result<CallToolResult, McpError> {
        let result = self
            .conns
            .create_table(&req.conn_id, &req.query)
            .await
            .map_err(map_pg_error)?;
        Ok(CallToolResult::success(vec![Content::text(result)]))
    }

    #[tool(description = "Drop a table")]
    async fn drop_table(
        &self,
        #[tool(aggr)] req: DropTableRequest,
    ) -> Result<CallToolResult, McpError> {
        let result = self
            .conns
            .drop_table(&req.conn_id, &req.table)
            .await
            .map_err(map_pg_error)?;
        Ok(CallToolResult::success(vec![Content::text(result)]))
    }

    #[tool(description = "Create an index")]
    async fn create_index(
        &self,
        #[tool(aggr)] req: CreateIndexRequest,
    ) -> Result<CallToolResult, McpError> {
        let result = self
            .conns
            .create_index(&req.conn_id, &req.query)
            .await
            .map_err(map_pg_error)?;
        Ok(CallToolResult::success(vec![Content::text(result)]))
    }

    #[tool(description = "Drop an index")]
    async fn drop_index(
        &self,
        #[tool(aggr)] req: DropIndexRequest,
    ) -> Result<CallToolResult, McpError> {
        let result = self
            .conns
            .drop_index(&req.conn_id, &req.index)
            .await
            .map_err(map_pg_error)?;
        Ok(CallToolResult::success(vec![Content::text(result)]))
    }

    #[tool(description = "Describe a table")]
    async fn describe(
        &self,
        #[tool(aggr)] req: DescribeRequest,
    ) -> Result<CallToolResult, McpError> {
        let result = self
            .conns
            .describe(&req.conn_id, &req.table)
            .await
            .map_err(map_pg_error)?;
        Ok(CallToolResult::success(vec![Content::text(result)]))
    }

    #[tool(description = "List tables in a schema")]
    async fn list_tables(
        &self,
        #[tool(aggr)] req: ListTablesRequest,
    ) -> Result<CallToolResult, McpError> {
        let result = self
            .conns
            .list_tables(&req.conn_id, &req.schema)
            .await
            .map_err(map_pg_error)?;
        Ok(CallToolResult::success(vec![Content::text(result)]))
    }

    #[tool(description = "Create a new schema")]
    async fn create_schema(
        &self,
        #[tool(aggr)] req: CreateSchemaRequest,
    ) -> Result<CallToolResult, McpError> {
        let result = self
            .conns
            .create_schema(&req.conn_id, &req.name)
            .await
            .map_err(map_pg_error)?;
        Ok(CallToolResult::success(vec![Content::text(result)]))
    }

    #[tool(description = "Create a new type")]
    async fn create_type(
        &self,
        #[tool(aggr)] req: CreateTypeRequest,
    ) -> Result<CallToolResult, McpError> {
        let result = self
            .conns
            .create_type(&req.conn_id, &req.query)
            .await
            .map_err(map_pg_error)?;
        Ok(CallToolResult::success(vec![Content::text(result)]))
    }
}

#[tool(tool_box)]
impl ServerHandler for PgMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "A Postgres MCP server that allows AI agents to interact with Postgres databases"
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

impl Default for PgMcp {
    fn default() -> Self {
        Self::new()
    }
}
