use async_trait::async_trait;
use std::collections::HashMap;

/// 通用数据库错误类型
#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("Postgres error: {0}")]
    Postgres(#[from] tokio_postgres::Error),
    #[error("Connection error: {0}")]
    Connection(String),
    #[error("Other error: {0}")]
    Other(String),
}

pub type DbResult<T> = Result<T, DatabaseError>;

/// 通用数据库存储接口
#[async_trait]
pub trait DatabaseStore: Send + Sync {
    /// 连接数据库
    async fn connect(&mut self, uri: &str) -> DbResult<()>;
    /// 初始化数据库（如建表）
    async fn setup(&mut self) -> DbResult<()>;
    /// 执行SQL查询，返回结果（简单用Vec<HashMap<String, String>>表示）
    async fn execute_query(&self, sql: &str) -> DbResult<Vec<HashMap<String, String>>>;
    /// 关闭连接
    async fn close(&mut self) -> DbResult<()>;
}

/// Sqlite实现
pub struct SqliteStore {
    pub conn: Option<rusqlite::Connection>,
}

impl SqliteStore {
    pub fn new() -> Self {
        Self { conn: None }
    }
}

#[async_trait]
impl DatabaseStore for SqliteStore {
    async fn connect(&mut self, uri: &str) -> DbResult<()> {
        let conn = rusqlite::Connection::open(uri)?;
        self.conn = Some(conn);
        Ok(())
    }
    async fn setup(&mut self) -> DbResult<()> {
        if let Some(conn) = &self.conn {
            conn.execute(
                "CREATE TABLE IF NOT EXISTS test (id INTEGER PRIMARY KEY, value TEXT)",
                [],
            )?;
            Ok(())
        } else {
            Err(DatabaseError::Connection("Not connected".into()))
        }
    }
    async fn execute_query(&self, sql: &str) -> DbResult<Vec<HashMap<String, String>>> {
        let mut results = Vec::new();
        if let Some(conn) = &self.conn {
            let mut stmt = conn.prepare(sql)?;
            let cols = stmt.column_names().to_vec();
            let rows = stmt.query_map([], |row| {
                let mut map = HashMap::new();
                for (i, col) in cols.iter().enumerate() {
                    let val: Result<String, _> = row.get(i);
                    map.insert(col.to_string(), val.unwrap_or_default());
                }
                Ok(map)
            })?;
            for row in rows {
                results.push(row?);
            }
            Ok(results)
        } else {
            Err(DatabaseError::Connection("Not connected".into()))
        }
    }
    async fn close(&mut self) -> DbResult<()> {
        self.conn = None;
        Ok(())
    }
}

/// Postgres实现
pub struct PostgresStore {
    client: Option<tokio_postgres::Client>,
    connection_handle: Option<tokio::task::JoinHandle<()>>,
}

impl PostgresStore {
    /// 创建新的PostgresStore
    pub fn new() -> Self {
        Self {
            client: None,
            connection_handle: None,
        }
    }
}

#[async_trait]
impl DatabaseStore for PostgresStore {
    async fn connect(&mut self, uri: &str) -> DbResult<()> {
        let (client, connection) = tokio_postgres::connect(uri, tokio_postgres::NoTls).await?;
        // 驱动连接future
        let handle = tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("Postgres connection error: {e}");
            }
        });
        self.client = Some(client);
        self.connection_handle = Some(handle);
        Ok(())
    }
    async fn setup(&mut self) -> DbResult<()> {
        if let Some(client) = &self.client {
            client.execute(
                "CREATE TABLE IF NOT EXISTS test (id SERIAL PRIMARY KEY, value TEXT)",
                &[],
            ).await?;
            Ok(())
        } else {
            Err(DatabaseError::Connection("Not connected".into()))
        }
    }
    async fn execute_query(&self, sql: &str) -> DbResult<Vec<HashMap<String, String>>> {
        let mut results = Vec::new();
        if let Some(client) = &self.client {
            let rows = client.query(sql, &[]).await?;
            for row in rows {
                let mut map = HashMap::new();
                for (i, col) in row.columns().iter().enumerate() {
                    let val: Result<String, _> = row.try_get(i);
                    map.insert(col.name().to_string(), val.unwrap_or_default());
                }
                results.push(map);
            }
            Ok(results)
        } else {
            Err(DatabaseError::Connection("Not connected".into()))
        }
    }
    async fn close(&mut self) -> DbResult<()> {
        self.client = None;
        if let Some(handle) = self.connection_handle.take() {
            handle.abort();
        }
        Ok(())
    }
}

/// 单元测试（sqlite内存库）
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_sqlite_store() {
        let mut store = SqliteStore::new();
        store.connect(":memory:").await.unwrap();
        store.setup().await.unwrap();
        store.execute_query("INSERT INTO test (value) VALUES ('hello')").await.unwrap();
        let rows = store.execute_query("SELECT * FROM test").await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["value"], "hello");
        store.close().await.unwrap();
    }
} 