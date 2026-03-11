use std::{sync::Arc};
use pg_embed::postgres::{PgSettings, PgEmbed};
use pg_embed::pg_fetch::{PgFetchSettings, PG_V14, PG_V15, PG_V16, PG_V17, PostgresVersion};
use pg_embed::pg_enums::PgAuthMethod;
use std::time::Duration;
use tokio::sync::Mutex;
use crate::error::OdooPodError;

pub struct PostgresSettings {
    pub version: String,
    pub username: String,
    pub password: String,
    pub data_dir: std::path::PathBuf,
    pub port: u16,
}

pub struct PostgresManager {
    postgres_servers: Vec<Arc<PostgresInstance>>,
}

pub struct PostgresInstance {
    version: String,
    server: Mutex<PgEmbed>,
}

fn get_pg_version(version: &str) -> PostgresVersion {
    match version {
        "14" => PG_V14,
        "15" => PG_V15,
        "16" => PG_V16,
        "17" => PG_V17,
        _ => panic!("Unsupported PostgreSQL version: {}", version),
    }
}

impl PostgresInstance {
    pub async fn new(settings: PgSettings, version: String) -> Self {
        let fetch_settings = PgFetchSettings {
            version: get_pg_version(&version),
            ..Default::default()
        };
        let mut postgres = match PgEmbed::new(settings, fetch_settings).await {
            Ok(pg) => {
                tracing::info!("PostgreSQL {} setup completed", version);
                pg
            },
            Err(e) => {
                tracing::error!("Failed to setup PostgreSQL {}: {}", version, e);
                panic!("Failed to initialize PostgreSQL: {}", e);
            },
        };
        postgres.setup().await;
        postgres.start_db().await;
        PostgresInstance { version, server: Mutex::new(postgres) }
    }

    pub async fn ensure_database(&self, db_name: &str) -> Result<(), OdooPodError> {
        let server = self.server.lock().await;
        match server.database_exists(db_name).await {
            Ok(exists) => {
                if !exists {
                    drop(server);
                    self.add_database(db_name).await?;
                }
            }
            Err(e) => return Err(OdooPodError::CheckDatabaseError(e.to_string())),
        };
        Ok(())
    }

    pub async fn add_database(&self, db_name: &str) -> Result<(), OdooPodError> {
        let server = self.server.lock().await;
        server.create_database(db_name).await;
        Ok(())
    }

    pub async fn remove_database(&self, db_name: &str) -> Result<(), OdooPodError> {
        let server = self.server.lock().await;
        server.drop_database(db_name).await;
        Ok(())
    }

    pub async fn start(&self) -> Result<(), OdooPodError> {
        let mut server = self.server.lock().await;
        server.start_db().await;
        Ok(())
    }

    pub async fn stop(&self) -> Result<(), OdooPodError> {
        let mut server = self.server.lock().await;
        server.stop_db().await;
        Ok(())
    }
}

impl PostgresManager {
    pub fn new(_postgres_path: std::path::PathBuf) -> Self {
        PostgresManager { postgres_servers: Vec::new() }
    }

    pub async fn available_ports(&self) -> Vec<u16> {
        let mut used_ports = Vec::new();
        for server in &self.postgres_servers {
            let port = server.server.lock().await.pg_settings.port;
            used_ports.push(port);
        }
        (5432..6000).filter(|port| !used_ports.contains(port)).collect()
    }

    pub async fn new_postgres_server(&mut self, settings: PostgresSettings) -> Result<Arc<PostgresInstance>, OdooPodError> {
        let pg_settings = PgSettings {
            database_dir: settings.data_dir.clone(),
            port: settings.port,
            user: settings.username.clone(),
            password: settings.password.clone(),
            auth_method: PgAuthMethod::Plain,
            persistent: false,
            timeout: Some(Duration::from_secs(15)),
            migration_dir: None,
        };
        let postgres = Arc::new(PostgresInstance::new(pg_settings, settings.version.clone()).await);
        self.postgres_servers.push(Arc::clone(&postgres));
        Ok(postgres)
    }

    pub async fn stop_all_servers(&self) -> Result<(), OdooPodError> {
        for server in &self.postgres_servers {
            server.stop().await.map_err(|e| OdooPodError::StopPostgresServerError(e.to_string()))?;
        }
        Ok(())
    }
}