use std::{collections::HashMap, hash::Hash, sync::Arc};
use pg_embed::postgres::{PgSettings, PgEmbed};
use pg_embed::pg_fetch::{PgFetchSettings, PG_V14, PG_V15, PG_V16, PG_V17, PostgresVersion};
use pg_embed::pg_enums::PgAuthMethod;
use std::time::Duration;
use std::path::PathBuf;
use crate::error::OdooPodError;

pub struct PostgresSettings {
    pub version: String,
    pub host: String,
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
    server: PgEmbed,
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
        PostgresInstance { version, server: postgres }
    }

    pub fn server_mut(&mut self) -> &mut PgEmbed {
        &mut self.server
    }

    pub async fn ensure_database(&self, db_name: &str) -> Result<(), OdooPodError> {
        match self.server.database_exists(db_name).await {
            Ok(exists) => {
                if !exists {
                    self.add_database(db_name).await?;
                }
            }
            Err(e) => return Err(OdooPodError::CheckDatabaseError(e.to_string())),
        };
        Ok(())
    }

    pub async fn add_database(&self, db_name: &str) -> Result<(), OdooPodError> {
       self.server.create_database(db_name).await;
        Ok(())
    }

    pub async fn remove_database(&self, db_name: &str) -> Result<(), OdooPodError> {
        self.server.drop_database(db_name).await;
        Ok(())
    }

    pub async fn start(&mut self) -> Result<(), OdooPodError> {
        self.server.start_db().await;
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<(), OdooPodError> {
        self.server.stop_db().await;
        Ok(())
    }
}

impl PostgresManager {
    pub fn new(_postgres_path: std::path::PathBuf) -> Self {
        PostgresManager { postgres_servers: Vec::new() }
    }

    pub async fn new_postgres_server(&mut self, settings: PostgresSettings) -> Result<Arc<PostgresInstance>, OdooPodError> {
        // let pg_settings = SettingsBuilder::new()
        //     .version(VersionReq::parse(&settings.version).unwrap())
        //     .host(settings.host)
        //     .port(settings.port)
        //     .username(settings.username)
        //     .password(settings.password)
        //     .data_dir(settings.data_dir)
        //     .temporary(false)
        //     .configuration(pg_config)
        //     .build();
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
            let mut server = Arc::clone(server);
            server.stop().await.map_err(|e| OdooPodError::StopPostgresServerError(e.to_string()))?;
        }
        Ok(())
    }

    pub fn get_postgres_server(&self, version: &str) -> Option<Arc<PostgresInstance>> {
        self.postgres_servers.iter().find(|server| server.version == version).map(Arc::clone)
    }
}