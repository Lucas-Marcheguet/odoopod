use clap::Parser;

use odoopod_core::instance::InstanceConfig;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the configuration file
    #[arg(short, long, default_value = "config.toml")]
    config: String,
}

// Commandes CLI envisagées :
// odoopod create --name test --odoo-version 19.0 --python-version 3.12 --pg-version 16 --http-port 8069 --longpolling-port 8072 --path ./instances/test
// odoopod start -d (detach, optional)
// odoopod stop
// odoopod remove --name test
// odoopod list
// odoopod stop-all

// Commandes CLI potentielles
// odoopod config 
    // - set default versions for odoo, python and postgres (instance only)
    // - set default ports (instance only)
    // - set path for instances and sources (general)
    // - set uv settings (cache path, etc.) (general)
    // - set postgres settings (data path, etc.) (general)
// odoopod logs --name test
// odoopod shell --name test
// odoopod db-shell --name test
// odoopod update --name test --odoo-version 19.0

#[tokio::main]
async fn main() {
    let _args = Args::parse();
    let mut odoo_box = odoopod_core::odoopod::new(None).await;

    odoo_box.delete_instance("new_instance").await.unwrap();

    let config = InstanceConfig {
        name: "new_instance".to_string(),
        odoo_version: "19.0".to_string(),
        python_version: "3.12".to_string(),
        pg_version: "16".to_string(),
        http_port: 8069,
        longpolling_port: 8072,
        path: std::path::PathBuf::from("./instances/test"),
    };
    let odoo_instance = odoo_box.create_instance(config).await.unwrap();
    let ready_instance = odoo_instance.setup().await.unwrap();
    let running_instance = ready_instance.start().await.unwrap();

    // Attendre le signal Ctrl+C
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            println!("\nArrêt demandé par l'utilisateur...");
        }
    }

    running_instance.stop().await.unwrap();
    
    odoo_box.drop_services().await.unwrap(); 

    println!("✅ Tous les services ont été arrêtés proprement.");
}