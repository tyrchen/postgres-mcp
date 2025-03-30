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
    #[schemars(description = "SQL query")]
    pub query: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct InsertRequest {
    #[schemars(description = "Connection ID")]
    pub conn_id: String,
    #[schemars(description = "SQL insert statement")]
    pub query: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct UpdateRequest {
    #[schemars(description = "Connection ID")]
    pub conn_id: String,
    #[schemars(description = "SQL update statement")]
    pub query: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct DeleteRequest {
    #[schemars(description = "Connection ID")]
    pub conn_id: String,
    #[schemars(description = "SQL delete statement")]
    pub query: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CreateTableRequest {
    #[schemars(description = "Connection ID")]
    pub conn_id: String,
    #[schemars(description = "SQL create table statement")]
    pub query: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct DropTableRequest {
    #[schemars(description = "Connection ID")]
    pub conn_id: String,
    #[schemars(description = "Table name")]
    pub table: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CreateIndexRequest {
    #[schemars(description = "Connection ID")]
    pub conn_id: String,
    #[schemars(description = "SQL create index statement")]
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
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(id)]))
    }

    #[tool(description = "Unregister a Postgres connection")]
    async fn unregister(
        &self,
        #[tool(aggr)] req: UnregisterRequest,
    ) -> Result<CallToolResult, McpError> {
        self.conns
            .unregister(req.conn_id)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
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
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(result)]))
    }

    #[tool(description = "Execute an INSERT statement")]
    async fn insert(&self, #[tool(aggr)] req: InsertRequest) -> Result<CallToolResult, McpError> {
        let result = self
            .conns
            .insert(&req.conn_id, &req.query)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(result)]))
    }

    #[tool(description = "Execute an UPDATE statement")]
    async fn update(&self, #[tool(aggr)] req: UpdateRequest) -> Result<CallToolResult, McpError> {
        let result = self
            .conns
            .update(&req.conn_id, &req.query)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(result)]))
    }

    #[tool(description = "Delete a row from a table")]
    async fn delete(&self, #[tool(aggr)] req: DeleteRequest) -> Result<CallToolResult, McpError> {
        let result = self
            .conns
            .delete(&req.conn_id, &req.query)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
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
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
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
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
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
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
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
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
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
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(CallToolResult::success(vec![Content::text(result)]))
    }

    #[tool(description = "List all tables")]
    async fn list_tables(
        &self,
        #[tool(aggr)] req: ListTablesRequest,
    ) -> Result<CallToolResult, McpError> {
        let result = self
            .conns
            .list_tables(&req.conn_id, &req.schema)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
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
