use arc_swap::ArcSwap;
use serde::{Deserialize, Serialize};
use sqlparser::ast::Statement;
use sqlx::postgres::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

#[allow(unused)]
#[derive(Error, Debug)]
pub enum PgMcpError {
    #[error("Connection not found for ID: {0}")]
    ConnectionNotFound(String),

    #[error("SQL validation failed for query '{query}': {kind}")]
    ValidationFailed {
        kind: ValidationErrorKind,
        query: String,
        details: String,
    },

    #[error("Database operation '{operation}' failed: {underlying}")]
    DatabaseError {
        operation: String,
        underlying: String,
    },

    #[error("Serialization failed: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Database connection failed: {0}")]
    ConnectionError(String),

    #[error("Internal error: {0}")]
    InternalError(String),
}

#[derive(Error, Debug)]
pub enum ValidationErrorKind {
    #[error("Invalid statement type, expected {expected}")]
    InvalidStatementType { expected: String },
    #[error("Failed to parse SQL")]
    ParseError,
}

impl From<sqlx::Error> for PgMcpError {
    fn from(e: sqlx::Error) -> Self {
        let msg = e.to_string();
        if let Some(db_err) = e.as_database_error() {
            PgMcpError::DatabaseError {
                operation: "unknown".to_string(),
                underlying: db_err.to_string(),
            }
        } else if msg.contains("error connecting") || msg.contains("timed out") {
            PgMcpError::ConnectionError(msg)
        } else {
            PgMcpError::DatabaseError {
                operation: "unknown".to_string(),
                underlying: msg,
            }
        }
    }
}

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

    pub(crate) async fn register(&self, conn_str: String) -> Result<String, PgMcpError> {
        let pool = PgPool::connect(&conn_str)
            .await
            .map_err(|e| PgMcpError::ConnectionError(e.to_string()))?;
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

    pub(crate) fn unregister(&self, id: String) -> Result<(), PgMcpError> {
        let mut conns = self.inner.load().as_ref().clone();
        if conns.remove(&id).is_none() {
            return Err(PgMcpError::ConnectionNotFound(id));
        }
        self.inner.store(Arc::new(conns));
        Ok(())
    }

    pub(crate) async fn query(&self, id: &str, query: &str) -> Result<String, PgMcpError> {
        let operation = "query (SELECT)";
        let conns = self.inner.load();
        let conn = conns
            .get(id)
            .ok_or_else(|| PgMcpError::ConnectionNotFound(id.to_string()))?;

        let validated_query =
            validate_sql(query, |stmt| matches!(stmt, Statement::Query(_)), "SELECT")?;

        let prepared_query = format!(
            "WITH data AS ({}) SELECT JSON_AGG(data.*) as ret FROM data;",
            validated_query
        );

        let ret = sqlx::query_as::<_, JsonRow>(&prepared_query)
            .fetch_one(&conn.pool)
            .await
            .map_err(|e| PgMcpError::DatabaseError {
                operation: operation.to_string(),
                underlying: e.to_string(),
            })?;

        Ok(serde_json::to_string(&ret.ret)?)
    }

    pub(crate) async fn insert(&self, id: &str, query: &str) -> Result<String, PgMcpError> {
        let operation = "insert (INSERT)";
        let conns = self.inner.load();
        let conn = conns
            .get(id)
            .ok_or_else(|| PgMcpError::ConnectionNotFound(id.to_string()))?;

        let validated_query = validate_sql(
            query,
            |stmt| matches!(stmt, Statement::Insert { .. }),
            "INSERT",
        )?;

        let result = sqlx::query(&validated_query)
            .execute(&conn.pool)
            .await
            .map_err(|e| PgMcpError::DatabaseError {
                operation: operation.to_string(),
                underlying: e.to_string(),
            })?;

        Ok(format!(
            "success, rows_affected: {}",
            result.rows_affected()
        ))
    }

    pub(crate) async fn update(&self, id: &str, query: &str) -> Result<String, PgMcpError> {
        let operation = "update (UPDATE)";
        let conns = self.inner.load();
        let conn = conns
            .get(id)
            .ok_or_else(|| PgMcpError::ConnectionNotFound(id.to_string()))?;

        let validated_query = validate_sql(
            query,
            |stmt| matches!(stmt, Statement::Update { .. }),
            "UPDATE",
        )?;

        let result = sqlx::query(&validated_query)
            .execute(&conn.pool)
            .await
            .map_err(|e| PgMcpError::DatabaseError {
                operation: operation.to_string(),
                underlying: e.to_string(),
            })?;

        Ok(format!(
            "success, rows_affected: {}",
            result.rows_affected()
        ))
    }

    pub(crate) async fn delete(&self, id: &str, query: &str) -> Result<String, PgMcpError> {
        let operation = "delete (DELETE)";
        let conns = self.inner.load();
        let conn = conns
            .get(id)
            .ok_or_else(|| PgMcpError::ConnectionNotFound(id.to_string()))?;

        let validated_query = validate_sql(
            query,
            |stmt| matches!(stmt, Statement::Delete { .. }),
            "DELETE",
        )?;

        let result = sqlx::query(&validated_query)
            .execute(&conn.pool)
            .await
            .map_err(|e| PgMcpError::DatabaseError {
                operation: operation.to_string(),
                underlying: e.to_string(),
            })?;

        Ok(format!(
            "success, rows_affected: {}",
            result.rows_affected()
        ))
    }

    pub(crate) async fn create_table(&self, id: &str, query: &str) -> Result<String, PgMcpError> {
        let operation = "create_table (CREATE TABLE)";
        let conns = self.inner.load();
        let conn = conns
            .get(id)
            .ok_or_else(|| PgMcpError::ConnectionNotFound(id.to_string()))?;

        let validated_query = validate_sql(
            query,
            |stmt| matches!(stmt, Statement::CreateTable { .. }),
            "CREATE TABLE",
        )?;

        sqlx::query(&validated_query)
            .execute(&conn.pool)
            .await
            .map_err(|e| PgMcpError::DatabaseError {
                operation: operation.to_string(),
                underlying: e.to_string(),
            })?;

        Ok("success".to_string())
    }

    pub(crate) async fn drop_table(&self, id: &str, table: &str) -> Result<String, PgMcpError> {
        let operation = format!("drop_table (DROP TABLE {})", table);
        let conns = self.inner.load();
        let conn = conns
            .get(id)
            .ok_or_else(|| PgMcpError::ConnectionNotFound(id.to_string()))?;

        let query = format!("DROP TABLE {}", table);
        sqlx::query(&query)
            .execute(&conn.pool)
            .await
            .map_err(|e| PgMcpError::DatabaseError {
                operation,
                underlying: e.to_string(),
            })?;

        Ok("success".to_string())
    }

    pub(crate) async fn create_index(&self, id: &str, query: &str) -> Result<String, PgMcpError> {
        let operation = "create_index (CREATE INDEX)";
        let conns = self.inner.load();
        let conn = conns
            .get(id)
            .ok_or_else(|| PgMcpError::ConnectionNotFound(id.to_string()))?;

        let validated_query = validate_sql(
            query,
            |stmt| matches!(stmt, Statement::CreateIndex { .. }),
            "CREATE INDEX",
        )?;

        sqlx::query(&validated_query)
            .execute(&conn.pool)
            .await
            .map_err(|e| PgMcpError::DatabaseError {
                operation: operation.to_string(),
                underlying: e.to_string(),
            })?;

        Ok("success".to_string())
    }

    pub(crate) async fn drop_index(&self, id: &str, index: &str) -> Result<String, PgMcpError> {
        let operation = format!("drop_index (DROP INDEX {})", index);
        let conns = self.inner.load();
        let conn = conns
            .get(id)
            .ok_or_else(|| PgMcpError::ConnectionNotFound(id.to_string()))?;

        let query = format!("DROP INDEX {}", index);
        sqlx::query(&query)
            .execute(&conn.pool)
            .await
            .map_err(|e| PgMcpError::DatabaseError {
                operation,
                underlying: e.to_string(),
            })?;

        Ok("success".to_string())
    }

    pub(crate) async fn describe(&self, id: &str, table: &str) -> Result<String, PgMcpError> {
        let operation = format!("describe (table: {})", table);
        let conns = self.inner.load();
        let conn = conns
            .get(id)
            .ok_or_else(|| PgMcpError::ConnectionNotFound(id.to_string()))?;

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
            .await
            .map_err(|e| PgMcpError::DatabaseError {
                operation: operation.to_string(),
                underlying: e.to_string(),
            })?;

        Ok(serde_json::to_string(&ret.ret)?)
    }

    pub(crate) async fn list_tables(&self, id: &str, schema: &str) -> Result<String, PgMcpError> {
        let operation = format!("list_tables (schema: {})", schema);
        let conns = self.inner.load();
        let conn = conns
            .get(id)
            .ok_or_else(|| PgMcpError::ConnectionNotFound(id.to_string()))?;

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
            .await
            .or_else(|e| {
                if let sqlx::Error::RowNotFound = e {
                    Ok(JsonRow {
                        ret: sqlx::types::Json(serde_json::json!([])),
                    })
                } else {
                    Err(PgMcpError::DatabaseError {
                        operation: operation.to_string(),
                        underlying: e.to_string(),
                    })
                }
            })?;

        Ok(serde_json::to_string(&ret.ret)?)
    }

    pub(crate) async fn create_schema(
        &self,
        id: &str,
        schema_name: &str,
    ) -> Result<String, PgMcpError> {
        let operation = format!("create_schema (CREATE SCHEMA {})", schema_name);
        let conns = self.inner.load();
        let conn = conns
            .get(id)
            .ok_or_else(|| PgMcpError::ConnectionNotFound(id.to_string()))?;

        let query = format!("CREATE SCHEMA {}", schema_name);
        sqlx::query(&query)
            .execute(&conn.pool)
            .await
            .map_err(|e| PgMcpError::DatabaseError {
                operation,
                underlying: e.to_string(),
            })?;

        Ok("success".to_string())
    }

    pub(crate) async fn create_type(&self, id: &str, query: &str) -> Result<String, PgMcpError> {
        let operation = "create_type (CREATE TYPE)";
        let conns = self.inner.load();
        let conn = conns
            .get(id)
            .ok_or_else(|| PgMcpError::ConnectionNotFound(id.to_string()))?;

        let validated_query = validate_sql(
            query,
            |stmt| matches!(stmt, Statement::CreateType { .. }),
            "CREATE TYPE",
        )?;

        sqlx::query(&validated_query)
            .execute(&conn.pool)
            .await
            .map_err(|e| PgMcpError::DatabaseError {
                operation: operation.to_string(),
                underlying: e.to_string(),
            })?;

        Ok("success".to_string())
    }
}

