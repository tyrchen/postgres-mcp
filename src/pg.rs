use anyhow::Error;
use arc_swap::ArcSwap;
use serde::{Deserialize, Serialize};
use sqlparser::ast::Statement;
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

        let query = validate_sql(
            query,
            |stmt| matches!(stmt, Statement::Query(_)),
            "Only SELECT queries are allowed",
        )?;

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

        let query = validate_sql(
            query,
            |stmt| matches!(stmt, Statement::Insert { .. }),
            "Only INSERT statements are allowed",
        )?;

        let result = sqlx::query(&query).execute(&conn.pool).await?;

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

        let query = validate_sql(
            query,
            |stmt| matches!(stmt, Statement::Update { .. }),
            "Only UPDATE statements are allowed",
        )?;

        let result = sqlx::query(&query).execute(&conn.pool).await?;

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

        let query = validate_sql(
            query,
            |stmt| matches!(stmt, Statement::Delete { .. }),
            "Only DELETE statements are allowed",
        )?;

        let result = sqlx::query(&query).execute(&conn.pool).await?;

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

        let query = validate_sql(
            query,
            |stmt| matches!(stmt, Statement::CreateTable { .. }),
            "Only CREATE TABLE statements are allowed",
        )?;

        sqlx::query(&query).execute(&conn.pool).await?;

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

        let query = validate_sql(
            query,
            |stmt| matches!(stmt, Statement::CreateIndex { .. }),
            "Only CREATE INDEX statements are allowed",
        )?;

        sqlx::query(&query).execute(&conn.pool).await?;

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

    pub(crate) async fn create_schema(&self, id: &str, schema_name: &str) -> Result<String, Error> {
        let conns = self.inner.load();
        let conn = conns
            .get(id)
            .ok_or_else(|| anyhow::anyhow!("Connection not found"))?;

        // Basic validation for schema name to prevent obvious SQL injection
        // A more robust validation might be needed depending on security requirements
        if !schema_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err(anyhow::anyhow!("Invalid schema name"));
        }

        let query = format!("CREATE SCHEMA \"{}\";", schema_name);
        sqlx::query(&query).execute(&conn.pool).await?;

        Ok("success".to_string())
    }

    pub(crate) async fn create_type(&self, id: &str, query: &str) -> Result<String, Error> {
        let conns = self.inner.load();
        let conn = conns
            .get(id)
            .ok_or_else(|| anyhow::anyhow!("Connection not found"))?;

        let query = validate_sql(
            query,
            |stmt| matches!(stmt, Statement::CreateType { .. }),
            "Only CREATE TYPE statements are allowed",
        )?;

        sqlx::query(&query).execute(&conn.pool).await?;

        Ok("success".to_string())
    }
}

impl Default for Conns {
    fn default() -> Self {
        Self::new()
    }
}

fn validate_sql<F>(query: &str, validator: F, error_msg: &'static str) -> Result<String, Error>
where
    F: Fn(&Statement) -> bool,
{
    let dialect = sqlparser::dialect::PostgreSqlDialect {};
    let ast = sqlparser::parser::Parser::parse_sql(&dialect, query)?;
    if ast.len() != 1 || !validator(&ast[0]) {
        return Err(anyhow::anyhow!(error_msg));
    }
    Ok(ast[0].to_string())
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
    async fn register_unregister_should_work() {
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
    async fn list_tables_describe_should_work() {
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
    }

    #[tokio::test]
    async fn create_table_drop_table_should_work() {
        let (_tdb, conn_str) = setup_test_db().await;
        let conns = Conns::new();
        let id = conns.register(conn_str).await.unwrap();

        // Test create table
        let create_table = "CREATE TABLE test_table2 (id SERIAL PRIMARY KEY, name TEXT)";
        assert_eq!(
            conns.create_table(&id, create_table).await.unwrap(),
            "success"
        );

        // Test drop table
        assert_eq!(
            conns.drop_table(&id, "test_table2").await.unwrap(),
            "success"
        );

        // Test drop table again
        assert!(conns.drop_table(&id, "test_table2").await.is_err());
    }

    #[tokio::test]
    async fn query_insert_update_delete_should_work() {
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
    async fn create_index_drop_index_should_work() {
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
    async fn sql_validation_should_work() {
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

    #[tokio::test]
    async fn create_type_should_work() {
        let (_tdb, conn_str) = setup_test_db().await;
        let conns = Conns::new();
        let id = conns.register(conn_str).await.unwrap();

        // Test create type
        let create_type = "CREATE TYPE user_role AS ENUM ('admin', 'user')";
        assert_eq!(
            conns.create_type(&id, create_type).await.unwrap(),
            "success"
        );

        // Test invalid type creation
        let invalid_type = "CREATE TABLE test (id INT)";
        assert!(conns.create_type(&id, invalid_type).await.is_err());
    }

    #[tokio::test]
    async fn create_schema_should_work() {
        let (_tdb, conn_str) = setup_test_db().await;
        let conns = Conns::new();
        let id = conns.register(conn_str).await.unwrap();

        // Test create schema with valid name
        let schema_name = "test_schema_unit";
        assert_eq!(
            conns.create_schema(&id, schema_name).await.unwrap(),
            "success"
        );

        // Verify schema exists using a query
        let query = format!(
            "SELECT schema_name FROM information_schema.schemata WHERE schema_name = '{}'",
            schema_name
        );
        let _result = sqlx::query(&query)
            .fetch_one(&conns.inner.load().get(&id).unwrap().pool)
            .await
            .unwrap();

        // Test create schema with invalid name
        let invalid_schema_name = "test;schema";
        assert!(conns.create_schema(&id, invalid_schema_name).await.is_err());
    }
}
