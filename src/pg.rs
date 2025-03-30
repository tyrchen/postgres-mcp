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
struct JsonRow {
    ret: sqlx::types::Json<serde_json::Value>,
}

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

        let query = format!(
            "WITH data AS ({}) SELECT JSON_AGG(data.*) as ret FROM data;",
            query
        );

        let ret = sqlx::query_as::<_, JsonRow>(&query)
            .fetch_one(&conn.pool)
            .await?;

        Ok(serde_json::to_string(&ret.ret)?)
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

    pub(crate) async fn delete(&self, id: &str, query: &str) -> Result<String, Error> {
        let conns = self.inner.load();
        let conn = conns
            .get(id)
            .ok_or_else(|| anyhow::anyhow!("Connection not found"))?;

        // Validate query is a DELETE statement
        let dialect = sqlparser::dialect::PostgreSqlDialect {};
        let ast = sqlparser::parser::Parser::parse_sql(&dialect, query)?;
        if ast.len() != 1 || !matches!(ast[0], sqlparser::ast::Statement::Delete { .. }) {
            return Err(anyhow::anyhow!("Only DELETE statements are allowed"));
        }

        let result = sqlx::query(query).execute(&conn.pool).await?;

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

        let query = r#"
        WITH data AS (
          SELECT column_name, data_type, character_maximum_length, column_default, is_nullable
          FROM information_schema.columns
          WHERE table_name = $1
          ORDER BY ordinal_position)
        SELECT JSON_AGG(data.*) as ret FROM data"#;

        let ret = sqlx::query_as::<_, JsonRow>(query)
            .bind(table)
            .fetch_one(&conn.pool)
            .await?;

        Ok(serde_json::to_string(&ret.ret)?)
    }

    pub(crate) async fn list_tables(&self, id: &str, schema: &str) -> Result<String, Error> {
        let conns = self.inner.load();
        let conn = conns
            .get(id)
            .ok_or_else(|| anyhow::anyhow!("Connection not found"))?;

        let query = r#"
        WITH data AS (
          SELECT
                t.table_name,
                obj_description(format('%s.%s', t.table_schema, t.table_name)::regclass::oid) as description,
                pg_stat_get_tuples_inserted(format('%s.%s', t.table_schema, t.table_name)::regclass::oid) as total_rows
            FROM information_schema.tables t
            WHERE
                t.table_schema = $1
                AND t.table_type = 'BASE TABLE'
            ORDER BY t.table_name
        )
        SELECT JSON_AGG(data.*) as ret FROM data"#;
        let ret = sqlx::query_as::<_, JsonRow>(query)
            .bind(schema)
            .fetch_one(&conn.pool)
            .await?;

        Ok(serde_json::to_string(&ret.ret)?)
    }
}

impl Default for Conns {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx_db_tester::TestPg;

    const TEST_CONN_STR: &str = "postgres://postgres:postgres@localhost:5432/postgres";

    async fn setup_test_db() -> (TestPg, String) {
        let tdb = TestPg::new(
            TEST_CONN_STR.to_string(),
            std::path::Path::new("./fixtures/migrations"),
        );
        let pool = tdb.get_pool().await;

        // Ensure migrations are applied
        sqlx::query("SELECT * FROM test_table LIMIT 1")
            .execute(&pool)
            .await
            .unwrap();

        let conn_str = tdb.url();

        (tdb, conn_str)
    }

    #[tokio::test]
    async fn test_connection_management() {
        let (_tdb, conn_str) = setup_test_db().await;
        let conns = Conns::new();

        // Test register
        let id = conns.register(conn_str.clone()).await.unwrap();
        assert!(!id.is_empty());

        // Test unregister
        assert!(conns.unregister(id.clone()).is_ok());
        assert!(conns.unregister(id).is_err());
    }

    #[tokio::test]
    async fn test_table_operations() {
        let (_tdb, conn_str) = setup_test_db().await;
        let conns = Conns::new();
        let id = conns.register(conn_str).await.unwrap();

        // Test list tables
        let tables = conns.list_tables(&id, "public").await.unwrap();
        assert!(tables.contains("test_table"));

        // Test describe table
        let description = conns.describe(&id, "test_table").await.unwrap();
        assert!(description.contains("id"));
        assert!(description.contains("name"));
        assert!(description.contains("created_at"));

        // Test drop table
        assert_eq!(
            conns.drop_table(&id, "test_table").await.unwrap(),
            "success"
        );
    }

    #[tokio::test]
    async fn test_data_operations() {
        let (_tdb, conn_str) = setup_test_db().await;
        let conns = Conns::new();
        let id = conns.register(conn_str).await.unwrap();

        // Test query
        let query = "SELECT * FROM test_table ORDER BY id";
        let result = conns.query(&id, query).await.unwrap();
        assert!(result.contains("test1"));
        assert!(result.contains("test2"));
        assert!(result.contains("test3"));

        // Test insert
        let insert = "INSERT INTO test_table (name) VALUES ('test4')";
        let result = conns.insert(&id, insert).await.unwrap();
        assert!(result.contains("rows_affected: 1"));

        // Test update
        let update = "UPDATE test_table SET name = 'updated' WHERE name = 'test1'";
        let result = conns.update(&id, update).await.unwrap();
        assert!(result.contains("rows_affected: 1"));

        // Test delete
        let result = conns
            .delete(&id, "DELETE FROM test_table WHERE name = 'updated'")
            .await
            .unwrap();
        assert!(result.contains("rows_affected: 1"));
    }

    #[tokio::test]
    async fn test_index_operations() {
        let (_tdb, conn_str) = setup_test_db().await;
        let conns = Conns::new();
        let id = conns.register(conn_str).await.unwrap();

        // Test create index
        let create_index = "CREATE INDEX idx_test_table_new ON test_table (name, created_at)";
        assert_eq!(
            conns.create_index(&id, create_index).await.unwrap(),
            "success"
        );

        // Test drop index
        assert_eq!(
            conns.drop_index(&id, "idx_test_table_new").await.unwrap(),
            "success"
        );
    }

    #[tokio::test]
    async fn test_sql_validation() {
        let (_tdb, conn_str) = setup_test_db().await;
        let conns = Conns::new();
        let id = conns.register(conn_str).await.unwrap();

        // Test invalid SELECT
        let invalid_query = "INSERT INTO test_table VALUES (1)";
        assert!(conns.query(&id, invalid_query).await.is_err());

        // Test invalid INSERT
        let invalid_insert = "SELECT * FROM test_table";
        assert!(conns.insert(&id, invalid_insert).await.is_err());

        // Test invalid UPDATE
        let invalid_update = "DELETE FROM test_table";
        assert!(conns.update(&id, invalid_update).await.is_err());

        // Test invalid CREATE TABLE
        let invalid_create = "CREATE INDEX idx_test ON test_table (id)";
        assert!(conns.create_table(&id, invalid_create).await.is_err());

        // Test invalid CREATE INDEX
        let invalid_index = "CREATE TABLE test (id INT)";
        assert!(conns.create_index(&id, invalid_index).await.is_err());
    }
}