impl Default for Conns {
    fn default() -> Self {
        Self::new()
    }
}

fn validate_sql<F>(
    query: &str,
    validator: F,
    expected_type: &'static str,
) -> Result<String, PgMcpError>
where
    F: Fn(&Statement) -> bool,
{
    let dialect = sqlparser::dialect::PostgreSqlDialect {};
    let statements = sqlparser::parser::Parser::parse_sql(&dialect, query).map_err(|e| {
        PgMcpError::ValidationFailed {
            kind: ValidationErrorKind::ParseError,
            query: query.to_string(),
            details: e.to_string(),
        }
    })?;

    if statements.len() != 1 {
        return Err(PgMcpError::ValidationFailed {
            kind: ValidationErrorKind::InvalidStatementType {
                expected: expected_type.to_string(),
            },
            query: query.to_string(),
            details: format!(
                "Expected exactly one SQL statement, found {}",
                statements.len()
            ),
        });
    }

    let stmt = &statements[0];
    if !validator(stmt) {
        return Err(PgMcpError::ValidationFailed {
            kind: ValidationErrorKind::InvalidStatementType {
                expected: expected_type.to_string(),
            },
            query: query.to_string(),
            details: format!("Statement type validation failed. Received: {:?}", stmt),
        });
    }

    Ok(query.to_string())
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

        let id = conns.register(conn_str.clone()).await.unwrap();
        assert!(!id.is_empty());

        assert!(conns.unregister(id.clone()).is_ok());
        assert!(conns.unregister(id).is_err());
    }

    #[tokio::test]
    async fn list_tables_describe_should_work() {
        let (_tdb, conn_str) = setup_test_db().await;
        let conns = Conns::new();
        let id = conns.register(conn_str).await.unwrap();

        let tables = conns.list_tables(&id, "public").await.unwrap();
        assert!(tables.contains("test_table"));

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

        let create_table = "CREATE TABLE test_table2 (id SERIAL PRIMARY KEY, name TEXT)";
        assert_eq!(
            conns.create_table(&id, create_table).await.unwrap(),
            "success"
        );

        assert_eq!(
            conns.drop_table(&id, "test_table2").await.unwrap(),
            "success"
        );

        assert!(conns.drop_table(&id, "test_table2").await.is_err());
    }

    #[tokio::test]
    async fn query_insert_update_delete_should_work() {
        let (_tdb, conn_str) = setup_test_db().await;
        let conns = Conns::new();
        let id = conns.register(conn_str).await.unwrap();

        let query = "SELECT * FROM test_table ORDER BY id";
        let result = conns.query(&id, query).await.unwrap();
        assert!(result.contains("test1"));
        assert!(result.contains("test2"));
        assert!(result.contains("test3"));

        let insert = "INSERT INTO test_table (name) VALUES ('test4')";
        let result = conns.insert(&id, insert).await.unwrap();
        assert!(result.contains("rows_affected: 1"));

        let update = "UPDATE test_table SET name = 'updated' WHERE name = 'test1'";
        let result = conns.update(&id, update).await.unwrap();
        assert!(result.contains("rows_affected: 1"));

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

        let create_index = "CREATE INDEX idx_test_table_new ON test_table (name, created_at)";
        assert_eq!(
            conns.create_index(&id, create_index).await.unwrap(),
            "success"
        );

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

        let invalid_query = "INSERT INTO test_table VALUES (1)";
        assert!(conns.query(&id, invalid_query).await.is_err());

        let invalid_insert = "SELECT * FROM test_table";
        assert!(conns.insert(&id, invalid_insert).await.is_err());

        let invalid_update = "DELETE FROM test_table";
        assert!(conns.update(&id, invalid_update).await.is_err());

        let invalid_create = "CREATE INDEX idx_test ON test_table (id)";
        assert!(conns.create_table(&id, invalid_create).await.is_err());

        let invalid_index = "CREATE TABLE test (id INT)";
        assert!(conns.create_index(&id, invalid_index).await.is_err());
    }

    #[tokio::test]
    async fn create_type_should_work() {
        let (_tdb, conn_str) = setup_test_db().await;
        let conns = Conns::new();
        let id = conns.register(conn_str).await.unwrap();

        let create_type = "CREATE TYPE user_role AS ENUM ('admin', 'user')";
        assert_eq!(
            conns.create_type(&id, create_type).await.unwrap(),
            "success"
        );

        let invalid_type = "CREATE TABLE test (id INT)";
        assert!(conns.create_type(&id, invalid_type).await.is_err());
    }

    #[tokio::test]
    async fn create_schema_should_work() {
        let (_tdb, conn_str) = setup_test_db().await;
        let conns = Conns::new();
        let id = conns.register(conn_str).await.unwrap();

        let schema_name = "test_schema_unit";
        assert_eq!(
            conns.create_schema(&id, schema_name).await.unwrap(),
            "success"
        );

        let query = format!(
            "SELECT schema_name FROM information_schema.schemata WHERE schema_name = '{}'",
            schema_name
        );
        let _result = sqlx::query(&query)
            .fetch_one(&conns.inner.load().get(&id).unwrap().pool)
            .await
            .unwrap();

        let invalid_schema_name = "test;schema";
        assert!(conns.create_schema(&id, invalid_schema_name).await.is_err());
    }
}
