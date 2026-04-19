use crate::book::models::Book;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub default_theme: String,
    pub text_size: usize,
    #[serde(default = "default_alignment")]
    pub text_alignment: String,
    #[serde(default = "default_startup_view")]
    pub startup_view: String,
    #[serde(default = "default_case_sensitive")]
    pub case_sensitive_search: bool,
    #[serde(default = "default_show_status_bar")]
    pub show_status_bar: bool,
    #[serde(default = "default_target_wpm")]
    pub target_wpm: usize,
}

fn default_alignment() -> String {
    "left".to_string()
}
fn default_startup_view() -> String {
    "library".to_string()
}
fn default_case_sensitive() -> bool {
    false
}
fn default_show_status_bar() -> bool {
    true
}
fn default_target_wpm() -> usize {
    250
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            default_theme: "dark".to_string(),
            text_size: 14,
            text_alignment: default_alignment(),
            startup_view: default_startup_view(),
            case_sensitive_search: default_case_sensitive(),
            show_status_bar: default_show_status_bar(),
            target_wpm: default_target_wpm(),
        }
    }
}

pub fn get_data_dir() -> Option<PathBuf> {
    if let Some(proj_dirs) = ProjectDirs::from("com", "termtome", "app") {
        Some(proj_dirs.data_dir().to_path_buf())
    } else {
        None
    }
}

pub fn save_book_state(book: &Book) -> Result<(), String> {
    if let Some(mut data_dir) = get_data_dir() {
        data_dir.push("states");
        fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;

        let file_name = format!("{}.json", book.id);
        data_dir.push(file_name);

        let json = serde_json::to_string_pretty(book).map_err(|e| e.to_string())?;
        fs::write(data_dir, json).map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("Could not determine data directory".to_string())
    }
}

pub fn load_book_state(id: &str) -> Option<Book> {
    if let Some(mut data_dir) = get_data_dir() {
        data_dir.push("states");
        let file_name = format!("{}.json", id);
        data_dir.push(file_name);

        if let Ok(json) = fs::read_to_string(data_dir) {
            return serde_json::from_str(&json).ok();
        }
    }
    None
}
