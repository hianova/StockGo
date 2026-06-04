mod lib_utils;
mod config;
mod parse;
mod crawl;
mod manager;
mod selecter;
mod backtest;
mod trust;

use std::collections::HashMap;
use std::io::{self, Write};
use std::sync::Arc;
use regex::Regex;
use config::Config;
use manager::Manager;
use selecter::Selecter;
use trust::TrustLayer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if !args.contains(&"--json".to_string()) {
        println!("Initializing Stockgo Rust...");
    }
    
    // Create folders if they don't exist
    std::fs::create_dir_all("downloads")?;
    std::fs::create_dir_all("strategy")?;
    
    // Initialize shared configuration
    let config = Arc::new(Config::new());
    let manager = Manager::new(&config);
    
    // Initialize trust layer (replaces IPFS)
    let trust = Arc::new(TrustLayer::new(config.clone())?);
    
    // Perform startup update if not skipped
    let args: Vec<String> = std::env::args().collect();
    if !args.contains(&"--skip-update".to_string()) {
        println!("Checking daily updates...");
        manager.update();
    }
    
    // CLI Argument Intercepts for ServerGo integration
    if args.contains(&"search".to_string()) && args.contains(&"--json".to_string()) {
        let mut line = String::new();
        io::stdin().read_line(&mut line)?;
        let query = line.trim().trim_start_matches("q=");
        
        let cmd_in = vec![format!("select {}", query)];
        let selecter = Selecter::new(cmd_in, &config);
        let data = match selecter.select(&config) {
            Ok(d) => d,
            Err(_) => Vec::new(),
        };
        
        if data.is_empty() {
            println!("[]");
            return Ok(());
        }
        
        let headers = &data[0];
        let mut json_arr = Vec::new();
        for row in data.iter().skip(1) {
            let mut obj = serde_json::Map::new();
            for (i, val) in row.iter().enumerate() {
                if i < headers.len() {
                    obj.insert(headers[i].clone(), serde_json::Value::String(val.clone()));
                }
            }
            json_arr.push(serde_json::Value::Object(obj));
        }
        println!("{}", serde_json::to_string(&json_arr).unwrap());
        return Ok(());
    }

    if args.contains(&"run".to_string()) && args.contains(&"backtest".to_string()) {
        let mut line = String::new();
        io::stdin().read_line(&mut line)?;
        let query = line.trim().trim_start_matches("q=");
        
        println!("Starting backtest for {}...", query);
        io::stdout().flush()?; // Ensure real-time WebSocket push
        
        let cmd_in = vec![format!("select {}", query)];
        let selecter = Selecter::new(cmd_in, &config);
        let data = match selecter.select(&config) {
            Ok(d) => d,
            Err(_) => Vec::new(),
        };
        
        if data.is_empty() || data.len() < 2 {
            println!("Error: No data available for backtest.");
            return Ok(());
        }
        
        println!("Running simulation algorithm on {} data points...", data.len() - 1);
        io::stdout().flush()?;
        
        let tester = backtest::BackTest::new(data.clone(), "default")?;
        
        println!("Backtest complete.");
        println!("Win Rate: {}", tester.get_win_rate());
        println!("Expected Value: {}", tester.get_expect_value());
        return Ok(());
    }
    
    let exit_rgx = Regex::new(r"(?i)exit").unwrap();
    let stdin = io::stdin();
    
    loop {
        println!("\nSelect function:");
        println!("                -M(manage config) -S(select data) -IO_OI(io_oi consensus ops)");
        print!("> ");
        io::stdout().flush()?;
        
        let mut line = String::new();
        if stdin.read_line(&mut line)? == 0 {
            break; // EOF
        }
        
        let cmd = line.trim();
        if cmd.is_empty() {
            continue;
        }
        
        if exit_rgx.is_match(cmd) {
            println!("Exiting Stockgo.");
            break;
        }
        
        // Parse prefix using regex
        let cmd_rgx = Regex::new(r"-\w+").unwrap();
        if let Some(mat) = cmd_rgx.find(cmd) {
            let mode = mat.as_str().replace('-', "");
            match mode.as_str() {
                "M" => {
                    // Manage Config Submenu
                    let config_list = config.list_config();
                    for item in config_list {
                        print!("{} ", item);
                    }
                    println!();
                    
                    loop {
                        println!("\nSelect \"manage\" function:");
                        println!("                           add(add list) update(update list) delete(del list)");
                        println!("                           addRelay(add relay) delRelay(del relay)");
                        println!("                           --help(how to use) or exit");
                        print!("(manage)> ");
                        io::stdout().flush()?;
                        
                        let mut sub_line = String::new();
                        if stdin.read_line(&mut sub_line)? == 0 {
                            break;
                        }
                        let sub_cmd = sub_line.trim();
                        if sub_cmd.is_empty() {
                            continue;
                        }
                        if exit_rgx.is_match(sub_cmd) {
                            break;
                        }
                        
                        if let Err(e) = handle_manager_cmd(sub_cmd, &manager) {
                            println!("Error: {}", e);
                        }
                    }
                }
                "S" => {
                    // Select Submenu
                    loop {
                        println!("\nSelect \"select\" function:");
                        println!("                           select(select data) export(export data) test(back test data)");
                        println!("                           view(quick view on data)");
                        println!("                           --help(how to use) or exit");
                        print!("(select)> ");
                        io::stdout().flush()?;
                        
                        let mut sub_line = String::new();
                        if stdin.read_line(&mut sub_line)? == 0 {
                            break;
                        }
                        let sub_cmd = sub_line.trim();
                        if sub_cmd.is_empty() {
                            continue;
                        }
                        if exit_rgx.is_match(sub_cmd) {
                            break;
                        }
                        
                        if let Err(e) = handle_selecter_cmd(sub_cmd, &config) {
                            println!("Error: {}", e);
                        }
                    }
                }
                "IO_OI" | "IPFS" => {
                    // Trust / io_oi consensus submenu (replacing old IPFS submenu)
                    loop {
                        println!("\nSelect \"IO_OI\" function:");
                        println!("                         import(import shared list) export(export shared list)");
                        println!("                         post(post signed article) delete(delete article)");
                        println!("                         profile(view node profile)");
                        println!("                         --help(how to use) or exit");
                        print!("(io_oi)> ");
                        io::stdout().flush()?;
                        
                        let mut sub_line = String::new();
                        if stdin.read_line(&mut sub_line)? == 0 {
                            break;
                        }
                        let sub_cmd = sub_line.trim();
                        if sub_cmd.is_empty() {
                            continue;
                        }
                        if exit_rgx.is_match(sub_cmd) {
                            break;
                        }
                        
                        if let Err(e) = handle_trust_cmd(sub_cmd, &trust) {
                            println!("Error: {}", e);
                        }
                    }
                }
                _ => println!("command not exist"),
            }
        } else {
            println!("invalid command");
        }
    }
    
    Ok(())
}

