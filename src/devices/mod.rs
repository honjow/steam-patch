pub mod device_ally;
pub mod device_generic;
pub mod device_go;


use crate::{patch::Patch, server::SettingsRequest};
use device_ally::DeviceAlly;
use device_go::DeviceGo;
use device_generic::DeviceGeneric;
use regex::Regex;
use std::fs;

use super::config::{self, Config, get_global_config};

pub trait Device {
    fn update_settings(&self, request: SettingsRequest);
    fn set_thermalpolicy(&self, thermal_policy: i32);
    fn set_tdp(&self, tdp: i8);
    fn set_gpu(&self, gpu: i16);
    fn get_patches(&self) -> Vec<Patch>;
    fn get_key_mapper(&self) -> Option<tokio::task::JoinHandle<()>>;
}

pub fn create_device() -> Option<Box<dyn Device>> {
        let conf = get_global_config();
        println!("Conf files loaded: {} {} {} {}", conf.gpu_control, conf.main_enabled, conf.max_tdp, conf.max_gpu);
        match get_device_name() {
        Some(device_name) => {
            match device_name.trim() {
                // Asus Rog Ally
                "AMD Ryzen Z1 Extreme ASUSTeK COMPUTER INC. RC71L" => {
                    Some(Box::new(DeviceAlly::new(conf.max_tdp, conf.max_gpu)))
                }

                // Ayaneo Geek
                "AMD Ryzen 7 6800U with Radeon Graphics AYANEO GEEK" => {
                    Some(Box::new(DeviceGeneric::new(28, 200, conf.max_gpu)))
                }

                // Ayaneo 2S
                "AMD Ryzen 7 7840U w/ Radeon 780M Graphics AYANEO AYANEO 2S" => {
                    Some(Box::new(DeviceGeneric::new(30, 800, conf.max_gpu)))
                }

                // Ayaneo Geek 1S
                "AMD Ryzen 7 7840U w/ Radeon 780M Graphics AYANEO GEEK 1S" => {
                    Some(Box::new(DeviceGeneric::new(30, 800, conf.max_gpu)))
                }

                // GPD WM2
                "AMD Ryzen 7 6800U with Radeon Graphics GPD G1619-04" => {
                    Some(Box::new(DeviceGeneric::new(28, 200, conf.max_gpu)))
                }

                // AOKZOE A1
                "AMD Ryzen 7 6800U with Radeon Graphics AOKZOE AOKZOE A1 AR07" => {
                    Some(Box::new(DeviceGeneric::new(28, 200, conf.max_gpu)))
                }
                "AMD Ryzen Z1 Extreme LENOVO LNVNB161216" => {
                    Some(Box::new(DeviceGo::new(conf.max_tdp, conf.max_gpu)))
                }

                s if s.contains("5560U") => {
                    Some(Box::new(DeviceGeneric::new(18, 200, conf.max_gpu)))
                }

                s if s.contains("6800U") => {
                    Some(Box::new(DeviceGeneric::new(30, 200, conf.max_gpu)))
                }

                s if s.contains("7840U") => {
                    Some(Box::new(DeviceGeneric::new(30, 200, conf.max_gpu)))
                }
                // Any other device
                _ => Some(Box::new(DeviceGeneric::new(conf.max_tdp,800, conf.max_gpu))),
            }
        }
        None => None,
    }
    
}

fn get_device_name() -> Option<String> {
    let cpuinfo = fs::read_to_string("/proc/cpuinfo").expect("Unknown");

    let model_re = Regex::new(r"model name\s*:\s*(.*)").unwrap();
    let model = model_re.captures_iter(&cpuinfo).next().unwrap()[1]
        .trim()
        .to_string();

    let board_vendor = match fs::read_to_string("/sys/devices/virtual/dmi/id/board_vendor") {
        Ok(str) => str.trim().to_string(),
        Err(_) => return None,
    };

    let board_name = match fs::read_to_string("/sys/devices/virtual/dmi/id/board_name") {
        Ok(str) => str.trim().to_string(),
        Err(_) => return None,
    };
    
    Some(format!("{} {} {}", model, board_vendor, board_name))
}
