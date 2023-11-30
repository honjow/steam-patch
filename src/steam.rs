#![allow(non_snake_case)] // Allow non-snake_case identifiers

use crate::devices::create_device;
use crate::patch::Patch;
use crate::utils::get_username;
use dirs::home_dir;
use hyper::{Client, Uri, body};
use inotify::{Inotify, WatchMask};
use serde::{Deserialize};
use tungstenite::http::response;
use std::collections::HashMap;
use std::env;
use std::f32::consts::E;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Error};
use std::os::unix::process;
use std::path::PathBuf;
use sysinfo::{ProcessExt, SystemExt};
use tokio::time::{sleep, Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tungstenite::connect;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Message, WebSocket};
use std::option::Option;
use std::process::{Command, Stdio};
use std::thread;
use reqwest;
use serde_json::Value;


#[derive(Deserialize)]
struct Tab {
    title: String,
    webSocketDebuggerUrl: String,
}

pub struct SteamClient {
    socket: Option<WebSocket<MaybeTlsStream<std::net::TcpStream>>>,
}

impl SteamClient {
    pub fn patch(&mut self, patches: Vec<Patch>) -> Result<(), Error> {
        let mut opened_files: HashMap<String, String> = HashMap::new();

        for patch in &patches {
            // Use `match` or `if let` to handle the Option returned by `get_file()`
            // println!("Applying patch: Curr->{:?}", patch.text_to_find);
            if let Ok(Some(path_file)) = patch.destination.get_file() {
                // Convert the `Path` to a string and handle the potential `None` case
                if let Some(path_str) = path_file.to_str() {
                    let path_string = path_str.to_string();
                    // Handle the Result of `read_to_string` with `unwrap_or_else`
                    let content = opened_files
                        .entry(path_string.clone())
                        .or_insert_with(|| {
                            match fs::read_to_string(&path_file) {
                                Ok(content) => {
                                    // println!("File read successfully: {}", path_string);
                                    content
                                }
                                Err(e) => {
                                    // Handle the error, e.g., by logging and continuing with an empty string
                                    eprintln!("Error reading the file '{}': {}", path_string, e);
                                    String::new() // return an empty string on error
                                }
                            }
                        });
                    let text_to_find = &patch.text_to_find;
                    let replacement_text = &patch.replacement_text;
                    if content.contains(text_to_find) {
                        // println!("Success: New->{}", replacement_text);
                        *content = content.replace(text_to_find, replacement_text);
                    } else {
                        println!("Failed to patch: '{}'", replacement_text);
                    }
                } else {
                    // Handle the error if path_str is None
                    eprintln!("Failed to convert the path to a string for file: {:?}", path_file);
                }
            } else {
                // Handle the error if get_file() returns None
                eprintln!("Failed to get the file from destination or the path is not valid for patch: {:?}", patch);
            }
        }

        println!("Writing changes to disk...");
        for (path, content) in &opened_files {
            match fs::write(path, content) {
                Ok(_) => println!("File written successfully: {}", path),
                Err(e) => eprintln!("Failed to write to file '{}': {}", path, e),
            };
        }
        println!("Patching complete.");



        Ok(())
    }

    pub fn unpatch(&mut self, patches: Vec<Patch>) -> Result<(), Error> {
        let mut opened_files: HashMap<String, String> = HashMap::new();


        for patch in &patches {
            if let Ok(Some(path_file)) = patch.destination.get_file() {
                // Convert the `Path` to a string and handle the potential `None` case
                if let Some(path_str) = path_file.to_str() {
                    let path_string = path_str.to_string();
                    // Handle the Result of `read_to_string` with `unwrap_or_else`
                    let content = opened_files
                        .entry(path_string.clone())
                        .or_insert_with(|| {
                            match fs::read_to_string(&path_file) {
                                Ok(content) => {
                                    content
                                }
                                Err(e) => {
                                    // Handle the error, e.g., by logging and continuing with an empty string
                                    eprintln!("Error reading the file '{}': {}", path_string, e);
                                    String::new() // return an empty string on error
                                }
                            }
                        });
                    let text_to_find = &patch.text_to_find;
                    let replacement_text = &patch.replacement_text;
                    if content.contains(replacement_text) {
                        // println!("Success: New->{}", text_to_find);
                        *content = content.replace(replacement_text, text_to_find);
                    } else {
                        println!("Failed to unpatch: '{}'", replacement_text);
                    }
                } else {
                    // Handle the error if path_str is None
                    eprintln!("Failed to convert the path to a string for file: {:?}", path_file);
                }
            } else {
                // Handle the error if get_file() returns None
                eprintln!("Failed to get the file from destination or the path is not valid for patch: {:?}", patch);
            }
        }

        println!("Reverting changes to disk...");
        for (path, content) in &opened_files {
            match fs::write(path, content) {
                Ok(_) => println!("File reverted successfully: {}", path),
                Err(e) => eprintln!("Failed to revert file '{}': {}", path, e),
            };
        }
        println!("Unpatching complete.");

        Ok(())
    }

