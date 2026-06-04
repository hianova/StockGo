use std::fs;
use std::path::Path;
use std::process::Command;
use regex::Regex;
use scraper::{Html, Selector};
use serde_json::Value;
use crate::config::Config;
use crate::lib_utils::LibUtils;

pub struct Parse {
    pub hd: Vec<String>,
    pub bd: Vec<String>,
    pub opt: Vec<String>,
    pub req: Vec<usize>,
    pub tag: Vec<Option<Regex>>,
    pub check_tag: bool,
    pub check_opt: bool,
}

impl Parse {
    pub fn new(path_in: &str, req_in: Vec<String>, config: &Config) -> Result<Self, Box<dyn std::error::Error>> {
        let opt_rgx = Regex::new(r"!")?;
        let tag_rgx = Regex::new(r"#")?;
        
        let path = Path::new(path_in);
        let file = fs::read_to_string(path)?;
        
        let mut opt = Vec::new();
        let mut hd = Vec::new();
        let mut bd = Vec::new();
        let mut req = Vec::new();
        let mut tag = Vec::new();
        let mut check_tag = false;
        let mut check_opt = false;
        
        if LibUtils::is_html(&file) {
            let html_str = LibUtils::clean_html(&file);
            let fragment = Html::parse_fragment(&html_str);
            
            // Extract tag text
            let tag_selector = Selector::parse("tag").unwrap();
            let tag_val = fragment.select(&tag_selector).next()
                .map(|el| el.text().collect::<String>())
                .unwrap_or_default();
                
            let head_sel_str = LibUtils::get_tag(&format!("{}/head/HTML", tag_val));
            let body_sel_str = LibUtils::get_tag(&format!("{}/body/HTML", tag_val));
            
            let hd_rows = Self::select_tr(&fragment, &head_sel_str);
            if let Some(first_row) = hd_rows.first() {
                let cell_selector = Selector::parse("td, th").unwrap();
                for cell in first_row.select(&cell_selector) {
                    hd.push(cell.text().collect::<String>().trim().to_string());
                }
            }
            
            let bd_rows = Self::select_tr(&fragment, &body_sel_str);
            let cell_selector = Selector::parse("td, th").unwrap();
            for row in bd_rows {
                for cell in row.select(&cell_selector) {
                    bd.push(cell.text().collect::<String>().trim().to_string());
                }
            }
            
        } else if LibUtils::is_json(&file) {
            let json: Value = serde_json::from_str(&file)?;
            let tag_val = json.get("tag").and_then(|v| v.as_str()).unwrap_or("").to_string();
            
            let head_pointer = LibUtils::get_tag(&format!("{}/head/JSON", tag_val));
            let body_pointer = LibUtils::get_tag(&format!("{}/body/JSON", tag_val));
            
            let head_node = json.pointer(&format!("/{}", head_pointer));
            let body_node = json.pointer(&format!("/{}", body_pointer));
            
            if let Some(hn) = head_node {
                if let Some(arr) = hn.as_array() {
                    for v in arr {
                        hd.push(v.as_str().unwrap_or("").to_string());
                    }
                }
            }
            
            if let Some(bn) = body_node {
                if let Some(arr) = bn.as_array() {
                    for row in arr {
                        if let Some(cells) = row.as_array() {
                            for cell in cells {
                                bd.push(cell.as_str().unwrap_or("").to_string());
                            }
                        }
                    }
                }
            }
            
        } else {
            // Parse CSV
            let mut rdr = csv::ReaderBuilder::new()
                .has_headers(false)
                .from_reader(file.as_bytes());
                
            let mut iter = rdr.records();
            if let Some(result) = iter.next() {
                let record = result?;
                for field in record.iter() {
                    hd.push(field.to_string());
                }
            }
            
            for result in iter {
                let record = result?;
                for field in record.iter() {
                    bd.push(field.to_string());
                }
            }
        }
        
        for req_item in req_in {
            let mut req_tmp = req_item.clone();
            
            // Check opt: e.g. operator!field
            if opt_rgx.is_match(&req_tmp) {
                let parts: Vec<&str> = req_tmp.split('!').collect();
                opt.push(parts[0].to_string());
                req_tmp = parts[1].to_string();
                check_opt = true;
            } else {
                opt.push("".to_string());
            }
            
            // Check tag: e.g. field#regex1#regex2
            if tag_rgx.is_match(&req_tmp) {
                let parts: Vec<String> = req_tmp.split('#').map(|s| s.to_string()).collect();
                req_tmp = parts[0].clone();
                check_tag = true;
                
                let mut rgx_str = parts[1].clone();
                for item in parts.iter().skip(2) {
                    rgx_str = format!("{}|{}", rgx_str, item);
                }
                tag.push(Some(Regex::new(&rgx_str)?));
            } else {
                tag.push(None);
            }
            
            // Find index of field in headers
            if let Some(idx) = hd.iter().position(|h| h == &req_tmp) {
                req.push(idx);
            } else {
                req.push(usize::MAX); // not found
            }
        }
        
        Ok(Self {
            hd,
            bd,
            opt,
            req,
            tag,
            check_tag,
            check_opt,
        })
    }

