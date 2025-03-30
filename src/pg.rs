use anyhow::Error;
use arc_swap::ArcSwap;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPool;
use std::collections::HashMap;
use std::sync::Arc;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) struct Conn {
    pub(crate) id: String,
    pub(crate) conn_str: String,
    pub(crate) pool: PgPool,
}

#[derive(Debug, Clone)]
pub struct Conns {
    pub(crate) inner: Arc<ArcSwap<HashMap<String, Conn>>>,
}

#[derive(Debug, Clone)]
pub struct PgMcp {
    pub(crate) conns: Conns,
}

#[derive(Debug, sqlx::FromRow, Serialize, Deserialize)]
struct JsonRow(serde_json::Value);

impl Conns {
    pub(crate) fn new() -> Self {
        Self {
            inner: Arc::new(ArcSwap::new(Arc::new(HashMap::new()))),
        }
    }

    pub(crate) async fn register(&self, conn_str: String) -> Result<String, Error> {
        let pool = PgPool::connect(&conn_str).await?;
        let id = uuid::Uuid::new_v4().to_string();
        let conn = Conn {
            id: id.clone(),
            conn_str: conn_str.clone(),
            pool,
        };

        let mut conns = self.inner.load().as_ref().clone();
        conns.insert(id.clone(), conn);
        self.inner.store(Arc::new(conns));

        Ok(id)
    }

    pub(crate) fn unregister(&self, id: String) -> Result<(), Error> {
        let mut conns = self.inner.load().as_ref().clone();
        if conns.remove(&id).is_none() {
            return Err(anyhow::anyhow!("Connection not found"));
        }
        self.inner.store(Arc::new(conns));
        Ok(())
    }

    pub(crate) async fn query(&self, id: &str, query: &str) -> Result<String, Error> {
        let conns = self.inner.load();
        let conn = conns
            .get(id)
            .ok_or_else(|| anyhow::anyhow!("Connection not found"))?;

        // Validate query is a SELECT statement
        let dialect = sqlparser::dialect::PostgreSqlDialect {};
        let ast = sqlparser::parser::Parser::parse_sql(&dialect, query)?;
        if ast.len() != 1 || !matches!(ast[0], sqlparser::ast::Statement::Query(_)) {
            return Err(anyhow::anyhow!("Only SELECT queries are allowed"));
        }

        let rows = sqlx::query_as::<_, JsonRow>(query)
            .fetch_all(&conn.pool)
            .await?;

        Ok(serde_json::to_string(&rows)?)
    }

    pub(crate) async fn insert(&self, id: &str, query: &str) -> Result<String, Error> {
        let conns = self.inner.load();
        let conn = conns
            .get(id)
            .ok_or_else(|| anyhow::anyhow!("Connection not found"))?;

        // Validate query is an INSERT statement
        let dialect = sqlparser::dialect::PostgreSqlDialect {};
        let ast = sqlparser::parser::Parser::parse_sql(&dialect, query)?;
        if ast.len() != 1 || !matches!(ast[0], sqlparser::ast::Statement::Insert { .. }) {
            return Err(anyhow::anyhow!("Only INSERT statements are allowed"));
        }

        let result = sqlx::query(query).execute(&conn.pool).await?;

        Ok(format!(
            "success, rows_affected: {}",
            result.rows_affected()
        ))
    }

    pub(crate) async fn update(&self, id: &str, query: &str) -> Result<String, Error> {
        let conns = self.inner.load();
        let conn = conns
            .get(id)
            .ok_or_else(|| anyhow::anyhow!("Connection not found"))?;

        // Validate query is an UPDATE statement
        let dialect = sqlparser::dialect::PostgreSqlDialect {};
        let ast = sqlparser::parser::Parser::parse_sql(&dialect, query)?;
        if ast.len() != 1 || !matches!(ast[0], sqlparser::ast::Statement::Update { .. }) {
            return Err(anyhow::anyhow!("Only UPDATE statements are allowed"));
        }

        let result = sqlx::query(query).execute(&conn.pool).await?;

        Ok(format!(
            "success, rows_affected: {}",
            result.rows_affected()
        ))
    }

