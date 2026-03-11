use clap::{Parser, Subcommand};

use odoopod_core::instance::{self, InstanceConfig};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Turn debugging information on
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,

    #[command(subcommand)]
    command: Commands,
}

// Commandes CLI envisagées :
// odoopod create --name test --odoo-version 19.0 --python-version 3.12 --pg-version 16 --http-port 8069 --longpolling-port 8072 --path ./instances/test
// odoopod start -d (detach, optional)
// odoopod stop
// odoopod remove --name test
// odoopod list
// odoopod stop-all

#[derive(Subcommand, Debug)]
enum Commands {
    Create {
        name: String,
        odoo_version: String,
        python_version: String,
        pg_version: String,
        http_port: u16,
        longpolling_port: u16,
    },
    Start {
        name: String,
    },
    Stop {
        name: String,
    },
    Remove {
        name: String,
    },
    List,
    StopAll,
}
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
    let cli = Cli::parse();
    let mut odoo_pod = odoopod_core::OdooPod::new(None).await;

    match cli.debug {
        0 => println!("Debug mode is off"),
        1 => println!("Debug mode is kind of on"),
        2 => println!("Debug mode is on"),
        _ => println!("Don't be crazy"),
    }

    // You can check for the existence of subcommands, and if found use their
    // matches just as you would the top level cmd
    match &cli.command {
        Commands::Create { name, odoo_version, python_version, pg_version, http_port, longpolling_port } => {
            println!("Creating instance: {}", name);
            println!("Odoo version: {}", odoo_version);
            println!("Python version: {}", python_version);
            println!("PostgreSQL version: {}", pg_version);
            println!("HTTP port: {}", http_port);
            println!("Longpolling port: {}", longpolling_port);
            // Get current path
            let current_path = std::env::current_dir().unwrap();
            let config = InstanceConfig {
                name: name.clone(),
                odoo_version: odoo_version.clone(),
                python_version: python_version.clone(),
                pg_version: pg_version.clone(),
                http_port: *http_port,
                longpolling_port: *longpolling_port,
                path: current_path.join(name.clone()),
            };
            let odoo_instance = odoo_pod.create_instance(config).await.unwrap();
            let _ = odoo_instance.setup().await.unwrap();
        }
        Commands::Start { name } => {
            println!("Starting instance: {}", name);
            let instance = odoo_pod.get_instance(name).await.unwrap();
            let running_instance = instance.start().await.unwrap();
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    println!("\nArrêt demandé par l'utilisateur...");
                }
            }

            running_instance.stop().await.unwrap();
        }
        Commands::Stop { name } => {
            println!("Stopping instance: {}", name);
            // let instance = odoo_pod.get_instance(name).await?;
            // if let Some(instance) = instance {
            //     instance.stop().await.unwrap();
            // } else {
            //     println!("Instance {} not found", name);
            // }
        }
        Commands::Remove { name } => {
            println!("Removing instance: {}", name);
            // let instance = odoo_pod.get_instance(name).await?;
            // if let Some(instance) = instance {
            //     instance.remove().await.unwrap();
            // } else {
            //     println!("Instance {} not found", name);
            // }
        }
        Commands::List => {
            println!("Listing all instances...");
        }
        Commands::StopAll => {
            println!("Stopping all instances...");
        }
    }
}