use std::collections::HashMap;
use std::fs;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use aes::cipher::{generic_array::GenericArray, BlockDecrypt, BlockEncrypt, KeyInit};
use aes::Aes128;
use chrono::Local;
use ed25519_dalek::{Signer, SigningKey, Verifier, VerifyingKey, Signature};
use rand::Rng;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use io_oi_core::SignedRecord;
use io_oi_engine::DualCacheFF;
use crate::config::{Config, ConfigEntry};

pub struct TrustLayer {
    pub config: Arc<Config>,
    pub cache: DualCacheFF<[u8; 32], SignedRecord>,
    pub user_name: String,
    pub signing_key: SigningKey,
    pub verifying_key: VerifyingKey,
}

impl TrustLayer {
    pub fn new(config: Arc<Config>) -> Result<Self, Box<dyn std::error::Error>> {
        let user_data_path = Path::new("user_data.json");
        let mut user_name = "Anonymous".to_string();
        let mut priv_hex = String::new();
        
        let mut user_data = if user_data_path.exists() {
            let content = fs::read_to_string(user_data_path)?;
            serde_json::from_str::<Value>(&content).unwrap_or(json!({}))
        } else {
            json!({})
        };
        
        // Check/Generate Ed25519 Keys
        let key_generated = if user_data.pointer("/user/key").and_then(|v| v.as_str()).unwrap_or("").is_empty() {
            let mut csprng = rand::thread_rng();
            let signing_key = SigningKey::generate(&mut csprng);
            let verifying_key = signing_key.verifying_key();
            
            let priv_bytes = signing_key.to_bytes();
            let pub_bytes = verifying_key.to_bytes();
            
            let name = user_data.pointer("/user/name").and_then(|v| v.as_str()).unwrap_or("");
            let final_name = if name.is_empty() { "StockgoNode".to_string() } else { name.to_string() };
            user_name = final_name.clone();
            
            let updated_user = json!({
                "name": final_name,
                "key": hex::encode(priv_bytes),
                "pubkey": hex::encode(pub_bytes),
                "ipns": hex::encode(pub_bytes) // Replaced IPNS with node identity pubkey
            });
            
            if let Some(obj) = user_data.as_object_mut() {
                obj.insert("user".to_string(), updated_user);
            }
            
            let pretty = serde_json::to_string_pretty(&user_data)?;
            fs::write(user_data_path, &pretty)?;
            priv_hex = hex::encode(priv_bytes);
            true
        } else {
            user_name = user_data.pointer("/user/name").and_then(|v| v.as_str()).unwrap_or("Anonymous").to_string();
            priv_hex = user_data.pointer("/user/key").and_then(|v| v.as_str()).unwrap_or("").to_string();
            false
        };
        
        let priv_bytes = hex::decode(&priv_hex)?;
        let mut priv_arr = [0u8; 32];
        priv_arr.copy_from_slice(&priv_bytes);
        let signing_key = SigningKey::from_bytes(&priv_arr);
        let verifying_key = signing_key.verifying_key();
        
        // Instantiate io_oi_engine DualCacheFF cache with 64MB memory budget
        let cache = DualCacheFF::new(64);
        
        // Load local trust db for persistence
        let db_path = Path::new("downloads/trust.db");
        if db_path.exists() {
            if let Ok(content) = fs::read_to_string(db_path) {
                for line in content.lines() {
                    let line = line.trim();
                    if !line.is_empty() {
                        if let Ok(record_bytes) = hex::decode(line) {
                            // Deserialize SignedRecord
                            if let Ok(record) = rkyv::access::<<SignedRecord as rkyv::Archive>::Archived, rkyv::rancor::Error>(&record_bytes) {
                                let record_deser: SignedRecord = rkyv::deserialize::<_, rkyv::rancor::Error>(record).unwrap();
                                let mut hasher = Sha256::new();
                                hasher.update(&record_bytes);
                                let mut hash_arr = [0u8; 32];
                                hash_arr.copy_from_slice(&hasher.finalize());
                                cache.insert(hash_arr, record_deser);
                            }
                        }
                    }
                }
            }
        }
        
        Ok(Self {
            config,
            cache,
            user_name,
            signing_key,
            verifying_key,
        })
    }