    pub(crate) async fn delete(&self, id: &str, table: &str, pk: &str) -> Result<String, Error> {
        let conns = self.inner.load();
        let conn = conns
            .get(id)
            .ok_or_else(|| anyhow::anyhow!("Connection not found"))?;

        let query = format!("DELETE FROM {} WHERE id = $1", table);
        let result = sqlx::query(&query).bind(pk).execute(&conn.pool).await?;

        Ok(format!(
            "success, rows_affected: {}",
            result.rows_affected()
        ))
    }

    pub(crate) async fn create_table(&self, id: &str, query: &str) -> Result<String, Error> {
        let conns = self.inner.load();
        let conn = conns
            .get(id)
            .ok_or_else(|| anyhow::anyhow!("Connection not found"))?;

        // Validate query is a CREATE TABLE statement
        let dialect = sqlparser::dialect::PostgreSqlDialect {};
        let ast = sqlparser::parser::Parser::parse_sql(&dialect, query)?;
        if ast.len() != 1 || !matches!(ast[0], sqlparser::ast::Statement::CreateTable { .. }) {
            return Err(anyhow::anyhow!("Only CREATE TABLE statements are allowed"));
        }

        sqlx::query(query).execute(&conn.pool).await?;

        Ok("success".to_string())
    }

    pub(crate) async fn drop_table(&self, id: &str, table: &str) -> Result<String, Error> {
        let conns = self.inner.load();
        let conn = conns
            .get(id)
            .ok_or_else(|| anyhow::anyhow!("Connection not found"))?;

        let query = format!("DROP TABLE {}", table);
        sqlx::query(&query).execute(&conn.pool).await?;

        Ok("success".to_string())
    }

    pub(crate) async fn create_index(&self, id: &str, query: &str) -> Result<String, Error> {
        let conns = self.inner.load();
        let conn = conns
            .get(id)
            .ok_or_else(|| anyhow::anyhow!("Connection not found"))?;

        // Validate query is a CREATE INDEX statement
        let dialect = sqlparser::dialect::PostgreSqlDialect {};
        let ast = sqlparser::parser::Parser::parse_sql(&dialect, query)?;
        if ast.len() != 1 || !matches!(ast[0], sqlparser::ast::Statement::CreateIndex { .. }) {
            return Err(anyhow::anyhow!("Only CREATE INDEX statements are allowed"));
        }

        sqlx::query(query).execute(&conn.pool).await?;

        Ok("success".to_string())
    }

    pub(crate) async fn drop_index(&self, id: &str, index: &str) -> Result<String, Error> {
        let conns = self.inner.load();
        let conn = conns
            .get(id)
            .ok_or_else(|| anyhow::anyhow!("Connection not found"))?;

        let query = format!("DROP INDEX {}", index);
        sqlx::query(&query).execute(&conn.pool).await?;

        Ok("success".to_string())
    }

    pub(crate) async fn describe(&self, id: &str, table: &str) -> Result<String, Error> {
        let conns = self.inner.load();
        let conn = conns
            .get(id)
            .ok_or_else(|| anyhow::anyhow!("Connection not found"))?;

        let query = format!(
            "SELECT column_name, data_type, character_maximum_length, column_default, is_nullable
             FROM information_schema.columns
             WHERE table_name = $1
             ORDER BY ordinal_position",
        );

        let rows = sqlx::query_as::<_, JsonRow>(&query)
            .bind(table)
            .fetch_all(&conn.pool)
            .await?;

        Ok(serde_json::to_string(&rows)?)
    }

    pub(crate) async fn list_tables(&self, id: &str) -> Result<String, Error> {
        let conns = self.inner.load();
        let conn = conns
            .get(id)
            .ok_or_else(|| anyhow::anyhow!("Connection not found"))?;

        let query =
            "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public'";
        let rows = sqlx::query_as::<_, JsonRow>(query)
            .fetch_all(&conn.pool)
            .await?;

        Ok(serde_json::to_string(&rows)?)
    }
}