fn handle_manager_cmd(cmd: &str, manager: &Manager) -> Result<(), Box<dyn std::error::Error>> {
    if cmd.starts_with("add ") || cmd.starts_with("-A ") {
        let clean = cmd.replace("add ", "").replace("-A ", "");
        let parts: Vec<&str> = clean.split(',').map(|s| s.trim()).collect();
        if parts.len() < 4 {
            println!("Usage: add title,URL,label,status");
            return Ok(());
        }
        let mut map = HashMap::new();
        map.insert("title".to_string(), parts[0].to_string());
        map.insert("URL".to_string(), parts[1].to_string());
        map.insert("label".to_string(), parts[2].to_string());
        map.insert("status".to_string(), parts[3].to_string());
        manager.add_config(map)?;
    } else if cmd == "update" || cmd == "-U" {
        manager.update();
    } else if cmd.starts_with("delete ") || cmd.starts_with("-D ") {
        let clean = cmd.replace("delete ", "").replace("-D ", "");
        if let Ok(idx) = clean.parse::<usize>() {
            manager.delete_config(idx)?;
        } else {
            println!("Invalid index");
        }
    } else if cmd.starts_with("addRelay ") || cmd.starts_with("-addRelay ") {
        let clean = cmd.replace("addRelay ", "").replace("-addRelay ", "");
        let parts: Vec<&str> = clean.split('-').map(|s| s.trim()).collect();
        if parts.is_empty() {
            println!("Usage: addRelay relayName -key1=val1 -key2=val2");
            return Ok(());
        }
        let name = parts[0];
        let mut map = HashMap::new();
        for kv_part in parts.iter().skip(1) {
            let kv: Vec<&str> = kv_part.split('=').collect();
            if kv.len() == 2 {
                map.insert(kv[0].trim().to_string(), kv[1].trim().to_string());
            }
        }
        manager.add_relay(name, map)?;
        println!("Relay {} added", name);
    } else if cmd.starts_with("delRelay ") || cmd.starts_with("-delRelay ") {
        let clean = cmd.replace("delRelay ", "").replace("-delRelay ", "");
        manager.delete_relay(&clean)?;
        println!("Relay {} deleted", clean);
    } else if cmd == "--help" {
        println!("\n-M(manage config.txt) page command:");
        println!("    add(add list): type in add [custom title,URL,label,status]");
        println!("    update(update list): update to now");
        println!("    delete(del list): type in delete [number of list]");
    } else {
        println!("command not found");
    }
    Ok(())
}

