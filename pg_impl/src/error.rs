use thiserror::Error;

/// Represents all possible errors that can occur in the database layer.
#[derive(Error, Debug)]
pub enum DbError {
    /// An error originating from the `sqlx` crate, typically related to query execution.
    #[error("Database query failed: {0}")]
    Sqlx(#[from] sqlx::Error),

    /// An error that occurred during database migration.
    #[error("Database migration failed: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),
}

/// A specialized `Result` type for database operations.
pub type Result<T> = std::result::Result<T, DbError>;
