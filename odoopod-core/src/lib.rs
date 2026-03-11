pub mod instance;
mod components;
mod error;
use std::path::PathBuf;
use std::sync::Arc;
use components::ComponentsManager;

use crate::instance::{OdooInstance, Configured, Ready, Running, InstanceConfig};
use crate::components::postgres::{PostgresSettings, PostgresInstance};
use crate::error::OdooPodError;

pub struct OdooPod {
    root_path: PathBuf,
    /// Configurations persistées sur disque, rechargées au démarrage.
    known_configs: Vec<instance::InstanceConfig>,
    components_manager: ComponentsManager,
    _ready_instances: Vec<OdooInstance<Ready>>,
    running_instances: Vec<Arc<OdooInstance<Running>>>,
}

impl OdooPod {

    // To create an Odoo instance I need :
    // Python
    // Postgresql
    // PIP Dependencies
    // Odoo source code

    // I don't want to use docker, but the system's resources, so I need to install all the dependencies as prebuilt binaries, for each platform
    // Eeach Odoo has a specific version of Python, Postgresql and PIP dependencies, so I need to manage the dependencies for each Odoo version

    // To manage python versions and dependencies I can use uv, it is fast and easy to use with std::process::Command
    // It also allows me to create virtual environments for each Odoo instance, so I can manage the dependencies for each Odoo version separately

    // To manage postgresql versions, I can use PostgreSQL Embedded from thesus-rs : https://github.com/theseus-rs/postgresql-embedded
    // It allows me to create and manage PostgreSQL instances easily, and it also allows me to specify the version of PostgreSQL I want to use for each Odoo instance

    // Each Odoo instance will have its own configuration file, where I can specify the version of Python, Postgresql and PIP dependencies to use for that instance
    // I can also specify the Odoo source code to use for each instance, so I can have multiple Odoo instances running with different versions of Odoo
    // I can also specify the port to use for each Odoo instance, so I can access them easily from the browser
    
    // To allow quick edit, development en debugging of Odoo, I will create a folder for each instance with :
    // - A read-only link to the Odoo source code, so I cannot edit the source code but create modules quickly and easily, 
    //   it can also be a parameter of the box to isolate the source code for a specific instance to allow write access to the source code for that instance.
    // - A folder for custom modules, where I can create and edit my custom modules easily, and they will be loaded by Odoo automatically
    // - A folder for logs, where I can find the logs of the Odoo instance easily, and it will be rotated automatically to avoid filling up the disk space
    // - A folder for data, where I can find the data of the Odoo instance
    // - A folder for configuration, where I can find the configuration file of the Odoo instance, and I can edit it easily to change the configuration of the instance
    
    // I want to handle the lifecycle of the Odoo instances, so I can start, stop, restart and delete instances easily, and I can also check the status of the instances easily
    // I also want to handle the installation and uninstallation of python and postgresql indenpendently of the Odoo instances, 
    // so I can manage the dependencies for each Odoo instance separately, and I can also update the dependencies easily without affecting the Odoo instances

    // This project needs to be a library for the time being as it will be used in a GUI app AND a CLI app
    // , so I need to design the API of the library in a way that it can be used easily in both contexts,
    //  and I also need to design the API in a way that it can be extended easily in the future to add more features and functionalities to the library without breaking the existing API

    pub async fn new(root_path: Option<PathBuf>) -> Self {
        let app_dir = match root_path {
            Some(p) => p,
            None => {
                let p = dirs::config_dir()
                    .map(|p| p.join("odoopod"))
                    .unwrap_or_else(|| PathBuf::from("."));
                std::fs::create_dir_all(&p).unwrap();
                p
            }
        };
        let components_manager = ComponentsManager::new(
            app_dir.join("bin/tools"),
            app_dir.join("bin/postgres"),
        ).await;
        let known_configs = Self::load_known_configs(&app_dir);
        OdooPod { root_path: app_dir, components_manager, known_configs, _ready_instances: vec![], running_instances: vec![] }
    }

    fn load_known_configs(root_path: &PathBuf) -> Vec<instance::InstanceConfig> {
        let yaml_path = root_path.join("instances.yaml");
        if yaml_path.exists() {
            let content = std::fs::read_to_string(&yaml_path).unwrap_or_default();
            serde_yaml::from_str(&content).unwrap_or_default()
        } else {
            vec![]
        }
    }