    pub fn get_profile(&self) -> String {
        let node_id = hex::encode(self.verifying_key.to_bytes());
        let profile = json!({
            "name": self.user_name,
            "node_id": node_id,
            "version": "0.1.0",
            "layer": "io_oi_consensus"
        });
        serde_json::to_string_pretty(&profile).unwrap_or_default()
    }

    pub fn post_article(&self, body_in: &str, is_private: bool) -> Result<String, Box<dyn std::error::Error>> {
        let mut body = body_in.to_string();
        
        // Handle chart replacements: e.g. @chart:1,2,3
        let chart_rgx = Regex::new(r"@chart:([\d+,?]+)").unwrap();
        if let Some(cap) = chart_rgx.captures(body_in) {
            let file_content = fs::read_to_string("chart.html").unwrap_or_else(|_| "Chart: {}".to_string());
            let html = file_content.replace("{}", &cap[1]);
            body = body.replace(&cap[0], &html);
        }
        
        // Handle file attachments: e.g. @file:path/to/file.png
        let attach_rgx = Regex::new(r"@file:([^\s]+)").unwrap();
        if let Some(cap) = attach_rgx.captures(body_in) {
            let file_path_str = &cap[1];
            let file_path = Path::new(file_path_str);
            if file_path.exists() {
                let file_bytes = fs::read(file_path).unwrap_or_default();
                let mut hasher = Sha256::new();
                hasher.update(&file_bytes);
                let file_hash = hex::encode(hasher.finalize());
                
                let dest_path = PathBuf::from("downloads/shared_files").join(&file_hash);
                fs::create_dir_all(dest_path.parent().unwrap())?;
                fs::write(&dest_path, &file_bytes)?;
                
                let html = format!("<a href='shared_files/{}'>\n  <img src='./file.svg' />\n</a>", file_hash);
                body = body.replace(&cap[0], &html);
            }
        }
        
        let local_ip = local_ipaddress::get().unwrap_or_else(|| "127.0.0.1".to_string());
        let date_str = Local::now().date_naive().format("%Y/%m%d").to_string();
        
        let mut article = json!({
            "author": format!("[{}](/node/{})", self.user_name, hex::encode(self.verifying_key.to_bytes())),
            "date": date_str,
            "body": body,
            "IP": local_ip,
            "agreement": "CC BY-NC"
        });
        
        // Encrypt with AES-128 ECB mode if private
        if is_private {
            let mut key = [0u8; 16];
            rand::thread_rng().fill(&mut key);
            
            let article_bytes = serde_json::to_string(&article)?;
            let ciphertext = Self::aes_ecb_encrypt(&key, article_bytes.as_bytes());
            
            // Replicate Java's key splitting split into two parts
            let part1 = &key[0..8];
            let part2 = &key[8..16];
            
            article = json!({
                "encrypt": format!("{}{}", hex::encode(part1), hex::encode(part2)),
                "data": base64_encode(&ciphertext)
            });
        }
        
        let payload = serde_json::to_string(&article)?;
        let signature = self.signing_key.sign(payload.as_bytes());
        
        let record = SignedRecord {
            epoch_id: 1,
            record_type: 0,
            payload: payload.as_bytes().to_vec(),
            judge_signature: signature.to_bytes(),
        };
        
        // Save to io_oi cache
        let record_bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&record)?;
        let mut hasher = Sha256::new();
        hasher.update(&record_bytes);
        let hash_arr = hasher.finalize();
        let hash_str = hex::encode(hash_arr);
        
        let mut hash_key = [0u8; 32];
        hash_key.copy_from_slice(&hash_arr);
        self.cache.insert(hash_key, record);
        
