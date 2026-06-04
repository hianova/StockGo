use std::fs;
use std::path::Path;
use regex::Regex;
use scraper::{Html, Selector};
use serde_json::Value;

pub struct LibUtils;

impl LibUtils {
    pub fn url_to_name(url_in: &str) -> String {
        // Remove file extension, @num, @date, query params, etc.
        let re = Regex::new(r"(\.\w+|@num|@date|\?.+)").unwrap();
        let cleaned = re.replace_all(url_in, "");
        let parts: Vec<&str> = cleaned.split('/').collect();
        if parts.len() >= 2 {
            format!("{}_{}", parts[parts.len() - 2], parts[parts.len() - 1])
        } else if !parts.is_empty() {
            parts[parts.len() - 1].to_string()
        } else {
            url_in.to_string()
        }
    }

    pub fn is_json(json_in: &str) -> bool {
        serde_json::from_str::<Value>(json_in).is_ok()
    }

    pub fn is_html(html_in: &str) -> bool {
        // Simple HTML check
        html_in.trim().starts_with('<') || html_in.contains("<html>") || html_in.contains("<tr")
    }

    pub fn clean_html(html_content: &str) -> String {
        let fragment = Html::parse_fragment(html_content);
        let tr_selector = Selector::parse("tr").unwrap();
        let table_selector = Selector::parse("table").unwrap();
        let tag_selector = Selector::parse("tag").unwrap();
        
        let mut rows_html = String::new();
        
        for tr in fragment.select(&tr_selector) {
            // Check if tr contains a table, or is nested in a table that isn't the parent
            // Jsoup: nextRow.children().select("table").isEmpty() & nextRow.parent().select("table").isEmpty()
            let has_nested_table = tr.select(&table_selector).next().is_some();
            if !has_nested_table {
                let mut row_content = String::new();
                for child in tr.children() {
                    if let Some(el) = child.value().as_element() {
                        if el.name() == "td" || el.name() == "th" {
                            // Extract colspan/rowspan if any
                            let colspan = el.attr("colspan").and_then(|s| s.parse::<usize>().ok()).unwrap_or(1);
                            let rowspan = el.attr("rowspan").and_then(|s| s.parse::<usize>().ok()).unwrap_or(1);
                            
                            let mut text = child.first_child().map(|c| c.value().as_text().map(|t| t.to_string()).unwrap_or_default()).unwrap_or_default();
                            text = text.replace("<br>", "").replace("<br/>", "").trim().to_string();
                            
                            // Recreate cell
                            let mut attrs = String::new();
                            if rowspan > 1 {
                                attrs.push_str(&format!(" rowspan='{}'", rowspan));
                            }
                            // Handle colspan replication
                            if colspan > 1 {
                                for _ in 0..colspan {
                                    row_content.push_str(&format!("<td{}>{}</td>", attrs, text));
                                }
                            } else {
                                row_content.push_str(&format!("<td{}>{}</td>", attrs, text));
                            }
                        }
                    }
                }
                rows_html.push_str(&format!("<tr>{}</tr>", row_content));
            }
        }
        
        let tag_text = fragment.select(&tag_selector).next()
            .map(|el| el.html())
            .unwrap_or_default();
            
        format!("<html>{}</html>{}", rows_html, tag_text)
    }

    pub fn get_tag(tag_in: &str) -> String {
        let file_path = Path::new("parse_rule.json");
        if file_path.exists() {
            if let Ok(content) = fs::read_to_string(file_path) {
                if let Ok(json) = serde_json::from_str::<Value>(&content) {
                    // Extract path /a/b/c
                    let mut current = &json;
                    let parts: Vec<&str> = tag_in.split('/').collect();
                    let mut found = true;
                    for part in &parts {
                        if let Some(val) = current.get(part) {
                            current = val;
                        } else {
                            found = false;
                            break;
                        }
                    }
                    if found {
                        if let Some(s) = current.as_str() {
                            return s.to_string();
                        }
                    }
                }
            }
        }
        
        // Fallbacks
        let tmp: Vec<&str> = tag_in.split('/').collect();
        if tmp.len() >= 3 {
            if tmp[1] == "codec" && tmp[2] == "UTF-8" {
                return "UTF-8".to_string();
            } else if tmp[1] == "head" {
                if tmp[2] == "JSON" {
                    return "fields".to_string();
                } else if tmp[2] == "HTML" {
                    return "tr:eq(0)".to_string();
                }
            } else if tmp[1] == "body" {
                if tmp[2] == "JSON" {
                    return "data".to_string();
                } else if tmp[2] == "HTML" {
                    return "tr:gt(0)".to_string();
                }
            }
        }
        
        "".to_string()
    }
}
