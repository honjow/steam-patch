use super::Device;
use crate::config::{get_global_config, self};
use crate::devices::device_generic::DeviceGeneric;
use crate::devices::Patch;
use crate::patch::PatchFile;
use crate::server::SettingsRequest;
use crate::steam::SteamClient;
use crate::{utils, main};
use std::fs::File;
use std::path::Path;
use std::{fs, env};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;
use std::io::{self, Write, Read};
use std::io::BufRead;
use std::collections::HashMap;


pub struct DeviceGo {
    device: DeviceGeneric,
}
#[derive(Debug)]
struct ByteData {
    index: usize,
    value: String,
}

impl DeviceGo {
    pub fn new(tdp: i8, gpu: i16) -> Self {
        DeviceGo {device: DeviceGeneric::new(tdp, 800,gpu)}
}
}

impl Device for DeviceGo {
    fn set_thermalpolicy(&self, thermalpolicy: i32){
        // The actual implementation would go here
        println!("Feature not implemented outside of ROG ALLY (Thermal policy): {}", thermalpolicy);
    }

    fn update_settings(&self, request: SettingsRequest) {
        if let Some(per_app) = &request.per_app {
            println!("{:#?}",per_app);
            // TDP changes
            if let Some(true) = per_app.is_tdp_limit_enabled {
                if let Some(tdp) = per_app.tdp_limit {
                    self.set_tdp(tdp);
                }
            } 
            if let Some(gpu) = per_app.gpu_performance_manual_mhz {
                self.set_gpu(gpu);
            }
        }
    }
    //Add more patches for device specific
    fn get_patches(&self) -> Vec<Patch> {
        let mut patches = self.device.get_patches();
        patches.push(Patch {
            text_to_find: String::from("this.m_rgControllers=new Map,\"undefined\"!=typeof SteamClient&&(this.m_hUnregisterControllerDigitalInput"),
            replacement_text: String::from("this.m_rgControllers=new Map; window.HandleSystemKeyEvents = this.HandleSystemKeyEvents; \"undefined\"!=typeof SteamClient&&(this.m_hUnregisterControllerDigitalInput"),
            destination: PatchFile::Library,
        });
        patches
    }

    fn set_tdp(&self, tdp: i8) {
        self.device.set_tdp(tdp);
    }

    fn set_gpu(&self, gpu: i16) {
        //Placeholder for later implementations
        println!("New GPU clock: {}", gpu);
    }

    fn get_key_mapper(&self) -> Option<tokio::task::JoinHandle<()>> {
        tokio::spawn(async move {
            let mut steam = SteamClient::new();
            steam.connect().await;
            start_mapper(steam);
        });
        None
    }
}

fn read_from_hidraw(device_path: &str, buffer_size: usize) -> io::Result<Vec<u8>> {
    let path = Path::new(device_path);
    let mut device = File::open(path)?;

    let mut buffer = vec![0u8; buffer_size];
    let bytes_read = device.read(&mut buffer)?;

    buffer.truncate(bytes_read);

    Ok(buffer)
}
// Function to find an active hidraw device
fn find_active_hidraw_device(device1: &str, device2: &str) -> io::Result<Option<String>> {
    let mut buffer = [0; 1024]; // Buffer to read data into

    for device_path in [device1, device2].iter() {
        if let Ok(mut file) = File::open(device_path) {
            if let Ok(size) = file.read(&mut buffer) {
                if size > 63 {
                    println!("Using {:?}", device_path);
                    return Ok(Some((*device_path).to_string()));
                }
            }
        }
    }
    
    Ok(None)
}

pub fn start_mapper(mut steam: SteamClient) -> Option<tokio::task::JoinHandle<()>> {
    let conf = get_global_config();
    let buffer_size = 1024;
    let mut previous_data = Vec::new(); // Variable to keep track of prev states
    println!("Steam mapper {}", conf.mapper);
    if conf.mapper {
        Some(tokio::spawn(async move {
            let active_device = match find_active_hidraw_device("/dev/hidraw3", "/dev/hidraw2") {
                Ok(Some(path)) => path,
                _ => {
                    eprintln!("No active HIDRAW device found, retrying in 2 seconds");
                    thread::sleep(Duration::from_secs(2));
                    tokio::spawn(async move {
                        start_mapper(steam)
                    });
                    return;
                }
            };

            loop {
                match read_from_hidraw(&active_device, buffer_size) {
                    Ok(data) => {
                        //Ensures that the data len is a whole packet of data
                        if previous_data != data && data.len() == 64{
                            // println!("Controller data: {:?}",data);
                            // println!("Data le {:?}", data.len());
                            if(data[18] == 64){
                                println!("Show QAM");
                                        steam
                                            .execute("GamepadNavTree.m_Controller.OnButtonActionInternal(true, 28, 2)")
                                            .await;
                            }
                            if(data[18] == 128){
                                println!("Show Menu");
                                        steam
                                            .execute("GamepadNavTree.m_Controller.OnButtonActionInternal(true, 27, 2); console.log(\"Show Menu\");")
                                            .await;
                            }
                            if(data[18] == 128 && data[19] == 32) {
                                println!("Show keyboard")
                            }
                        } else if data.len() < 64 {
                            println!("Device data length {:?}", data.len());
                            println!("Device path {:?}", &active_device);
                            // println!("Device data received: {:?}", data);
                        }
                            //                             //Update prev state
                        previous_data = data.clone();
                    },
                    Err(e) => {
                        eprintln!("Failed to read from device: {}", e);                       
                        print!("Error reading event stream, retrying in 3 second");
                        thread::sleep(Duration::from_secs(2));
                        tokio::spawn(async move {
                            start_mapper(steam)
                        });
                        break
                    },
                }
            }
        }))
    } else {
        println!("Mapper disabled");
        None
    }
}