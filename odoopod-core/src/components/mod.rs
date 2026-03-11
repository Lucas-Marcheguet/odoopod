pub mod uv;
pub mod postgres;
pub mod odoo;

use std::sync::Arc;

pub struct ComponentsManager {
    uv: Arc<uv::UvInstance>,
    postgres: postgres::PostgresManager,
}

impl ComponentsManager {
    pub async fn new(uv_path: std::path::PathBuf, postgres_path: std::path::PathBuf) -> Self {
        let uv_version = uv::UvInstaller::get_current_uv_version(uv_path.clone());
        if uv_version.is_err() {
            uv::UvInstaller::install_uv(uv_path.clone()).await;
        }
        ComponentsManager {
            uv: Arc::new(uv::UvInstance::new(uv_path)),
            postgres: postgres::PostgresManager::new(postgres_path),
        }
    }

    /// Clone le pointeur Arc vers uv (ne copie pas la valeur).
    pub fn uv(&self) -> Arc<uv::UvInstance> {
        Arc::clone(&self.uv)
    }

    /// Retourne une référence partagée vers le PostgresManager.
    pub fn postgres(&self) -> &postgres::PostgresManager {
        &self.postgres
    }

    /// Retourne une référence mutable vers le PostgresManager.
    pub fn postgres_mut(&mut self) -> &mut postgres::PostgresManager {
        &mut self.postgres
    }
}
