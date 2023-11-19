use config::get_global_config;

use crate::devices::create_device;

#[macro_use]
extern crate lazy_static;

mod devices;
mod patch;
mod server;
mod steam;
mod utils;
mod config;

#[tokio::main]
async fn main() {
    // Initialize the config by reading it and storing it in the global CONFIG
    let _ = config::initialize_config();


    let config = get_global_config();
    if config.main_enabled == true {
        let mut tasks = vec![];
        tasks.push(tokio::spawn(server::run()));

        if let Some(device) = create_device() {
            if let Some(mapper) = device.get_key_mapper() {
                tasks.push(mapper);
            }
        }

        if let Some(steam) = steam::SteamClient::watch().await {
            tasks.push(steam);
        }

        let _ = futures::future::join_all(tasks).await;
    } else {
        println!("Steam patch disabled in config.")
    }
    println!("Configuration is not available.")

    
}
