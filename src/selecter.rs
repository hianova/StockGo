use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use regex::Regex;
use cdDB::{CdDBDispatcher, WriteCommand, Query, QueryNode, QueryResult, Attributes};
use crate::config::Config;
use crate::lib_utils::LibUtils;
use crate::parse::Parse;
use crate::backtest::BackTest;

pub struct Selecter {
    pub urls: Vec<String>,
    pub nums: Vec<String>,
    pub dates: Vec<String>,
    pub req: Vec<Vec<String>>,
    pub data: Arc<Mutex<Vec<Vec<String>>>>,
    pub check_date: bool,
}

impl Selecter {
    pub fn new(cmd_in: Vec<String>, config: &Config) -> Self {
        let mut urls = Vec::new();
        let mut nums = Vec::new();
        let mut dates = Vec::new();
        let mut req: Vec<Vec<String>> = Vec::new();
        let mut check_date = true;
        let all_rgx = Regex::new(r"^ALL$").unwrap();
        
        for cmd_item in cmd_in {
            // Split by '-'
            let cmd_parts: Vec<String> = cmd_item.split('-')
                .map(|s| s.trim().to_string())
                .collect();
                
            if cmd_parts.is_empty() {
                continue;
            }
            
            let name_or_url = &cmd_parts[0];
            let mut map = config.relay(name_or_url);
            
            if map.get("URL").unwrap().is_empty() {
                map.insert("URL".to_string(), name_or_url.clone());
            }
            
            for part in cmd_parts.iter().skip(1) {
                let kv: Vec<&str> = part.split('=').collect();
                if kv.len() == 2 {
                    map.insert(kv[0].trim().to_string(), kv[1].trim().to_string());
                }
            }
            
            if let Some(wd) = map.get("withdate") {
                if wd.contains("false") || wd.contains("fales") {
                    check_date = false;
                }
            }
            
            let url = map.get("URL").unwrap().clone();
            let req_str = map.get("request").unwrap().clone();
            let req_list: Vec<String> = req_str.split('.').map(|s| s.to_string()).collect();
            
            urls.push(url);
            req.push(req_list);
            dates.push(map.get("date").unwrap().clone());
            nums.push(map.get("num").unwrap().clone());
        }
        
        // Resolve 'ALL' request fields
        for i in 0..req.len() {
            if !req[i].is_empty() && all_rgx.is_match(&req[i][0]) {
                // Read downloads/<folder>/index.csv
                let folder = {
                    let entries = config.entries.lock().unwrap();
                    entries.values().find(|e| e.url == urls[i]).map(|e| e.folder.clone()).unwrap_or_default()
                };
                let index_path = PathBuf::from("downloads").join(folder).join("index.csv");
                if index_path.exists() {
                    if let Ok(content) = fs::read_to_string(index_path) {
                        let fields: Vec<String> = content.split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                        if !fields.is_empty() {
                            req[i] = fields;
                        }
                    }
                } else {
                    println!("index.csv not found: ALL command not available");
                }
            }
        }
        
        let mut total_cols = 0;
        for i in 0..req.len() {
            total_cols += req[i].len();
            if check_date && dates[i].is_empty() {
                total_cols += 1;
            }
        }
        
        Self {
            urls,
            nums,
            dates,
            req,
            data: Arc::new(Mutex::new(vec![Vec::new(); total_cols])),
            check_date,
        }
    }

