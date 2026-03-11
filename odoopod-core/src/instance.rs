use std::marker::PhantomData;
use std::sync::Arc;
use std::fs::OpenOptions;
use std::process::Stdio;
use serde::{Serialize, Deserialize};
use crate::error::OdooPodError;
use crate::components::uv::UvInstance;
use crate::components::postgres::PostgresInstance;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceConfig {
    pub name: String,
    pub odoo_version: String,    // "17.0"
    pub community: bool,        // true for community, false for enterprise
    pub python_version: String,  // "3.10"
    pub pg_version: String,      // "15"
    pub http_port: u16,
    pub longpolling_port: u16,
    pub path: std::path::PathBuf,
}

pub struct Configured;
pub struct Ready;
pub struct Running;
pub trait InstanceState {}
impl InstanceState for Configured {}
impl InstanceState for Ready {}
impl InstanceState for Running {}

pub struct OdooInstance<S: InstanceState> {
    pub config: InstanceConfig,
    uv: Arc<UvInstance>,
    postgres: Arc<PostgresInstance>,
    child: Option<tokio::process::Child>,
    odoo_path: std::path::PathBuf,
    _state: PhantomData<S>,
}

impl OdooInstance<Configured> {
    pub fn new(config: InstanceConfig, uv: Arc<UvInstance>, postgres: Arc<PostgresInstance>, odoo_path: std::path::PathBuf) -> Self {
        OdooInstance { config, uv, postgres, child: None, odoo_path, _state: PhantomData }
    }

    async fn ensure_python(&self) -> Result<(), OdooPodError> {
        self.uv.install_python_version(&self.config.python_version).await
            .map_err(|e| OdooPodError::SetupFailed(e.to_string()))
    }

    async fn ensure_venv(&self) -> Result<(), OdooPodError> {
        if !self.config.path.join(".venv").exists() {
            self.uv.create_venv(&self.config.python_version, self.config.path.join(".venv")).await
                .map_err(|e| OdooPodError::CreateVenvError(e.to_string()))?;
        }
        Ok(())
    }

    fn check_odoo_installed(&self, odoo_path: &std::path::PathBuf) -> bool {
        odoo_path.join("odoo-bin").exists()
    }

    async fn ensure_source(&self) -> Result<(), OdooPodError> {
        // Check odoo version, download it if not in cache, and extract it to the cache folder
        if !self.odoo_path.exists() || !self.check_odoo_installed(&self.odoo_path) {
            std::fs::create_dir_all(self.odoo_path.clone()).unwrap();
            crate::components::odoo::Odoo::download_extract_source_code(
                self.config.odoo_version.clone(),
                !self.config.community,
                self.odoo_path.clone(),
            ).await
                .map_err(|e| OdooPodError::SetupFailed(e.to_string()))?;
        }
        Ok(())
    }

    async fn ensure_dependencies(&self) -> Result<(), OdooPodError> {
        self.uv.pip_install_requirements(
            self.config.path.join(".venv"),
            self.odoo_path.join("requirements.txt"),
        ).await
            .map_err(|e| OdooPodError::InstallRequirementsError(e.to_string()))
    }

    async fn ensure_postgres(&self) -> Result<(), OdooPodError> {
        // TODO - Check if the database for this instance exists, and if not, create it.
        // let db_name = self.get_db_name();
        // self.postgres.ensure_database(&db_name).await
        //     .map_err(|e| OdooPodError::SetupFailed(e.to_string()))?;
        Ok(())
    }

    fn ensure_config(&self) -> Result<(), OdooPodError> {
        let config_path = self.config.path.join("odoo.conf");
        if !config_path.exists() {
            let config_content = format!(
                "[options]\n\
                addons_path = {addons_path}\n\
                db_user = odoo\n\
                db_password = odoo\n\
                db_port = {db_port}\n\
                db_host = 127.0.0.1\n\
                http_port = {http_port}\n\
                longpolling_port = {longpolling_port}\n",
                addons_path = self.config.path.join("odoo/addons").to_string_lossy(),
                db_port = 5432,
                http_port = self.config.http_port,
                longpolling_port = self.config.longpolling_port,
            );
            std::fs::write(config_path, config_content)
                .map_err(|e| OdooPodError::SetupFailed(e.to_string()))?;
        }
        Ok(())
    }