// Global mutable selecter reference
static mut GLOBAL_SELECTER: Option<Selecter> = None;

fn handle_selecter_cmd(cmd: &str, config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    if cmd.starts_with("select ") || cmd.starts_with("-D ") {
        let clean = cmd.replace("select ", "").replace("-D ", "");
        // Support splitting multiple requests by comma
        let reqs: Vec<String> = clean.split(',').map(|s| s.trim().to_string()).collect();
        let sel = Selecter::new(reqs, config);
        let _ = sel.select(config)?;
        unsafe {
            GLOBAL_SELECTER = Some(sel);
        }
        println!("Data selected.");
    } else if cmd.starts_with("export ") || cmd.starts_with("-E ") {
        let clean = cmd.replace("export ", "").replace("-E ", "");
        unsafe {
            if let Some(ref sel) = GLOBAL_SELECTER {
                sel.export(&clean, true)?;
            } else {
                println!("please select data(select) first");
            }
        }
    } else if cmd.starts_with("test ") || cmd.starts_with("-T ") {
        let clean = cmd.replace("test ", "").replace("-T ", "");
        unsafe {
            if let Some(ref sel) = GLOBAL_SELECTER {
                let res = sel.back_test(&clean)?;
                println!("{}", res);
            } else {
                println!("please select data(select) first");
            }
        }
    } else if cmd == "view" || cmd == "-detail" {
        unsafe {
            if let Some(ref sel) = GLOBAL_SELECTER {
                let reqs = &sel.req;
                let data = sel.data.lock().unwrap();
                
                println!("\nrequest:");
                for (i, col_req) in reqs.iter().enumerate() {
                    let preview_len = std::cmp::min(col_req.len(), 10);
                    println!("  URL #{} req fields: {:?}", i, &col_req[..preview_len]);
                }
                
                println!("\ndata:");
                for (i, col) in data.iter().enumerate() {
                    let preview_len = std::cmp::min(col.len(), 10);
                    println!("  Column #{} preview: {:?}", i, &col[..preview_len]);
                }
            } else {
                println!("please select data(select) first");
            }
        }
    } else if cmd == "--help" {
        println!("\n-S(select data) page command:");
        println!("    select(select data): type in select [title -request req.req... option:(-date 8digit~8digit)(-numbers num.num...)],[]...");
        println!("    export(export data): type in export [path](default:downloads/export.csv)");
        println!("    test(back test data): type in test [strategy]");
        println!("    view(quick check on data): preview elements per column");
    } else {
        println!("command not found");
    }
    Ok(())
}

fn handle_trust_cmd(cmd: &str, trust: &TrustLayer) -> Result<(), Box<dyn std::error::Error>> {
    if cmd.starts_with("import ") || cmd.starts_with("-I ") {
        let hash = cmd.replace("import ", "").replace("-I ", "");
        trust.import_list(&hash)?;
    } else if cmd.starts_with("export ") || cmd.starts_with("-O ") {
        let clean = cmd.replace("export ", "").replace("-O ", "");
        if let Ok(idx) = clean.parse::<usize>() {
            let hash = trust.share_list(idx)?;
            println!("Export shared record hash: {}", hash);
        } else {
            println!("Invalid index");
        }
    } else if cmd.starts_with("post ") || cmd.starts_with("-P ") {
        let clean = cmd.replace("post ", "").replace("-P ", "");
        let private = clean.contains("--private");
        let body = clean.replace("--private", "").trim().to_string();
        trust.post_article(&body, private)?;
    } else if cmd.starts_with("delete ") {
        let hash = cmd.replace("delete ", "");
        trust.delete_article(&hash)?;
    } else if cmd == "profile" {
        println!("{}", trust.get_profile());
    } else if cmd == "--help" {
        println!("\n-IO_OI(io_oi consensus) page command:");
        println!("    import: type in import [record_hash]");
        println!("    export: type in export [number of config list]");
        println!("    post: type in post [--private] [article body]");
        println!("    delete: type in delete [article record_hash]");
        println!("    profile: view node profile");
    } else {
        println!("command not found");
    }
    Ok(())
}