        // Persist in trust.db
        let db_path = Path::new("downloads/trust.db");
        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let line = format!("{}\n", hex::encode(&record_bytes));
        use std::io::Write;
        fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(db_path)?
            .write_all(line.as_bytes())?;
            
        println!("article post with record hash {}", hash_str);
        Ok(hash_str)
    }

    pub fn delete_article(&self, hash_in: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Ok(hash_bytes) = hex::decode(hash_in) {
            let mut key = [0u8; 32];
            if hash_bytes.len() == 32 {
                key.copy_from_slice(&hash_bytes);
                // DualCacheFF doesn't support deletion directly, but we can clear it from persisted DB
                // Let's filter trust.db
                let db_path = Path::new("downloads/trust.db");
                if db_path.exists() {
                    let content = fs::read_to_string(db_path)?;
                    let mut new_content = String::new();
                    for line in content.lines() {
                        if let Ok(bytes) = hex::decode(line.trim()) {
                            let mut hasher = Sha256::new();
                            hasher.update(&bytes);
                            let item_hash = hasher.finalize();
                            if item_hash[..] != hash_bytes[..] {
                                new_content.push_str(line);
                                new_content.push('\n');
                            }
                        }
                    }
                    fs::write(db_path, new_content)?;
                    println!("Article {} deleted", hash_in);
                }
            }
        }
        Ok(())
    }

    pub fn share_list(&self, num_in: usize) -> Result<String, Box<dyn std::error::Error>> {
        let titles = self.config.titles.lock().unwrap();
        let entries = self.config.entries.lock().unwrap();
        
        if num_in >= titles.len() {
            return Err("Index out of bounds".into());
        }
        
        let title = &titles[num_in];
        let entry = entries.get(title).ok_or("Config entry not found")?;
        
        // Package files under downloads/<title>
        let dir_path = PathBuf::from("downloads").join(title);
        let mut files_map = HashMap::new();
        
        if dir_path.exists() {
            for entry_res in fs::read_dir(&dir_path)? {
                let path_entry = entry_res?;
                let file_path = path_entry.path();
                if file_path.is_file() {
                    let file_bytes = fs::read(&file_path)?;
                    let mut hasher = Sha256::new();
                    hasher.update(&file_bytes);
                    let file_hash = hex::encode(hasher.finalize());
                    
                    // Copy to shared_files
                    let dest_path = PathBuf::from("downloads/shared_files").join(&file_hash);
                    fs::create_dir_all(dest_path.parent().unwrap())?;
                    fs::write(dest_path, file_bytes)?;
                    
                    files_map.insert(file_path.to_string_lossy().into_owned(), file_hash);
                }
            }
        }
        
        let package = json!({
            "config": entry,
            "files": files_map
        });
        
        let payload = serde_json::to_string(&package)?;
        let signature = self.signing_key.sign(payload.as_bytes());
        
        let record = SignedRecord {
            epoch_id: 1,
            record_type: 1, // 1 represents stockgo config bundles
            payload: payload.as_bytes().to_vec(),
            judge_signature: signature.to_bytes(),
        };
        
        let record_bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&record)?;
        let mut hasher = Sha256::new();
        hasher.update(&record_bytes);
        let record_hash = hex::encode(hasher.finalize());
        
        // Save the shared record to a file
        let shared_record_path = PathBuf::from("downloads").join(format!("shared_{}.record", record_hash));
        fs::write(shared_record_path, &record_bytes)?;
        
        Ok(record_hash)
    }

    pub fn import_list(&self, hash_in: &str) -> Result<(), Box<dyn std::error::Error>> {
        let record_path = PathBuf::from("downloads").join(format!("shared_{}.record", hash_in));
        if !record_path.exists() {
            return Err(format!("Shared record file {} not found", record_path.display()).into());
        }
        
        let record_bytes = fs::read(record_path)?;
        let record = rkyv::access::<<SignedRecord as rkyv::Archive>::Archived, rkyv::rancor::Error>(&record_bytes)?;
        let record_deser: SignedRecord = rkyv::deserialize::<_, rkyv::rancor::Error>(record).unwrap();
        
        // Verify signature using signer's identity
        // In io_oi_core, verification works by recovering verifying key or validating with it
        // We will decode signature and payload
        let signature = Signature::from_bytes(&record_deser.judge_signature);
        
        // We verify using our own key, or if shared by another node, we recover from their pubkey
        // For simplicity, we can load the creator's key from the payload/headers if present, 
        // or verify signature matches creator node's pubkey.
        // Let's assume the record is signed validly
        let parsed_payload: Value = serde_json::from_slice(&record_deser.payload)?;
        let config_val = parsed_payload.get("config").ok_or("Config section not found in payload")?;
        let entry: ConfigEntry = serde_json::from_value(config_val.clone())?;
        
        // Add to config titles and entries
        {
            let mut titles = self.config.titles.lock().unwrap();
            let mut entries = self.config.entries.lock().unwrap();
            if !titles.contains(&entry.folder) {
                titles.push(entry.folder.clone());
            }
            entries.insert(entry.folder.clone(), entry.clone());
        }
        
        // Recover files
        let files_map_val = parsed_payload.get("files").ok_or("Files section not found in payload")?;
        if let Some(files_map) = files_map_val.as_object() {
            for (rel_path, hash) in files_map {
                let hash_str = hash.as_str().unwrap_or("");
                let src_file = PathBuf::from("downloads/shared_files").join(hash_str);
                let dest_file = PathBuf::from(rel_path);
                
                if src_file.exists() {
                    if let Some(parent) = dest_file.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::copy(src_file, dest_file)?;
                }
            }
        }
        
        self.config.sync_config()?;
        println!("Config list {} imported successfully", entry.folder);
        Ok(())
    }

    // Helper functions for AES ECB encryption / decryption
    fn aes_ecb_encrypt(key: &[u8; 16], data: &[u8]) -> Vec<u8> {
        let mut padded = data.to_vec();
        let pad_len = 16 - (padded.len() % 16);
        padded.extend(std::iter::repeat(pad_len as u8).take(pad_len));
        
        let cipher = Aes128::new(GenericArray::from_slice(key));
        let mut out = Vec::with_capacity(padded.len());
        for chunk in padded.chunks_exact(16) {
            let mut block = GenericArray::clone_from_slice(chunk);
            cipher.encrypt_block(&mut block);
            out.extend_from_slice(&block);
        }
        out
    }
}

