use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use chrono::{Duration, Local, NaiveDate};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::lib_utils::LibUtils;
use crate::parse::Parse;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConfigEntry {
    #[serde(rename = "URL")]
    pub url: String,
    pub folder: String,
    #[serde(rename = "tag")]
    pub label: String,
    pub status: String,
}

pub struct Config {
    pub titles: Mutex<Vec<String>>,
    pub entries: Mutex<HashMap<String, ConfigEntry>>,
}

impl Config {
    pub fn new() -> Self {
        let mut titles = Vec::new();
        let mut entries = HashMap::new();
        
        let config_path = Path::new("downloads/config.json");
        if config_path.exists() {
            if let Ok(content) = fs::read_to_string(config_path) {
                if let Ok(json_map) = serde_json::from_str::<HashMap<String, ConfigEntry>>(&content) {
                    for (title, entry) in json_map {
                        titles.push(title.clone());
                        entries.insert(title, entry);
                    }
                }
            }
        }
        
        Self {
            titles: Mutex::new(titles),
            entries: Mutex::new(entries),
        }
    }

    pub fn list_config(&self) -> Vec<String> {
        let titles = self.titles.lock().unwrap();
        let entries = self.entries.lock().unwrap();
        let mut out = vec![
            "number".to_string(),
            "title".to_string(),
            "URL".to_string(),
            "folder".to_string(),
            "label".to_string(),
            "status".to_string(),
            "\n".to_string(),
        ];
        
        for (i, title) in titles.iter().enumerate() {
            if let Some(entry) = entries.get(title) {
                out.push(i.to_string());
                out.push(title.clone());
                out.push(entry.url.clone());
                out.push(entry.folder.clone());
                out.push(entry.label.clone());
                out.push(format!("{}\n", entry.status));
            }
        }
        out
    }

    pub fn sync_config(&self) -> Result<(), std::io::Error> {
        let titles = self.titles.lock().unwrap();
        let entries = self.entries.lock().unwrap();
        
        let mut json_map = serde_json::Map::new();
        for title in titles.iter() {
            if let Some(entry) = entries.get(title) {
                if let Ok(val) = serde_json::to_value(entry) {
                    json_map.insert(title.clone(), val);
                }
            }
        }
        
        let pretty = serde_json::to_string_pretty(&serde_json::Value::Object(json_map))?;
        fs::write("downloads/config.json", &pretty)?;
        // Java code writes to downloads/config.txt too
        fs::write("downloads/config.txt", &pretty)?;
        Ok(())
    }

    pub fn stream_date(&self, url_in: &str, st_ed_in: &str) -> Vec<String> {
        let date_rgx = Regex::new(r"@date").unwrap();
        if !date_rgx.is_match(url_in) {
            return vec!["".to_string()];
        }
        
        let entries = self.entries.lock().unwrap();
        let entry = entries.values().find(|e| e.url == url_in);
        if entry.is_none() {
            return vec!["".to_string()];
        }
        let entry = entry.unwrap();
        
        // Extract label details
        let label_str = entry.label.split(',')
            .filter(|p| !date_rgx.is_match(p))
            .collect::<String>();
        let label_parts: Vec<&str> = label_str.split(':')
            .map(|s| s.trim())
            .collect();
            
        if label_parts.len() < 3 {
            return vec!["".to_string()];
        }
        
        let format_pattern = label_parts[1];
        let step_size = label_parts[2];
        
        let st_ed_rgx = Regex::new(r"\d+~\d+").unwrap();
        let mut start_date: NaiveDate;
        let mut end_date: NaiveDate;
        
        let has_yyyy = format_pattern.contains("yyyy");
        
        if st_ed_rgx.is_match(st_ed_in) {
            let parts: Vec<&str> = st_ed_in.split('~').collect();
            start_date = NaiveDate::parse_from_str(parts[0], "%Y%m%d").unwrap_or_else(|_| Local::now().date_naive());
            end_date = NaiveDate::parse_from_str(parts[1], "%Y%m%d").unwrap_or_else(|_| Local::now().date_naive());
        } else {
            // Check status
            let status = &entry.status;
            let mut parsed_year_status = NaiveDate::parse_from_str(status, "%Y%m%d").unwrap_or_else(|_| Local::now().date_naive());
            if !has_yyyy {
                // If it is Minguo status (e.g. year is 111), parse_from_str gets year 111. 
                // We add 1911 years to match Gregorian year 2022.
                if let Some(d) = parsed_year_status.checked_add_months(chrono::Months::new(1911 * 12)) {
                    parsed_year_status = d;
                }
            }
            start_date = parsed_year_status;
            end_date = Local::now().date_naive();
        }
        
        let mut out = Vec::new();
        while start_date < end_date {
            out.push(start_date.format("%Y%m%d").to_string());
            match step_size {
                "M" => {
                    start_date = start_date.checked_add_months(chrono::Months::new(1)).unwrap_or(start_date + Duration::days(30));
                }
                "W" => {
                    start_date += Duration::days(7);
                }
                "D" => {
                    start_date += Duration::days(1);
                }
                _ => {
                    start_date += Duration::days(1);
                }
            }
        }
        out
    }

