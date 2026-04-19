use crate::book::models::Book;
use crate::book::online::OnlineBook;
use crossbeam_channel::{Receiver, Sender};
use std::time::Instant;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ActivePane {
    Library,
    Reader,
    Downloader,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Dark,
    Light,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SortMode {
    Name,
    Progress,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Normal,
    Command,
    Search,
    OnlineSearch,
    LibrarySearch,
    Toc,
    Preferences,
}

pub enum BackgroundMessage {
    OnlineSearchResults(Vec<OnlineBook>),
    DownloadComplete(Result<Book, String>),
    Error(String),
}

pub struct App {
    pub should_quit: bool,
    pub mode: AppMode,
    pub active_pane: ActivePane,

    // Library state
    pub books: Vec<Book>,
    pub selected_book_index: Option<usize>,
    pub current_book: Option<Book>,
    pub library_filter: String,
    pub library_sort: SortMode,

    // Downloader state
    pub online_books: Vec<OnlineBook>,
    pub selected_online_index: Option<usize>,
    pub is_loading: bool,

    pub theme: Theme,
    pub margin_width: u16, // using margin width for font size replacement
    pub command_buffer: String,
    pub command_suggestions: Vec<String>,
    pub suggestion_index: Option<usize>,
    pub search_query: String,
    pub error_message: Option<String>,
    pub toc_selected_index: usize,
    pub preferences_selected_index: usize,

    pub wpm: usize,
    pub last_tick: Instant,

    pub text_alignment: String,
    pub startup_view: String,
    pub case_sensitive_search: bool,
    pub show_status_bar: bool,

    pub tx: Sender<BackgroundMessage>,
    pub rx: Receiver<BackgroundMessage>,
    pub show_help: bool,
    pub help_scroll: u16,
}

impl App {
    pub fn new() -> Self {
        let (tx, rx) = crossbeam_channel::unbounded();
        Self {
            should_quit: false,
            mode: AppMode::Normal,
            active_pane: ActivePane::Library,
            books: Vec::new(),
            selected_book_index: None,
            current_book: None,
            library_filter: String::new(),
            library_sort: SortMode::Name,
            online_books: Vec::new(),
            selected_online_index: None,
            is_loading: false,
            theme: Theme::Dark,
            margin_width: 0,
            command_buffer: String::new(),
            command_suggestions: Vec::new(),
            suggestion_index: None,
            search_query: String::new(),
            error_message: None,
            toc_selected_index: 0,
            preferences_selected_index: 0,
            wpm: 250,
            last_tick: Instant::now(),
            text_alignment: "left".to_string(),
            startup_view: "library".to_string(),
            case_sensitive_search: false,
            show_status_bar: true,
            tx,
            rx,
            show_help: false,
            help_scroll: 0,
        }
    }

    pub fn tick(&mut self) -> bool {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_tick).as_secs();
        if elapsed >= 1 {
            if matches!(self.active_pane, ActivePane::Reader) {
                if let Some(ref mut book) = self.current_book {
                    book.time_spent_secs += elapsed;
                }
            }
            self.last_tick = now;
            true
        } else {
            false
        }
    }

    pub fn filtered_books(&self) -> Vec<&Book> {
        let mut filtered: Vec<&Book> = self
            .books
            .iter()
            .filter(|b| {
                if self.library_filter.is_empty() {
                    true
                } else {
                    b.title
                        .to_lowercase()
                        .contains(&self.library_filter.to_lowercase())
                }
            })
            .collect();

        match self.library_sort {
            SortMode::Name => filtered.sort_by(|a, b| a.title.cmp(&b.title)),
            SortMode::Progress => {
                filtered.sort_by(|a, b| b.current_chapter.cmp(&a.current_chapter));
            }
        }
        filtered
    }

    pub fn next_book(&mut self) {
        let filtered = self.filtered_books();
        if filtered.is_empty() {
            self.selected_book_index = None;
        } else {
            let i = match self.selected_book_index {
                Some(i) => {
                    if i >= filtered.len() - 1 {
                        0
                    } else {
                        i + 1
                    }
                }
                None => 0,
            };
            self.selected_book_index = Some(i);
        }
    }

    pub fn previous_book(&mut self) {
        let filtered = self.filtered_books();
        if filtered.is_empty() {
            self.selected_book_index = None;
        } else {
            let i = match self.selected_book_index {
                Some(i) => {
                    if i == 0 {
                        filtered.len() - 1
                    } else {
                        i - 1
                    }
                }
                None => 0,
            };
            self.selected_book_index = Some(i);
        }
    }

    pub fn open_selected_book(&mut self) {
        let filtered = self.filtered_books();
        if let Some(i) = self.selected_book_index {
            if let Some(book) = filtered.get(i) {
                // Now we need to load the full content
                match crate::book::loader::load_book(&book.path) {
                    Ok(mut full_book) => {
                        if let Some(saved) = crate::storage::bookmarks::load_book_state(&book.id) {
                            full_book.current_chapter = saved.current_chapter;
                            full_book.current_position = saved.current_position;
                            full_book.bookmarks = saved.bookmarks;
                            full_book.time_spent_secs = saved.time_spent_secs;
                            full_book.last_read = saved.last_read;
                        }
                        // Copy the category that was assigned
                        full_book.category = book.category.clone();
                        self.current_book = Some(full_book);
                        self.active_pane = ActivePane::Reader;
                    }
                    Err(e) => {
                        self.error_message = Some(format!("Failed to open book: {}", e));
                    }
                }
            }
        }
    }

    pub fn next_online_book(&mut self) {
        if self.online_books.is_empty() {
            self.selected_online_index = None;
        } else {
            let i = match self.selected_online_index {
                Some(i) => {
                    if i >= self.online_books.len() - 1 {
                        0
                    } else {
                        i + 1
                    }
                }
                None => 0,
            };
            self.selected_online_index = Some(i);
        }
    }

    pub fn previous_online_book(&mut self) {
        if self.online_books.is_empty() {
            self.selected_online_index = None;
        } else {
            let i = match self.selected_online_index {
                Some(i) => {
                    if i == 0 {
                        self.online_books.len() - 1
                    } else {
                        i - 1
                    }
                }
                None => 0,
            };
            self.selected_online_index = Some(i);
        }
    }

    pub fn toggle_pane(&mut self) {
        if matches!(self.active_pane, ActivePane::Reader) {
            if let Some(ref mut current_book) = self.current_book {
                // Sync current book state to app.books
                for b in &mut self.books {
                    if b.id == current_book.id {
                        b.current_chapter = current_book.current_chapter;
                        b.current_position = current_book.current_position;
                        b.bookmarks = current_book.bookmarks.clone();
                        b.time_spent_secs = current_book.time_spent_secs;
                        b.last_read = current_book.last_read.clone();
                        break;
                    }
                }
                // Also save it
                let _ = crate::storage::bookmarks::save_book_state(current_book);
            }
        }

        self.active_pane = match self.active_pane {
            ActivePane::Library => ActivePane::Reader,
            ActivePane::Reader => ActivePane::Library,
            ActivePane::Downloader => ActivePane::Library,
        };
    }

    pub fn toggle_theme(&mut self) {
        self.theme = match self.theme {
            Theme::Dark => Theme::Light,
            Theme::Light => Theme::Dark,
        };
    }

    pub fn update_suggestions(&mut self) {
        self.command_suggestions.clear();
        self.suggestion_index = None;
        let parts: Vec<&str> = self.command_buffer.splitn(2, ' ').collect();

        if parts.is_empty() {
            return;
        }
        let cmd = parts[0].to_lowercase();

        if cmd == "delete" || cmd == "move" {
            let arg = if parts.len() > 1 {
                parts[1].to_lowercase()
            } else {
                String::new()
            };

            for book in &self.books {
                let cat = book.category.as_deref().unwrap_or("uncategorized");
                let full_path = format!("{}/{}", cat, book.title);

                if full_path.to_lowercase().contains(&arg) {
                    if cmd == "delete" {
                        self.command_suggestions
                            .push(format!("delete \"{}\"", full_path));
                    } else if cmd == "move" {
                        self.command_suggestions
                            .push(format!("move \"{}\" <new_category>", full_path));
                    }
                }
            }
        }
    }

    pub fn apply_suggestion(&mut self) {
        if let Some(idx) = self.suggestion_index {
            if let Some(sug) = self.command_suggestions.get(idx) {
                if sug.starts_with("move") {
                    self.command_buffer = sug.replace(" <new_category>", " ");
                } else {
                    self.command_buffer = sug.clone();
                }
                self.command_suggestions.clear();
                self.suggestion_index = None;
            }
        } else if !self.command_suggestions.is_empty() {
            let sug = &self.command_suggestions[0];
            if sug.starts_with("move") {
                self.command_buffer = sug.replace(" <new_category>", " ");
            } else {
                self.command_buffer = sug.clone();
            }
            self.command_suggestions.clear();
            self.suggestion_index = None;
        }
    }

    pub fn cycle_suggestion(&mut self, forward: bool) {
        if self.command_suggestions.is_empty() {
            return;
        }

        let len = self.command_suggestions.len();
        self.suggestion_index = Some(match self.suggestion_index {
            Some(i) => {
                if forward {
                    (i + 1) % len
                } else {
                    if i == 0 {
                        len - 1
                    } else {
                        i - 1
                    }
                }
            }
            None => {
                if forward {
                    0
                } else {
                    len - 1
                }
            }
        });
    }

    fn parse_book_arg<'a>(&self, buf: &'a str, cmd: &str) -> Option<(&'a str, String)> {
        let rest = buf.trim_start_matches(cmd).trim_start();
        if rest.starts_with('"') {
            if let Some(end_idx) = rest[1..].find('"') {
                let path = &rest[1..end_idx + 1];
                let remainder = &rest[end_idx + 2..].trim();
                return Some((remainder, path.to_string()));
            }
        }
        None
    }

    pub fn execute_command(&mut self) {
        let cmd = self.command_buffer.trim();
        let parts: Vec<&str> = cmd.split_whitespace().collect();

        if parts.is_empty() {
            self.mode = AppMode::Normal;
            self.command_buffer.clear();
            self.command_suggestions.clear();
            return;
        }

        let cmd_name = parts[0].to_lowercase();
        match cmd_name.as_str() {
            "q" | "quit" | "exit" => {
                self.should_quit = true;
            }
            "dl" | "downloader" => {
                self.active_pane = ActivePane::Downloader;
            }
            "import" => {
                if parts.len() > 1 {
                    let path_str = cmd.split_once(' ').unwrap().1;
                    let src_path = std::path::PathBuf::from(path_str);
                    if src_path.exists() && src_path.is_file() {
                        if let Some(ext) = src_path.extension() {
                            if ext == "epub" || ext == "txt" {
                                let books_dir = std::env::current_dir().unwrap().join("books");
                                let file_name = src_path.file_name().unwrap();
                                let dest_path = books_dir.join(file_name);

                                std::fs::create_dir_all(&books_dir).unwrap();
                                match std::fs::copy(&src_path, &dest_path) {
                                    Ok(_) => match crate::book::loader::load_book(&dest_path) {
                                        Ok(book) => {
                                            self.books.push(book.clone());
                                            self.current_book = Some(book);
                                            self.active_pane = ActivePane::Reader;
                                            self.error_message =
                                                Some("Book imported successfully!".to_string());
                                        }
                                        Err(e) => {
                                            self.error_message =
                                                Some(format!("Failed to parse book: {}", e));
                                            let _ = std::fs::remove_file(&dest_path);
                                        }
                                    },
                                    Err(e) => {
                                        self.error_message =
                                            Some(format!("Failed to copy file: {}", e))
                                    }
                                }
                            } else {
                                self.error_message =
                                    Some("Only .epub and .txt are supported".to_string());
                            }
                        }
                    } else {
                        self.error_message = Some("File not found".to_string());
                    }
                } else {
                    self.error_message = Some("Usage: :import /path/to/book.epub".to_string());
                }
            }
            "create" | "mkdir" | "category" => {
                if parts.len() > 1 {
                    let new_cat = parts[1];
                    let new_dir = std::env::current_dir().unwrap().join("books").join(new_cat);
                    match std::fs::create_dir_all(&new_dir) {
                        Ok(_) => {
                            self.error_message = Some(format!("Created category: {}", new_cat))
                        }
                        Err(e) => {
                            self.error_message = Some(format!("Failed to create category: {}", e))
                        }
                    }
                } else {
                    self.error_message = Some("Usage: :create <category_name>".to_string());
                }
            }
            "delete" => {
                let target_book =
                    if let Some((_, path)) = self.parse_book_arg(&self.command_buffer, "delete") {
                        self.books
                            .iter()
                            .find(|b| {
                                let cat = b.category.as_deref().unwrap_or("uncategorized");
                                format!("{}/{}", cat, b.title).to_lowercase() == path.to_lowercase()
                            })
                            .cloned()
                    } else if let Some(i) = self.selected_book_index {
                        self.filtered_books().get(i).map(|b| (*b).clone())
                    } else {
                        None
                    };

                if let Some(book) = target_book {
                    let id_to_delete = book.id.clone();
                    let path_to_delete = book.path.clone();

                    if std::fs::remove_file(&path_to_delete).is_ok() {
                        self.books.retain(|b| b.id != id_to_delete);
                        if let Some(ref cb) = self.current_book {
                            if cb.id == id_to_delete {
                                self.current_book = None;
                                self.active_pane = ActivePane::Library;
                            }
                        }
                        self.error_message = Some("Book deleted".to_string());
                        self.selected_book_index =
                            if self.books.is_empty() { None } else { Some(0) };
                    } else {
                        self.error_message = Some("Failed to delete file".to_string());
                    }
                } else {
                    self.error_message = Some("No book selected or found".to_string());
                }
            }
            "move" => {
                let parsed = self.parse_book_arg(&self.command_buffer, "move");
                let (target_book, new_cat) = if let Some((remainder, path)) = parsed {
                    let book = self
                        .books
                        .iter()
                        .find(|b| {
                            let cat = b.category.as_deref().unwrap_or("uncategorized");
                            format!("{}/{}", cat, b.title).to_lowercase() == path.to_lowercase()
                        })
                        .cloned();
                    (book, remainder.to_string())
                } else if let Some(i) = self.selected_book_index {
                    let book = self.filtered_books().get(i).map(|b| (*b).clone());
                    let new_cat = parts.get(1).unwrap_or(&"").to_string();
                    (book, new_cat)
                } else {
                    (None, String::new())
                };

                if let Some(book) = target_book {
                    if !new_cat.is_empty() {
                        let id_to_move = book.id.clone();
                        let old_path = book.path.clone();
                        let books_dir = std::env::current_dir().unwrap().join("books");
                        let new_dir = books_dir.join(&new_cat);
                        let new_path = new_dir.join(old_path.file_name().unwrap());

                        std::fs::create_dir_all(&new_dir).unwrap();
                        if std::fs::rename(&old_path, &new_path).is_ok() {
                            for b in self.books.iter_mut() {
                                if b.id == id_to_move {
                                    b.path = new_path.clone();
                                    b.category = Some(new_cat.clone());
                                }
                            }
                            if let Some(ref mut cb) = self.current_book {
                                if cb.id == id_to_move {
                                    cb.path = new_path;
                                    cb.category = Some(new_cat.clone());
                                }
                            }
                            self.error_message =
                                Some(format!("Book moved to category: {}", new_cat));
                        } else {
                            self.error_message = Some("Failed to move file".to_string());
                        }
                    } else {
                        self.error_message =
                            Some("Usage: :move [\"category/title\"] <category>".to_string());
                    }
                } else {
                    self.error_message = Some("No book selected or found".to_string());
                }
            }
            "theme" => {
                if parts.len() > 1 {
                    match parts[1] {
                        "light" => self.theme = Theme::Light,
                        "dark" => self.theme = Theme::Dark,
                        _ => {
                            self.error_message =
                                Some("Invalid theme. Use 'light' or 'dark'".to_string())
                        }
                    }
                } else {
                    self.toggle_theme();
                }
            }
            _ => {
                self.error_message = Some(format!("Unknown command: {}", parts[0]));
            }
        }

        self.mode = AppMode::Normal;
        self.command_buffer.clear();
    }

    pub fn execute_search(&mut self) {
        if matches!(self.mode, AppMode::OnlineSearch) {
            self.is_loading = true;
            let query = self.command_buffer.clone();
            let tx = self.tx.clone();
            std::thread::spawn(move || {
                let url = format!(
                    "https://gutendex.com/books/?search={}",
                    urlencoding::encode(&query)
                );
                match reqwest::blocking::get(&url) {
                    Ok(res) => {
                        if let Ok(data) = res.json::<crate::book::online::GutendexResponse>() {
                            let _ = tx.send(BackgroundMessage::OnlineSearchResults(data.results));
                        } else {
                            let _ = tx.send(BackgroundMessage::Error(
                                "Failed to parse API response".to_string(),
                            ));
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(BackgroundMessage::Error(format!("Network error: {}", e)));
                    }
                }
            });
        } else {
            self.search_query = self.command_buffer.clone();
        }
        self.mode = AppMode::Normal;
        self.command_buffer.clear();
    }

    pub fn find_next_search_result(&mut self, forward: bool) {
        if self.search_query.is_empty() {
            return;
        }

        let query = if self.case_sensitive_search {
            self.search_query.clone()
        } else {
            self.search_query.to_lowercase()
        };

        if let Some(ref mut book) = self.current_book {
            if let Some(chapter) = book.chapters.get(book.current_chapter) {
                let lines: Vec<&str> = chapter.content.lines().collect();
                if lines.is_empty() {
                    return;
                }

                let start_idx = book.current_position;
                let mut found_idx = None;

                let check_match = |line: &str| -> bool {
                    if self.case_sensitive_search {
                        line.contains(&query)
                    } else {
                        line.to_lowercase().contains(&query)
                    }
                };

                if forward {
                    for (i, line) in lines.iter().enumerate().skip(start_idx + 1) {
                        if check_match(line) {
                            found_idx = Some(i);
                            break;
                        }
                    }
                    if found_idx.is_none() {
                        // Wrap around
                        for (i, line) in lines.iter().enumerate().take(start_idx + 1) {
                            if check_match(line) {
                                found_idx = Some(i);
                                break;
                            }
                        }
                    }
                } else {
                    for (i, line) in lines.iter().enumerate().take(start_idx).rev() {
                        if check_match(line) {
                            found_idx = Some(i);
                            break;
                        }
                    }
                    if found_idx.is_none() {
                        // Wrap around
                        for (i, line) in lines.iter().enumerate().skip(start_idx).rev() {
                            if check_match(line) {
                                found_idx = Some(i);
                                break;
                            }
                        }
                    }
                }

                if let Some(idx) = found_idx {
                    book.current_position = idx;
                    self.error_message = None; // clear error if found
                } else {
                    self.error_message = Some("No matches found in this chapter".to_string());
                }
            }
        }
    }

    pub fn calculate_reading_stats(&self) -> Option<(usize, usize)> {
        let book = self.current_book.as_ref()?;
        let current_ch_idx = book.current_chapter;

        let mut words_left_chapter = 0;
        if let Some(chapter) = book.chapters.get(current_ch_idx) {
            // Very fast approximate word count (assuming ~6 chars per word including spaces)
            let bytes_passed: usize = chapter
                .content
                .lines()
                .take(book.current_position)
                .map(|l| l.len())
                .sum();
            let total_bytes = chapter.content.len();
            let remaining_bytes = total_bytes.saturating_sub(bytes_passed);
            words_left_chapter = remaining_bytes / 6;
        }

        let mut words_left_book = words_left_chapter;
        for ch in book.chapters.iter().skip(current_ch_idx + 1) {
            words_left_book += ch.content.len() / 6;
        }

        let wpm = self.wpm.max(1);
        let mins_chapter = (words_left_chapter as f64 / wpm as f64).ceil() as usize;
        let mins_book = (words_left_book as f64 / wpm as f64).ceil() as usize;

        Some((mins_chapter, mins_book))
    }
}