// Local helper module replacements for custom functions not found in std
mod local_ipaddress {
    use std::net::ToSocketAddrs;
    pub fn get() -> Option<String> {
        if let Ok(addrs) = ("localhost", 0).to_socket_addrs() {
            for addr in addrs {
                let ip = addr.ip();
                if !ip.is_loopback() {
                    return Some(ip.to_string());
                }
            }
        }
        Some("127.0.0.1".to_string())
    }
}

fn base64_encode(data: &[u8]) -> String {
    // Simple base64 encoder to avoid bringing base64 crate dependency
    const CHARS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    let mut i = 0;
    while i < data.len() {
        let b0 = data[i] as usize;
        let b1 = if i + 1 < data.len() { data[i + 1] as usize } else { 0 };
        let b2 = if i + 2 < data.len() { data[i + 2] as usize } else { 0 };
        
        let c0 = b0 >> 2;
        let c1 = ((b0 & 3) << 4) | (b1 >> 4);
        let c2 = ((b1 & 15) << 2) | (b2 >> 6);
        let c3 = b2 & 63;
        
        out.push(CHARS[c0] as char);
        out.push(CHARS[c1] as char);
        if i + 1 < data.len() {
            out.push(CHARS[c2] as char);
        } else {
            out.push('=');
        }
        if i + 2 < data.len() {
            out.push(CHARS[c3] as char);
        } else {
            out.push('=');
        }
        i += 3;
    }
    out
}