    pub fn to_origin(&self, date_in: &str, url_in: &str) -> String {
        if date_in.is_empty() {
            return date_in.to_string();
        }
        
        let date_rgx = Regex::new(r"@date").unwrap();
        let entries = self.entries.lock().unwrap();
        let entry = entries.values().find(|e| e.url == url_in);
        if entry.is_none() {
            return date_in.to_string();
        }
        let entry = entry.unwrap();
        
        let num_rgx = Regex::new(r"@num").unwrap();
        let label_str = entry.label.split(',')
            .filter(|p| !num_rgx.is_match(p))
            .collect::<String>();
        let label_parts: Vec<&str> = label_str.split(':')
            .map(|s| s.trim())
            .collect();
            
        if label_parts.len() < 2 {
            return date_in.to_string();
        }
        
        let pattern = label_parts[1];
        let date = NaiveDate::parse_from_str(date_in, "%Y%m%d").unwrap_or_else(|_| Local::now().date_naive());
        
        if !pattern.contains("yyyy") {
            // Minguo Year logic: subtract 1911
            let minguo_year = date.format("%Y").to_string().parse::<i32>().unwrap_or(0) - 1911;
            // Format format without year, and replace 'yyy' or 'yy' with minguo_year
            let pattern_without_year = pattern.replace("yyyy", "").replace("yyy", "").replace("yy", "");
            let mut formatted = date.format(&pattern_without_year).to_string();
            // Prefix or insert Minguo year in the format
            if pattern.starts_with("yyy") || pattern.starts_with("yy") {
                formatted = format!("{}{}", minguo_year, formatted);
            } else {
                formatted = format!("{}{}", formatted, minguo_year);
            }
            formatted
        } else {
            // Standard Gregorian format
            date.format(pattern).to_string()
        }
    }

    pub fn stream_num(&self, url_in: &str, num_in: &str) -> Vec<String> {
        let num_rgx = Regex::new(r"@num").unwrap();
        if !num_rgx.is_match(url_in) {
            return vec!["".to_string()];
        }
        
        let num_in_rgx = Regex::new(r"\w+(\.\w+)+").unwrap();
        if num_in_rgx.is_match(num_in) {
            return num_in.split('.').map(|s| s.to_string()).collect();
        }
        
        let entries = self.entries.lock().unwrap();
        let entry = entries.values().find(|e| e.url == url_in);
        if entry.is_none() {
            return vec!["".to_string()];
        }
        let entry = entry.unwrap();
        
        let label_str = entry.label.split(',')
            .filter(|p| !num_rgx.is_match(p))
            .collect::<String>();
        let label_parts: Vec<&str> = label_str.split(':')
            .map(|s| s.trim())
            .collect();
            
        if label_parts.is_empty() {
            return vec!["".to_string()];
        }
        
        let label_type = label_parts[0];
        match label_type {
            "stock" => {
                let sub_type = if label_parts.len() > 1 { label_parts[1] } else { "" };
                self.stock_num(sub_type)
            }
            "ETF" => {
                self.etf_num()
            }
            _ => vec!["".to_string()],
        }
    }

    pub fn stock_num(&self, name_in: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut lists = Vec::new();
        let rgx = Regex::new(r"^[0-9]{4}　").unwrap();
        
        if name_in == "listed" || name_in == "上市" {
            lists.push("上市");
        } else if name_in == "OTC" || name_in == "上櫃" {
            lists.push("上櫃");
        } else {
            lists.push("上市");
            lists.push("上櫃");
        }
        
        for next_list in lists {
            let path = format!("downloads/{}證券代號/isin_C_public.txt", next_list);
            if let Ok(mut parse) = Parse::new(&path, vec!["有價證券代號及名稱".to_string()], self) {
                let data = parse.data();
                if !data.is_empty() {
                    for next in &data[0] {
                        if rgx.is_match(next) {
                            let code = next.split('　').next().unwrap_or("").to_string();
                            out.push(code);
                        }
                    }
                }
            } else {
                println!("number not exist for path: {}", path);
            }
        }
        out
    }

    pub fn etf_num(&self) -> Vec<String> {
        let mut out = Vec::new();
        let rgx = Regex::new(r"^T[0-9]+\w").unwrap();
        let path = "downloads/基金＿國際證券代號/isin_C_public.txt";
        
        if let Ok(mut parse) = Parse::new(path, vec!["有價證券代號及名稱".to_string()], self) {
            let data = parse.data();
            if !data.is_empty() {
                for next in &data[0] {
                    if rgx.is_match(next) {
                        let code = next.split('　').next().unwrap_or("").to_string();
                        out.push(code);
                    }
                }
            }
        }
        out
    }


    pub fn relay(&self, name_in: &str) -> HashMap<String, String> {
        let mut out = HashMap::new();
        let path = Path::new("downloads/relay.json");
        if path.exists() {
            if let Ok(content) = fs::read_to_string(path) {
                if let Ok(json) = serde_json::from_str::<Value>(&content) {
                    if json.get(name_in).is_some() {
                        let base = format!("/{}", name_in);
                        out.insert("URL".to_string(), json.pointer(&format!("{}/URL", base)).and_then(|v| v.as_str()).unwrap_or("").to_string());
                        out.insert("request".to_string(), json.pointer(&format!("{}/request", base)).and_then(|v| v.as_str()).unwrap_or("").to_string());
                        out.insert("date".to_string(), json.pointer(&format!("{}/date", base)).and_then(|v| v.as_str()).unwrap_or("").to_string());
                        out.insert("num".to_string(), json.pointer(&format!("{}/num", base)).and_then(|v| v.as_str()).unwrap_or("").to_string());
                        return out;
                    }
                }
            }
        }
        out.insert("URL".to_string(), "".to_string());
        out.insert("request".to_string(), "".to_string());
        out.insert("date".to_string(), "".to_string());
        out.insert("num".to_string(), "".to_string());
        out
    }
}