    /// Liste les configurations connues (persistées sur disque).
    pub fn get_known_configs(&self) -> &[instance::InstanceConfig] {
        &self.known_configs
    }

    fn persist_configs(&self) {
        let yaml = serde_yaml::to_string(&self.known_configs).unwrap();
        // Prevent two instances of OdooPod with the same name
        let mut names = std::collections::HashSet::new();
        for config in &self.known_configs {
            if !names.insert(&config.name) {
                panic!("Duplicate instance name found: {}", config.name);
            }
        }
        std::fs::write(self.root_path.join("instances.yaml"), yaml).unwrap();
    }

    async fn ensure_postgres_server(&mut self, config: InstanceConfig) -> Result<Arc<PostgresInstance>, OdooPodError> {
        let settings = PostgresSettings {
            version: config.pg_version.clone(),
            username: "odoo".to_string(),
            port: self.available_postgres_ports().await.into_iter().next().ok_or_else(|| OdooPodError::NoAvailablePort("No available port for Postgres".to_string()))?,
            password: "odoo".to_string(),
            data_dir: self.root_path.join(format!("postgres_data/{}", config.pg_version.clone())),
        };
        let server = self.components_manager.postgres_mut().new_postgres_server(settings).await?;
        Ok(server)
    }

    pub fn available_odoo_ports(&self) -> Vec<u16> {
        let used_ports: Vec<u16> = self.running_instances.iter()
            .map(|instance| instance.config.http_port)
            .collect();
        (8000..9000).filter(|port| !used_ports.contains(port)).collect()
    }

    pub fn available_longpolling_ports(&self) -> Vec<u16> {
        let used_ports: Vec<u16> = self.running_instances.iter()
            .map(|instance| instance.config.longpolling_port)
            .collect();
        (9000..10000).filter(|port| !used_ports.contains(port)).collect()
    }

    pub async fn available_postgres_ports(&self) -> Vec<u16> {
        self.components_manager.postgres().available_ports().await
    }

    /// Crée et enregistre une nouvelle instance.
    /// Retourne un `OdooInstance<Configured>` prêt à être configuré via `.setup().await`.
    pub async fn create_instance(&mut self, config: InstanceConfig) -> Result<OdooInstance<Configured>, OdooPodError> {
        self.known_configs.push(config.clone());
        self.persist_configs();

        let postgres_server = self.ensure_postgres_server(config.clone()).await.unwrap();
        let odoo_path = self.root_path.join("sources").join(format!("odoo_{}", config.odoo_version.clone()));
        if !odoo_path.exists() {
            std::fs::create_dir_all(&odoo_path).unwrap();
        }
        Ok(instance::OdooInstance::new(config, self.components_manager.uv(), postgres_server, odoo_path))
    }

    pub async fn get_instance(&mut self, name: &str) -> Option<OdooInstance<Ready>> {
        let config = self.known_configs.iter()
            .find(|config| config.name == name)
            .cloned();

        if let Some(config) = config {
            let pg_server = &self.ensure_postgres_server(config.clone()).await.unwrap();
            let instance = OdooInstance::from_config(
                config.clone(),
                self.components_manager.uv(),
                pg_server.clone(),
                self.root_path.join("sources").join(format!("odoo_{}", config.odoo_version.clone())),
            );
            return Some(instance);
        }
        None
    }

    pub async fn stop_all_instances(&mut self) -> Result<(), OdooPodError> {
        let instances = self.running_instances.clone();
        for instance in instances {
            let instance = Arc::try_unwrap(instance).unwrap_or_else(|_| panic!("Failed to unwrap Arc"));
            instance.stop().await?;
        }
        Ok(())
    }

    pub async fn delete_instance(&mut self, name: &str) -> Result<(), OdooPodError> {
        // Stop the instance if it's running
        self.stop_all_instances().await?;
        // Remove the configuration
        self.known_configs.retain(|c| c.name != name);
        self.persist_configs();
        Ok(())
    }

    pub async fn drop_services(&mut self) -> Result<(), OdooPodError> {
        tracing::info!("Stopping all Postgres servers...");
        self.components_manager.postgres().stop_all_servers().await?;
        tracing::info!("Stopping all Odoo instances...");
        self.stop_all_instances().await?;
        Ok(())
    }
}