    async fn send_message(&mut self, message: serde_json::Value) {
        let mut retries = 3;

        while retries > 0 {
            match self.socket.as_mut() {
                Some(socket) => match socket.send(Message::Text(message.to_string())) {
                    Ok(_) => {
                        println!("Successfully sent message., {:}", message);
                        break
                    }
                    Err(_) => {
                        eprintln!("Couldn't send message to Steam, retrying...");
                        self.connect().await;
                        retries -= 1;
                    }
                },
                None => {
                    self.connect().await;
                    retries -= 1;
                }
            }
        }
    }

    pub async fn reboot(&mut self) {
        self.send_message(serde_json::json!({
            "id": 1,
            "method": "Page.reload",
        }))
        .await;
    }

    pub async fn execute(&mut self, js_code: &str) {
        self.send_message(serde_json::json!({
            "id": 1,
            "method": "Runtime.evaluate",
            "params": {
                "expression": js_code,
            }
        }))
        .await;
    }

    async fn get_context() -> Option<String> {
        println!("Getting Steam...");

        let client = Client::new();
        let start_time = Instant::now();
        let uri = match "http://localhost:8080/json".parse::<Uri>() {
            Ok(uri) => uri,
            Err(_) => {
                println!("Error: Invalid URI.");
                return None;
            }
        };

        println!("URI: {}", uri);
        //Retry time
        while start_time.elapsed() < Duration::from_secs(60) {
            match client.get(uri.clone()).await {
                Ok(response) => {
                    match body::to_bytes(response.into_body()).await {
                        Ok(bytes) => {
                            match serde_json::from_slice::<Vec<Tab>>(&bytes) {
                                Ok(tabs) => {
                                    if let Some(tab) = tabs.into_iter().find(|tab| {
                                        tab.title == "SharedJSContext" && !tab.webSocketDebuggerUrl.is_empty()
                                    }) {
                                        return Some(tab.webSocketDebuggerUrl);
                                    } 
                                }
                                Err(_) => println!("Error: Failed to deserialize the response."),
                            }
                        }
                        Err(_) => println!("Error: Failed to read the response body."),
                    }
                }
                Err(e) => {
                    println!("Couldn't connect to Steam, retrying... {:?}",e);
                    if e.is_connect(){
                        println!("Failed to connect to host.");
                    } else if e.is_timeout(){
                        println!("Connection has timed out.");
                    } else {
                        println!("Error: {:?}", e);
                    }
                }
            }
            sleep(Duration::from_millis(50)).await;
        }
        println!("Timeout while trying to fetch Steam data!");
        None
        
    }

    pub fn new() -> SteamClient {
        return SteamClient { socket: None };
    }

    pub async fn connect(&mut self) {
        if let Some(context) = Self::get_context().await {
            self.socket = match connect(context) {
                Ok((socket, _)) => Some(socket),
                Err(_) => None,
            };
        }
    }
    fn is_patched() -> bool {
        let username = get_username(); // Assuming get_username() correctly returns a String
    
        // Use PathBuf for building the file path
        let mut path = if let Some(home_dir) = dirs::home_dir() {
            home_dir
        } else {
            return false;
        };
        path.push(format!("/home/{}/steam-patch/patched", username));
        path.exists()
    }
    fn create_patched_file() -> Result<(), Error> {
        let username = get_username(); 
    
        // Use PathBuf for building the file path
        let mut path = if let Some(home_dir) = dirs::home_dir() {
            home_dir
        } else {
            // Return an error if home directory cannot be determined
            return Err(Error::new(std::io::ErrorKind::NotFound, "Home directory not found"));
        };
    
        // Append the remaining path components
        path.push(format!("/home/{}/steam-patch/patched", username));
    
        // Create the file
        println!("Creating file at {:?}", path);
        File::create(path)?;
    
        Ok(())
    }
    fn remove_patched_file() -> Result<(), Error> {
        let username = get_username();
    
        // Use PathBuf for building the file path
        let mut path = if let Some(home_dir) = dirs::home_dir() {
            home_dir
        } else {
            // Return an error if home directory cannot be determined
            return Err(Error::new(std::io::ErrorKind::NotFound, "Home directory not found"));
        };
    
        // Append the remaining path components
        path.push(format!("/home/{}/steam-patch/patched", username));
    
        // Remove the file
        println!("Removing file at {:?}", path);
        fs::remove_file(path)?;
    
        Ok(())
    }
    