    pub fn select(&self, config: &Config) -> Result<Vec<Vec<String>>, Box<dyn std::error::Error>> {
        // Initialize cdDB dispatcher
        let mut db: CdDBDispatcher<1024> = CdDBDispatcher::new_std(None);
        let mut writers = HashMap::new();
        
        let num_urls = self.urls.len();
        
        // Target column write indexes
        let mut col_offsets = Vec::new();
        let mut current_offset = 0;
        for i in 0..num_urls {
            col_offsets.push(current_offset);
            let mut num_cols_for_url = self.req[i].len();
            if self.check_date && self.dates[i].is_empty() {
                num_cols_for_url += 1;
            }
            current_offset += num_cols_for_url;
            
            let url = &self.urls[i];
            let date_param = &self.dates[i];
            let num_param = &self.nums[i];
            let nums = config.stream_num(url, num_param);
            let dates = config.stream_date(url, date_param);
            
            let folder = {
                let entries = config.entries.lock().unwrap();
                entries.values().find(|e| e.url == *url).map(|e| e.folder.clone()).unwrap_or_default()
            };
            
            for next_num in &nums {
                for next_date in &dates {
                    let partition_name = format!("{}_{}_{}", folder, next_num, next_date)
                        .replace('.', "_")
                        .replace('/', "_");
                    let writer = db.register_partition(partition_name.clone());
                    writers.insert(partition_name, writer);
                }
            }
        }
        
        let db = Arc::new(db);
        let writers = Arc::new(writers);
        let mut threads = Vec::new();
        
        for url_idx in 0..num_urls {
            let url = self.urls[url_idx].clone();
            let req_fields = self.req[url_idx].clone();
            let date_param = self.dates[url_idx].clone();
            let num_param = self.nums[url_idx].clone();
            let col_offset = col_offsets[url_idx];
            let check_date = self.check_date;
            
            // Shared config pointers
            let nums = config.stream_num(&url, &num_param);
            let dates = config.stream_date(&url, &date_param);
            
            // We need to parse matching folder
            let folder = {
                let entries = config.entries.lock().unwrap();
                entries.values().find(|e| e.url == url).map(|e| e.folder.clone()).unwrap_or_default()
            };
            
            let data_share = self.data.clone();
            let db_clone = db.clone();
            let writers_clone = writers.clone();
            
            // Spawning a background thread for each URL source
            let handle = std::thread::spawn(move || {
                let dir = PathBuf::from("downloads").join(&folder);
                let num_rgx = Regex::new(r"@num").unwrap();
                let date_rgx = Regex::new(r"@date").unwrap();
                
                let num_cols = req_fields.len() + if check_date && date_param.is_empty() { 1 } else { 0 };
                let mut local_data = vec![Vec::new(); num_cols];
                
                for next_num in &nums {
                    for next_date in &dates {
                        let clean_url = url.split("@Post:").next().unwrap_or(&url);
                        let file_base = LibUtils::url_to_name(clean_url);
                        
                        let num_suffix = if num_rgx.is_match(&url) { format!("_{}", next_num) } else { "".to_string() };
                        let date_suffix = if date_rgx.is_match(&url) { format!("_{}", next_date) } else { "".to_string() };
                        
                        let path = format!("{}/{}{}{}.txt", dir.display(), file_base, num_suffix, date_suffix);
                        
                        // We will check and populate cdDB
                        let partition_name = format!("{}_{}_{}", folder, next_num, next_date)
                            .replace('.', "_")
                            .replace('/', "_");
                            
                        let writer = writers_clone.get(&partition_name).unwrap();
                        
                        // 2. Perform file parse and ingest if cdDB cache partition is empty
                        let mut route_len = 0;
                        if let Some(route) = db_clone.get_route(&partition_name) {
                            let worker = route.register_worker();
                            worker.enter();
                            route_len = route.len(&worker);
                            worker.leave();
                        }
                        
                        if route_len == 0 {
                            // Parse data
                            if let Ok(mut parse) = Parse::new(&path, req_fields.clone(), &Config::new()) {
                                let parsed_cols = parse.data();
                                // We ingest row-by-row into cdDB
                                if !parsed_cols.is_empty() && !parsed_cols[0].is_empty() {
                                    let num_rows = parsed_cols[0].len();
                                    for r in 0..num_rows {
                                        let mut attrs = cdDB::AHashMap::default();
                                        for col_idx in 0..req_fields.len() {
                                            let val = parsed_cols[col_idx].get(r).cloned().unwrap_or_else(|| "null".to_string());
                                            attrs.insert(req_fields[col_idx].clone(), val);
                                        }
                                        
                                        // Include date if empty
                                        if check_date && date_param.is_empty() {
                                            attrs.insert("pathIn".to_string(), next_date.clone());
                                        }
                                        
                                        let _ = writer.send(WriteCommand::Insert {
                                            entity_id: r,
                                            attributes: Attributes::from(attrs),
                                            attributes_int: Attributes::default(),
                                            attributes_blob: Attributes::default(),
                                        });
                                    }
                                }
                            }
                            
                            // Sleep briefly to let background thread complete processing queue
                            std::thread::sleep(std::time::Duration::from_millis(50));
                        }
                        
                        // 3. Query cdDB for the columns using Scan
                        if let Some(route) = db_clone.get_route(&partition_name) {
                            let query = Query::new(&route);
                            
                            let mut write_idx = 0;
                            if check_date && date_param.is_empty() {
                                let mut date_col_data = Vec::new();
                                // Query 'pathIn' for each entity
                                let len = {
                                    let w = route.register_worker();
                                    w.enter();
                                    let l = route.len(&w);
                                    w.leave();
                                    l
                                };
                                date_col_data.reserve(len);
                                for entity_id in 0..len {
                                    let val = query.get_str(entity_id, "pathIn").unwrap_or_else(|| next_date.clone());
                                    date_col_data.push(val);
                                }
                                
                                local_data[write_idx].extend(date_col_data);
                                write_idx += 1;
                            }
                            
                            // Scan other columns
                            for col_name in &req_fields {
                                let mut col_data = Vec::new();
                                route.execute_batch(&[QueryNode::Scan { attr: col_name }], |res| {
                                    if let QueryResult::StrList(list) = res {
                                        col_data.reserve(list.len());
                                        for s in list {
                                            col_data.push(s.to_string());
                                        }
                                    }
                                });
                                
                                // Fallback if Scan returned empty
                                if col_data.is_empty() {
                                    let len = {
                                        let w = route.register_worker();
                                        w.enter();
                                        let l = route.len(&w);
                                        w.leave();
                                        l
                                    };
                                    col_data.reserve(len);
                                    for entity_id in 0..len {
                                        let val = query.get_str(entity_id, col_name).unwrap_or_else(|| "null".to_string());
                                        col_data.push(val);
                                    }
                                }
                                
                                local_data[write_idx].extend(col_data);
                                write_idx += 1;
                            }
                        }
                    }
                }
                
                let mut d = data_share.lock().unwrap();
                for (i, col_data) in local_data.into_iter().enumerate() {
                    d[col_offset + i].extend(col_data);
                }
            });
            
            threads.push(handle);
        }
        
        for t in threads {
            t.join().unwrap();
        }
        
        let out = self.data.lock().unwrap().clone();
        Ok(out)
    }

