use super::Device;
use crate::config::{get_global_config, self};
use crate::devices::device_generic::DeviceGeneric;
use crate::devices::Patch;
use crate::patch::PatchFile;
use crate::server::SettingsRequest;
use crate::steam::SteamClient;
use crate::{utils, main};
use std::fs::File;
use std::{fs, env};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;
use std::io::{self, Write};
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

pub fn pick_device() -> Option<evdev::Device> {
    let target_vendor_id = 0x17efu16; //Device address hex
    let target_product_id = 0x6183u16; //device product in hex

    let devices = evdev::enumerate();
    for (_, device) in devices {
        let input_id = device.input_id();
        println!("INPUT: {:?}", input_id);
        
        if input_id.vendor() == target_vendor_id && input_id.product() == target_product_id {
            // if device.supported_keys().map_or(false, |keys: &evdev::AttributeSetRef<evdev::Key>| keys.contains(evdev::Key::BTN_SIDE)) {
                return Some(device);   
            // }
        }
    }
    None
}

fn process_block_data(block: &str) -> Vec<ByteData> {
    let mut data_array = Vec::new();
    let mut index = 0;
    // println!("One full block");
    for line in block.lines() {
        for byte in line.split_whitespace() {
            data_array.push(ByteData {
                index,
                value: byte.to_string(),
            });
            index += 1;
        }
    }

    data_array
}
// CREATE UDEV RULE TO ACCESS HID DEVICE OUTSIDE ROOT
async fn run_usbhid_dump(vendor_id: &str, product_id: &str, steam: &mut SteamClient) {
    println!("Running usbhid-dump for vendor_id: {}, product_id: {}", vendor_id, product_id);
    
    let cmd = "usbhid-dump";
    let args = format!("-m {}:{} -e all -i 2", vendor_id, product_id);
    let args_vec: Vec<&str> = args.split_whitespace().collect();
    println!("Executing command: {}", cmd);

    let mut process = Command::new(cmd)
        .args(&args_vec)
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to start usbhid-dump process");
    
    let mut previous_values: HashMap<usize, String> = HashMap::new();
    let mut block_data = String::new();

    println!("Process started, reading output...");
    if let Some(stdout) = process.stdout.take() {
        let reader = io::BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(ln) => {
                    if !ln.starts_with("001:") {
                        // println!("Processing line: {}", ln);

                        block_data.push_str(&ln);
                        block_data.push('\n');

                        if ln.is_empty() {
                            let data_array = process_block_data(&block_data);
                            for data in &data_array {
                                match previous_values.get(&data.index) {
                                    Some(prev_value) if prev_value == &data.value => {
                                        //Value hasn't changed, do nothing or handle
                                    }, 
                                    _ => {
                                        //Value has changed or is new, process accordingly
                                        // println!("Changed: Index: {}, Value: {}", data.index, data.value);

                                        // Example: handling Steam button
                                        if data.index == 18 && data.value == "80" {
                                            println!("Steam button");
                                            steam.execute("GamepadNavTree.m_Controller.OnButtonActionInternal(true, 27, 2); console.log(\"Show Menu\");").await;
                                        }
                                        if data.index == 18 && data.value == "40" {
                                            println!("QAM button");
                                            steam.execute("GamepadNavTree.m_Controller.OnButtonActionInternal(true, 28, 2)").await; 
                                        }

                                         // Update the previous value
                                         previous_values.insert(data.index, data.value.clone());
                                    }
                                }
                            }
                            // values aren't filtered here.
                            
                            block_data.clear();
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error reading line: {}", e);
                    break;
                }
            }
        }
    }

    match process.wait() {
        Ok(status) => println!("Process finished with status: {}", status),
        Err(e) => eprintln!("Error waiting for process: {}", e),
    }
}

pub fn start_mapper(mut steam:SteamClient) -> Option<tokio::task::JoinHandle<()>> {
    let conf = get_global_config();
    let vendor_id: &str = "17ef";
    let product_ids = vec!["6182", "6183"];
    println!("Steam mapper {}", conf.mapper);
    Some(tokio::spawn(async move {
        if conf.mapper {
            loop {
                for product_id in &product_ids {
                    println!("Trying product ID {}", product_id);
                    run_usbhid_dump(vendor_id, product_id, &mut steam).await;
                    //Sleep between attempts
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }))
    
}