    const DEBUG_URL: &str = "http://localhost:8080/json";
    const TABS_TO_FIND: &'static[&'static str] = &["SharedJSContext"];

    async fn find_tabs() -> Result<bool, reqwest::Error> {
        let response = reqwest::get(Self::DEBUG_URL).await?;

        if response.status().is_success() {
            let tabs: Vec<Value> = response.json().await?;
            for tab in tabs {
                if let Some(title) = tab["title"].as_str() {
                    if Self::TABS_TO_FIND.contains(&title) {
                        return Ok(true);
                    }
                }
            }
        }
        Ok(false)
    }

    pub async fn watch() -> Option<tokio::task::JoinHandle<()>> {
        // If Steam client is already running, patch it and restart
        let mut client = Self::new();
        if Self::is_running()  {
            client.connect().await;
            if let Some(device) = create_device() {
                //Attempt to unpatch previous changes
                if Self::is_patched(){
                    match client.unpatch(device.get_patches()) {
                        Ok(_) => {
                            println!("Unpatching to remove previous patches and repatching.");
                            let _ = Self::remove_patched_file();
                        },
                        Err(_) => eprintln!("Couldn't unpatch Steam"),
                    }
                }
                //Then repatches new changes
                match client.patch(device.get_patches()) {
                    Ok(_) => {
                        println!("Steam was running and patched");
                        let _ = Self::create_patched_file();
                    },
                    Err(_) => eprintln!("Couldn't patch Steam"),
                }
            }
            // println!("Rebooting client");
            // client.reboot().await;
        }


        println!("Watching Steam cef status...");
        let task = tokio::spawn(async move {
            let mut patched = false;
            let mut server_was_down = false;


            loop {
                match SteamClient::find_tabs().await {
                    Ok(tabs_found) => {
                        server_was_down = false;
                        if tabs_found {
                            if !SteamClient::is_patched() {
                                if let Some(device) = create_device() {
                                    match client.patch(device.get_patches()) {
                                        Ok(_) => {
                                            println!("Steam patched");
                                            let _ = Self::create_patched_file();
                                        },
                                        Err(_) => eprintln!("Couldn't patch Steam"),
                                    }
                                    
                                }
                                println!("Rebooting client");
                                client.reboot().await;
                                patched = true;
                                println!(r#"{{"status": "patched"}}"#);
                            }
                        }
                    }
                    Err(_) => {
                        if !server_was_down {
                            server_was_down = true;
                            if SteamClient::is_patched() {
                                if let Some(device) = create_device() {
                                    match client.unpatch(device.get_patches()) {
                                        Ok(_) => {
                                            println!("Unpatching to remove previous patches and repatching.");
                                            let _ = Self::remove_patched_file();
                                        },
                                        Err(_) => eprintln!("Couldn't unpatch Steam"),
                                    }
                                }
                            }
                        }
                        println!("Server not available, rechecking in 1 seconds...");
                        tokio::time::sleep(Duration::from_millis(100)).await;
                        continue;
                    }
                }
                // Notes for later: Maybe check for all tabs that would be in a normal session rather than one.

                // Trial and sucess back to back runs
                // 300 worked 1x
                // 500 worked 2x bad 2x bad |NEW METHOD| 4x bad 1x
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        });  
        Some(task)   
    }

    

    fn is_running() -> bool {
        let mut sys = sysinfo::System::new_all();

        // We need to update the system value to get the fresh process list
        sys.refresh_all();

        sys.processes()
            .values()
            .any(|process| process.name() == "steam")
    }
}