    fn ensure_symlink(&self) -> Result<(), OdooPodError> {
        let symlink_path = self.config.path.join("odoo");
        if !symlink_path.exists() {
            std::os::unix::fs::symlink(&self.odoo_path, &symlink_path)
                .map_err(|e| OdooPodError::SetupFailed(e.to_string()))?;
        }
        Ok(())
    }

    /// Configure l'instance et la fait passer à l'état Ready.
    pub async fn setup(self) -> Result<OdooInstance<Ready>, OdooPodError> {
        self.ensure_python().await?;
        self.ensure_venv().await?;
        self.ensure_source().await?;
        self.ensure_dependencies().await?;
        self.ensure_postgres().await?;
        self.ensure_config()?;
        self.ensure_symlink()?;
        Ok(OdooInstance { config: self.config, uv: self.uv, postgres: self.postgres, child: None, _state: PhantomData, odoo_path: self.odoo_path })
    }
}

impl OdooInstance<Ready> {
    /// Reconstruit une instance existante depuis la persistance disque.
    pub fn from_config(config: InstanceConfig, uv: Arc<UvInstance>, postgres: Arc<PostgresInstance>, odoo_path: std::path::PathBuf) -> Self {
        OdooInstance { config, uv, postgres, child: None, _state: PhantomData, odoo_path }
    }

    pub fn name(&self) -> &str { &self.config.name }
    pub fn path(&self) -> &std::path::PathBuf { &self.config.path }

    /// Lance l'instance Odoo et passe à l'état Running.
    /// stdout et stderr d'odoo-bin sont redirigés vers `<instance>/odoo.log`.
    pub async fn start(self) -> Result<OdooInstance<Running>, OdooPodError> {
        let log_path = self.config.path.join("odoo.log");
        let log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .map_err(|e| OdooPodError::StartInstanceError(
                format!("Cannot open log file {}: {}", log_path.display(), e)
            ))?;
        // On clone le descripteur pour pouvoir le passer à stderr séparément.
        let log_file_stderr = log_file.try_clone()
            .map_err(|e| OdooPodError::StartInstanceError(e.to_string()))?;

        let child = tokio::process::Command::new(self.config.path.join(".venv/bin/python"))
            .arg(self.odoo_path.join("odoo-bin"))
            .arg("-c").arg(self.config.path.join("odoo.conf"))
            .stdout(Stdio::from(log_file))
            .stderr(Stdio::from(log_file_stderr))
            .spawn()
            .map_err(|e| OdooPodError::StartInstanceError(e.to_string()))?;

        println!("Instance '{}' started — logs: {}", self.config.name, log_path.display());
        println!("Odoo will be available at http://localhost:{} after it finishes booting.", self.config.http_port);
        Ok(OdooInstance { config: self.config, uv: self.uv, postgres: self.postgres, child: Some(child), _state: PhantomData, odoo_path: self.odoo_path })
    }

    pub async fn stop(self) -> Result<OdooInstance<Ready>, OdooPodError> {
        if let Some(mut child) = self.child {
            child.kill().await
                .map_err(|e| OdooPodError::StopInstanceError(e.to_string()))?;
            child.wait().await
                .map_err(|e| OdooPodError::StopInstanceError(e.to_string()))?;
        }
        tracing::info!("Instance '{}' stopped.", self.config.name);
        Ok(OdooInstance { config: self.config, uv: self.uv, postgres: self.postgres, child: None, _state: PhantomData, odoo_path: self.odoo_path })
    }

}

impl OdooInstance<Running> {
    /// Reconnecte une instance déjà lancée (après redémarrage de l'appli) via son PID.
    pub fn attach(config: InstanceConfig, uv: Arc<UvInstance>, postgres: Arc<PostgresInstance>, odoo_path: std::path::PathBuf) -> Self {
        OdooInstance { config, uv, postgres, child: None, _state: PhantomData, odoo_path }
    }

    pub fn name(&self) -> &str { &self.config.name }

    /// Arrête l'instance et revient à l'état Ready.
    pub async fn stop(mut self) -> Result<OdooInstance<Ready>, OdooPodError> {
        if let Some(ref mut child) = self.child {
            child.kill().await
                .map_err(|e| OdooPodError::StopInstanceError(e.to_string()))?;
            child.wait().await
                .map_err(|e| OdooPodError::StopInstanceError(e.to_string()))?;
        }
        tracing::info!("Instance '{}' stopped.", self.config.name);
        Ok(OdooInstance { config: self.config, uv: self.uv, postgres: self.postgres, child: None, _state: PhantomData, odoo_path: self.odoo_path })
    }
}