    pub fn export(&self, path_in: &str, _assert_head_in: bool) -> Result<(), Box<dyn std::error::Error>> {
        let path = if path_in.is_empty() {
            PathBuf::from("downloads").join("export.csv")
        } else {
            PathBuf::from(path_in)
        };
        
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        let data = self.data.lock().unwrap().clone();
        if data.is_empty() {
            println!("No data to export. Run select first.");
            return Ok(());
        }
        
        // Build header row
        let mut headers = Vec::new();
        for i in 0..self.req.len() {
            if self.check_date && self.dates[i].is_empty() {
                headers.push("pathIn".to_string());
            }
            for col in &self.req[i] {
                headers.push(col.clone());
            }
        }
        
        let mut wtr = csv::Writer::from_path(&path)?;
        wtr.write_record(&headers)?;
        
        // Determine number of rows to export
        let max_rows = data.iter().map(|col| col.len()).max().unwrap_or(0);
        
        for r in 0..max_rows {
            let mut row = Vec::new();
            for col in &data {
                let cell_val = col.get(r).cloned().unwrap_or_else(|| "".to_string());
                row.push(cell_val);
            }
            wtr.write_record(&row)?;
        }
        
        wtr.flush()?;
        println!("Exported to {}", path.display());
        Ok(())
    }

    pub fn back_test(&self, name_in: &str) -> Result<String, Box<dyn std::error::Error>> {
        let data = self.data.lock().unwrap().clone();
        let mut tester = BackTest::new(data, name_in)?;
        let out = format!(
            "WinRate: {}\nExpectValue: {}\n{:?}",
            tester.get_win_rate(),
            tester.get_expect_value(),
            tester
        );
        Ok(out)
    }
}
