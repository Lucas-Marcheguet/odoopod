use thiserror::Error;

#[derive(Error, Debug)]
pub enum OdooPodError {
    #[error("UV is not installed: {0}")]
    UVNotInstalled(String),
    #[error("Failed to setup environment: {0}")]
    SetupFailed(String),
    #[error("Failed to create virtual environment: {0}")]
    CreateVenvError(String),
    #[error("Failed to install requirements: {0}")]
    InstallRequirementsError(String),
    #[error("Failed to install package: {0}")]
    InstallPackageError(String),
    #[error("Failed to start instance: {0}")]
    StartInstanceError(String),
    #[error("Failed to stop instance: {0}")]
    StopInstanceError(String),
    #[error("Failed to create PostgreSQL server: {0}")]
    CreatePostgresServerError(String),
    #[error("Failed to stop PostgreSQL server: {0}")]
    StopPostgresServerError(String),
    #[error("Failed to ensure PostgreSQL database: {0}")]
    EnsurePostgresDatabaseError(String),
    #[error("Instance is not in a valid state for this operation")]
    InstanceStatusError,
    #[error("Failed to check PostgreSQL database existence: {0}")]
    CheckDatabaseError(String),
    #[error("No available port for PostgreSQL: {0}")]
    NoAvailablePort(String),
}