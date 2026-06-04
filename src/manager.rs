use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use chrono::{Local, NaiveDate};
use regex::Regex;
use serde_json::{json, Value};
use crate::config::{Config, ConfigEntry};
use crate::crawl::Crawl;
use crate::lib_utils::LibUtils;
use rand::Rng;

pub struct Manager<'a> {
    pub config: &'a Config,
}

impl<'a> Manager<'a> {
    pub fn new(config: &'a Config) -> Self {
        Self { config }
    }

    pub fn add_config(&self, list_in: HashMap<String, String>) -> Result<(), Box<dyn std::error::Error>> {
        let title = list_in.get("title").cloned().unwrap_or_default();
        let url = list_in.get("URL").cloned().unwrap_or_default();
        let label = list_in.get("label").cloned().unwrap_or_default();
        let status = list_in.get("status").cloned().unwrap_or_default();
        
        {
            let titles = self.config.titles.lock().unwrap();
            if titles.contains(&title) {
                println!("Title exists");
                return Ok(());
            }
        }
        
        let entry = ConfigEntry {
            url: url.clone(),
            folder: title.clone(),
            label,
            status,
        };
        
        {
            let mut titles = self.config.titles.lock().unwrap();
            let mut entries = self.config.entries.lock().unwrap();
            titles.push(title.clone());
            entries.insert(title.clone(), entry);
        }
        
        self.download(&url)?;
        
        // Update status to today
        let today = Local::now().date_naive().format("%Y%m%d").to_string();
        {
            let mut entries = self.config.entries.lock().unwrap();
            if let Some(e) = entries.get_mut(&title) {
                e.status = today;
            }
        }
        
        self.config.sync_config()?;
        println!("{} added", title);
        
        Ok(())
    }

    pub fn delete_config(&self, num_in: usize) -> Result<(), Box<dyn std::error::Error>> {
        let mut title_to_remove = String::new();
        {
            let mut titles = self.config.titles.lock().unwrap();
            let mut entries = self.config.entries.lock().unwrap();
            if num_in < titles.len() {
                title_to_remove = titles.remove(num_in);
                entries.remove(&title_to_remove);
            }
        }
        
        if !title_to_remove.is_empty() {
            self.config.sync_config()?;
            println!("{} deleted", num_in);
        } else {
            println!("Index {} out of bounds", num_in);
        }
        Ok(())
    }

    pub fn update(&self) {
        let titles: Vec<String> = self.config.titles.lock().unwrap().clone();
        
        for title in titles {
            let mut should_update = false;
            let mut url = String::new();
            {
                let entries = self.config.entries.lock().unwrap();
                if let Some(entry) = entries.get(&title) {
                    url = entry.url.clone();
                    if let Ok(status_date) = NaiveDate::parse_from_str(&entry.status, "%Y%m%d") {
                        let today = Local::now().date_naive();
                        let duration = today - status_date;
                        if duration.num_days() > 1 {
                            should_update = true;
                        }
                    } else {
                        should_update = true; // bad status date format, update anyway
                    }
                }
            }
            
            if should_update {
                println!("Updating: {}", title);
                if let Err(e) = self.download(&url) {
                    println!("Update has been suspended for {}: {}", title, e);
                    continue;
                }
                
                let today = Local::now().date_naive().format("%Y%m%d").to_string();
                {
                    let mut entries = self.config.entries.lock().unwrap();
                    if let Some(entry) = entries.get_mut(&title) {
                        entry.status = today;
                    }
                }
                if let Err(e) = self.config.sync_config() {
                    println!("Failed to sync config after update of {}: {}", title, e);
                }
            }
        }
        println!("Files are up to update");
    }

    pub fn download(&self, url_in: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut folder = String::new();
        {
            let entries = self.config.entries.lock().unwrap();
            if let Some(entry) = entries.values().find(|e| e.url == url_in) {
                folder = entry.folder.clone();
            }
        }
        
        if folder.is_empty() {
            return Err("Config entry matching URL not found".into());
        }
        
        let dir = PathBuf::from("downloads").join(&folder);
        fs::create_dir_all(&dir)?;
        
        let num_rgx = Regex::new(r"@num").unwrap();
        let date_rgx = Regex::new(r"@date").unwrap();
        let post_rgx = Regex::new(r"@Post:").unwrap();
        
        let nums = self.config.stream_num(url_in, "");
        let dates = self.config.stream_date(url_in, "");
        
        for next_num in &nums {
            for next_date in &dates {
                // Perform replacement on template
                let replaced_url = url_in
                    .replace("@date", &self.config.to_origin(next_date, url_in))
                    .replace("@num", next_num);
                    
                let url_parts: Vec<&str> = replaced_url.split("@Post:").collect();
                let actual_url = url_parts[0];
                
                let clean_url_for_name = actual_url.to_string();
                let file_base = LibUtils::url_to_name(&clean_url_for_name);
                
                let num_suffix = if num_rgx.is_match(url_in) { format!("_{}", next_num) } else { "".to_string() };
                let date_suffix = if date_rgx.is_match(url_in) { format!("_{}", next_date) } else { "".to_string() };
                
                let path_str = format!("{}/{}{}{}.txt", dir.display(), file_base, num_suffix, date_suffix);
                
                let mut crawl = Crawl::new(actual_url);
                crawl.set_path(&path_str);
                
                if post_rgx.is_match(url_in) && url_parts.len() > 1 {
                    crawl.set_post(url_parts[1]);
                }
                
                if let Err(e) = crawl.save() {
                    println!("Time iterator stopped: {}", e);
                }
                
                // Sleep up to 5000ms randomly to be gentle toTWSE/TPEx servers
                let sleep_ms = rand::thread_rng().gen_range(500..5000);
                std::thread::sleep(std::time::Duration::from_millis(sleep_ms));
            }
        }
        
        Ok(())
    }

    pub fn add_relay(&self, name_in: &str, list_in: HashMap<String, String>) -> Result<(), Box<dyn std::error::Error>> {
        let path = Path::new("downloads/relay.json");
        let mut json_data = if path.exists() {
            let content = fs::read_to_string(path)?;
            serde_json::from_str::<Value>(&content).unwrap_or(json!({}))
        } else {
            json!({})
        };
        
        if let Some(obj) = json_data.as_object_mut() {
            let mut entry = serde_json::Map::new();
            for (k, v) in list_in {
                entry.insert(k, Value::String(v));
            }
            obj.insert(name_in.to_string(), Value::Object(entry));
        }
        
        let pretty = serde_json::to_string_pretty(&json_data)?;
        fs::write(path, &pretty)?;
        Ok(())
    }

    pub fn delete_relay(&self, name_in: &str) -> Result<(), Box<dyn std::error::Error>> {
        let path = Path::new("downloads/relay.json");
        if path.exists() {
            let content = fs::read_to_string(path)?;
            let mut json_data = serde_json::from_str::<Value>(&content).unwrap_or(json!({}));
            if let Some(obj) = json_data.as_object_mut() {
                obj.remove(name_in);
            }
            let pretty = serde_json::to_string_pretty(&json_data)?;
            fs::write(path, &pretty)?;
        }
        Ok(())
    }
}
