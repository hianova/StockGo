use std::fs;
use std::path::Path;
use std::process::Command;
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Serialize)]
pub struct BackTest {
    pub data: Vec<Vec<String>>,
    pub mark: Vec<usize>,
    pub odd_p: f64,
    pub odd_n: f64,
    pub point_p: f64,
    pub point_n: f64,
}

impl BackTest {
    pub fn new(data_in: Vec<Vec<String>>, name_in: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let strategy_path = format!("strategy/{}.js", name_in);
        if !Path::new(&strategy_path).exists() {
            return Err(format!("Strategy file {} not found", strategy_path).into());
        }
        
        // Write a temporary JavaScript wrapper file inside the scratch directory
        // or execution folder to execute the strategy using bun.
        let wrapper_code = r#"
const fs = require('fs');
const in_val = process.argv[2];
const data_val = JSON.parse(process.argv[3]);
const strategy_path = process.argv[4];

// Global scope bindings for the script
global.in = in_val;
global.data = data_val;
global.out = [];

const strategy_code = fs.readFileSync(strategy_path, 'utf8');
try {
    eval(strategy_code);
    console.log(JSON.stringify(global.out));
} catch (e) {
    console.error("Error executing strategy:", e.message);
    process.exit(1);
}
"#;

        let wrapper_path = "downloads/backtest_wrapper.js";
        if let Some(parent) = Path::new(wrapper_path).parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(wrapper_path, wrapper_code)?;
        
        // Serialize input dataset to pass to bun
        let serialized_data = serde_json::to_string(&data_in)?;
        
        let output = Command::new("bun")
            .arg(wrapper_path)
            .arg(name_in)
            .arg(&serialized_data)
            .arg(&strategy_path)
            .output()?;
            
        let mut mark: Vec<usize> = Vec::new();
        if output.status.success() {
            let stdout_str = String::from_utf8_lossy(&output.stdout);
            if let Ok(parsed_mark) = serde_json::from_str::<Vec<usize>>(&stdout_str) {
                mark = parsed_mark;
            }
        } else {
            let stderr_str = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Bun execution failed: {}", stderr_str).into());
        }
        
        let mut odd_p = 0.0;
        let mut odd_n = 0.0;
        let mut point_p = 0.0;
        let mut point_n = 0.0;
        
        if !data_in.is_empty() && mark.len() >= 2 {
            let mut i = 0;
            while i + 1 < mark.len() {
                let start_idx = mark[i];
                let end_idx = mark[i + 1];
                
                if start_idx < data_in[0].len() && end_idx < data_in[0].len() {
                    let start_val: f64 = data_in[0][start_idx].parse().unwrap_or(0.0);
                    let end_val: f64 = data_in[0][end_idx].parse().unwrap_or(0.0);
                    let sum = end_val - start_val;
                    
                    if sum > 0.0 {
                        odd_p += 1.0;
                        point_p += sum;
                    } else {
                        odd_n += 1.0;
                        point_n += sum;
                    }
                }
                i += 2;
            }
        }
        
        // Cleanup wrapper script
        let _ = fs::remove_file(wrapper_path);
        
        Ok(Self {
            data: data_in,
            mark,
            odd_p,
            odd_n,
            point_p,
            point_n,
        })
    }

    pub fn get_win_rate(&self) -> String {
        let total = self.odd_p + self.odd_n;
        if total > 0.0 {
            format!("{:.2}%", (self.odd_p / total) * 100.0)
        } else {
            "0.00%".to_string()
        }
    }

    pub fn get_expect_value(&self) -> String {
        let total = self.odd_p + self.odd_n;
        if total > 0.0 {
            let exp = (self.point_p * self.odd_p / total) + (self.point_n * self.odd_n / total);
            format!("{:.2}", exp)
        } else {
            "0.00".to_string()
        }
    }
}
