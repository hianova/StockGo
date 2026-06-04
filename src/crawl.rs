use std::fs;
use std::path::{Path, PathBuf};
use rand::Rng;
use regex::Regex;
use serde_json::Value;
use crate::lib_utils::LibUtils;

pub struct Crawl {
    url: String,
    tag: String,
    codec_name: String,
    path: PathBuf,
    post_data: Option<String>,
}

impl Crawl {
    pub fn new(url_in: &str) -> Self {
        // Parse host to generate TAG
        let host = url_in
            .split("://")
            .nth(1)
            .unwrap_or(url_in)
            .split('/')
            .next()
            .unwrap_or("")
            .split(':')
            .next()
            .unwrap_or("")
            .to_string();
            
        let tag = host.replace('.', "_");
        let codec_name = LibUtils::get_tag(&format!("{}/codec", tag));
        let codec_name = if codec_name.is_empty() { "UTF-8".to_string() } else { codec_name };
        
        let file_name = LibUtils::url_to_name(url_in);
        let path = PathBuf::from("downloads").join(format!("{}.txt", file_name));
        
        Self {
            url: url_in.to_string(),
            tag,
            codec_name,
            path,
            post_data: None,
        }
    }

    pub fn set_post(&mut self, form_in: &str) {
        self.post_data = Some(form_in.to_string());
    }

    pub fn set_path(&mut self, path_in: &str) {
        self.path = PathBuf::from(path_in);
    }

    pub fn save(&self) -> Result<String, Box<dyn std::error::Error>> {
        // Read user agents
        let mut uas = vec![
            "Opera/9.64 (Windows NT 6.0; U; pl) Presto/2.1.1".to_string(),
            "Mozilla/1.22 (compatible; MSIE 10.0; Windows 3.1)".to_string(),
            "Mozilla/4.0(compatible; MSIE 7.0b; Windows NT 6.0)".to_string(),
        ];
        
        let ua_path = Path::new("userAgent.txt");
        if ua_path.exists() {
            if let Ok(ua_content) = fs::read_to_string(ua_path) {
                let lines: Vec<String> = ua_content.lines()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                if !lines.is_empty() {
                    uas = lines;
                }
            }
        }
        
        let random_ua = &uas[rand::thread_rng().gen_range(0..uas.len())];
        
        // Setup ureq request
        // Java code replaces https with http, but we can keep it standard or allow both.
        // Twse requires HTTPS modern protocols now, so using HTTPS is safer.
        let parsed_url = self.url.clone();
        
        let agent = ureq::AgentBuilder::new()
            .redirects(10)
            .build();
            
        let request = if self.post_data.is_some() {
            agent.post(&parsed_url)
        } else {
            agent.get(&parsed_url)
        };
        
        // Extract host from url
        let host = parsed_url
            .split("://")
            .nth(1)
            .unwrap_or(&parsed_url)
            .split('/')
            .next()
            .unwrap_or("")
            .split(':')
            .next()
            .unwrap_or("");
            
        let mut request = request
            .set("Host", host)
            .set("Accept", "*/*")
            .set("Origin", &parsed_url)
            .set("Referer", &parsed_url)
            .set("Connection", "keep-alive")
            .set("X-Requested-With", "XMLHttpRequest")
            .set("Accept-Language", "zh-TW,zh-Hant;q=0.9")
            .set("User-Agent", random_ua)
            .set("Content-Type", &format!("application/x-www-form-urlencoded; charset={}", self.codec_name));
            
        let response = if let Some(ref form) = self.post_data {
            request.send_string(form)?
        } else {
            request.call()?
        };
        
        // Read response body as raw bytes
        let mut bytes = Vec::new();
        std::io::copy(&mut response.into_reader(), &mut bytes)?;
        
        // Decode bytes based on codec
        let decoded_str = if self.codec_name.to_uppercase() == "BIG5" {
            let (res, _, has_error) = encoding_rs::BIG5.decode(&bytes);
            if has_error {
                String::from_utf8_lossy(&bytes).into_owned()
            } else {
                res.into_owned()
            }
        } else {
            String::from_utf8(bytes).unwrap_or_else(|e| {
                String::from_utf8_lossy(e.as_bytes()).into_owned()
            })
        };
        
        let mut final_data = decoded_str;
        
        // Append domain tags based on file format
        if LibUtils::is_html(&final_data) {
            final_data.push_str("<tag>");
            final_data.push_str(&self.tag);
            final_data.push_str("</tag>");
        } else if LibUtils::is_json(&final_data) {
            if let Ok(mut json_val) = serde_json::from_str::<Value>(&final_data) {
                if let Some(obj) = json_val.as_object_mut() {
                    obj.insert("tag".to_string(), Value::String(self.tag.clone()));
                    if let Ok(pretty) = serde_json::to_string_pretty(&json_val) {
                        final_data = pretty;
                    }
                }
            }
        }
        
        // Create folder path if it does not exist
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        fs::write(&self.path, &final_data)?;
        println!("{} added", self.path.display());
        
        Ok(final_data)
    }
}
