pub mod schema;
pub mod password;
pub mod users;
pub mod devices;
pub mod sessions;

use anyhow::Result;
use limbo::{Database as LimboDatabase, params};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct Database {
    db: Arc<RwLock<LimboDatabase>>,
}

impl Database {
    pub async fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db = if path.as_ref().to_str() == Some(":memory:") {
            LimboDatabase::open_in_memory()?
        } else {
            LimboDatabase::open(path)?
        };

        let database = Database {
            db: Arc::new(RwLock::new(db)),
        };

        // Initialize schema
        database.init_schema().await?;

        Ok(database)
    }

    pub async fn init_schema(&self) -> Result<()> {
        let db = self.db.write().await;
        schema::create_tables(&*db)?;
        Ok(())
    }

    pub async fn execute(&self, sql: &str, params: impl limbo::params::Params) -> Result<()> {
        let db = self.db.write().await;
        let mut stmt = db.prepare(sql)?;
        stmt.execute(params)?;
        Ok(())
    }

    pub async fn query_row<T, F>(&self, sql: &str, params: impl limbo::params::Params, f: F) -> Result<Option<T>>
    where
        F: FnOnce(&limbo::Row) -> Result<T>,
    {
        let db = self.db.read().await;
        let mut stmt = db.prepare(sql)?;
        let mut rows = stmt.query(params)?;
        
        if let Some(row) = rows.next()? {
            Ok(Some(f(&row)?))
        } else {
            Ok(None)
        }
    }

    pub async fn query_rows<T, F>(&self, sql: &str, params: impl limbo::params::Params, f: F) -> Result<Vec<T>>
    where
        F: Fn(&limbo::Row) -> Result<T>,
    {
        let db = self.db.read().await;
        let mut stmt = db.prepare(sql)?;
        let mut rows = stmt.query(params)?;
        
        let mut results = Vec::new();
        while let Some(row) = rows.next()? {
            results.push(f(&row)?);
        }
        Ok(results)
    }

    pub async fn transaction<T, F>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&LimboDatabase) -> Result<T>,
    {
        let db = self.db.write().await;
        db.execute("BEGIN TRANSACTION", params!())?;
        
        match f(&*db) {
            Ok(result) => {
                db.execute("COMMIT", params!())?;
                Ok(result)
            }
            Err(e) => {
                let _ = db.execute("ROLLBACK", params!());
                Err(e)
            }
        }
    }
}

// Re-export types
pub use users::{User, UserRole, CreateUserRequest};
pub use devices::{Device, DeviceInfo, TopicInfo, PingRequest};
pub use sessions::Session;
pub use password::PasswordManager;