    fn select_tr<'a>(fragment: &'a Html, selector_str: &str) -> Vec<scraper::ElementRef<'a>> {
        let tr_selector = Selector::parse("tr").unwrap();
        let all_tr: Vec<scraper::ElementRef> = fragment.select(&tr_selector).collect();
        
        let eq_rgx = Regex::new(r"tr:eq\((\d+)\)").unwrap();
        let gt_rgx = Regex::new(r"tr:gt\((\d+)\)").unwrap();
        
        if let Some(cap) = eq_rgx.captures(selector_str) {
            let idx: usize = cap[1].parse().unwrap_or(0);
            if idx < all_tr.len() {
                vec![all_tr[idx]]
            } else {
                Vec::new()
            }
        } else if let Some(cap) = gt_rgx.captures(selector_str) {
            let idx: usize = cap[1].parse().unwrap_or(0);
            if idx + 1 < all_tr.len() {
                all_tr.into_iter().skip(idx + 1).collect()
            } else {
                Vec::new()
            }
        } else {
            // Standard selector fallback
            if let Ok(sel) = Selector::parse(selector_str) {
                fragment.select(&sel).collect()
            } else {
                all_tr
            }
        }
    }

    pub fn data(&mut self) -> Vec<Vec<String>> {
        let _req_size = self.req.len();
        let mut out: Vec<Vec<String>> = vec![Vec::new(); self.req.len()];
        if self.hd.is_empty() {
            return out;
        }
        
        let row_size = self.hd.len();
        let num_rows = self.bd.len() / row_size;
        
        for row in 0..num_rows {
            let mut pass = !self.check_tag;
            
            // Validate against regex tags first without allocating the whole line
            if self.check_tag {
                for (i, req_col_idx) in self.req.iter().enumerate() {
                    if *req_col_idx != usize::MAX && *req_col_idx < row_size {
                        let cell_offset = row * row_size + *req_col_idx;
                        if let Some(cell_val) = self.bd.get(cell_offset) {
                            let val_str = if cell_val.is_empty() { "null" } else { cell_val };
                            if let Some(ref rgx) = self.tag[i] {
                                if rgx.is_match(val_str) {
                                    pass = true;
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            
            if pass {
                for (i, req_col_idx) in self.req.iter().enumerate() {
                    if *req_col_idx != usize::MAX && *req_col_idx < row_size {
                        let cell_offset = row * row_size + *req_col_idx;
                        let cell_val = self.bd.get(cell_offset).cloned().unwrap_or_default();
                        let cell_val = if cell_val.is_empty() { "null".to_string() } else { cell_val };
                        out[i].push(cell_val);
                    } else {
                        out[i].push("null".to_string());
                    }
                }
            }
        }
        
        // Execute opt calculations using bun
        if self.check_opt {
            for i in 0..self.opt.len() {
                if !self.opt[i].is_empty() && !out[i].is_empty() {
                    // Try parsing numbers to pass to Javascript
                    let int_vals: Vec<i64> = out[i].iter()
                        .map(|s| s.parse::<i64>().unwrap_or(0))
                        .collect();
                        
                    if let Ok(json_input) = serde_json::to_string(&int_vals) {
                        // Formula evaluates: x.map(num => formula)
                        // bun -e "const x = JSON.parse(process.argv[1]); const y = x.map(num => formula); console.log(JSON.stringify(y))" json_input
                        let formula = &self.opt[i];
                        let js_code = format!(
                            "const x = JSON.parse(process.argv[1]); const y = x.map(num => {}); console.log(JSON.stringify(y))",
                            formula
                        );
                        
                        let run_res = Command::new("bun")
                            .arg("-e")
                            .arg(&js_code)
                            .arg(&json_input)
                            .output();
                            
                        match run_res {
                            Ok(output) => {
                                if output.status.success() {
                                    let stdout_str = String::from_utf8_lossy(&output.stdout);
                                    if let Ok(new_vals) = serde_json::from_str::<Vec<Value>>(&stdout_str) {
                                        out[i] = new_vals.iter()
                                            .map(|v| match v {
                                                Value::Number(num) => num.to_string(),
                                                Value::String(s) => s.clone(),
                                                _ => v.to_string(),
                                            })
                                            .collect();
                                    }
                                } else {
                                    eprintln!("Bun execution failed: {}", String::from_utf8_lossy(&output.stderr));
                                }
                            }
                            Err(e) => {
                                eprintln!("Failed to invoke bun: {}. Formula not applied.", e);
                            }
                        }
                    }
                }
            }
        }
        
        out
    }
}
