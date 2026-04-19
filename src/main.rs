pub mod app;
pub mod book;
pub mod storage;
pub mod ui;

use crate::storage::bookmarks::{get_data_dir, AppConfig};
use app::{ActivePane, App, AppMode, Theme};
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers, MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    Frame, Terminal,
};
use std::{error::Error, io};

fn load_config() -> Option<AppConfig> {
    if let Some(mut data_dir) = get_data_dir() {
        data_dir.push("config.json");
        if let Ok(json) = std::fs::read_to_string(&data_dir) {
            return serde_json::from_str(&json).ok();
        }
    }
    None
}

fn save_config(app: &App) {
    if let Some(mut data_dir) = get_data_dir() {
        std::fs::create_dir_all(&data_dir).ok();
        data_dir.push("config.json");

        let config = AppConfig {
            default_theme: match app.theme {
                Theme::Light => "light".to_string(),
                Theme::Dark => "dark".to_string(),
            },
            text_size: app.margin_width as usize,
            text_alignment: app.text_alignment.clone(),
            startup_view: app.startup_view.clone(),
            case_sensitive_search: app.case_sensitive_search,
            show_status_bar: app.show_status_bar,
            target_wpm: app.wpm,
        };
        if let Ok(json) = serde_json::to_string_pretty(&config) {
            std::fs::write(&data_dir, json).ok();
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let mut app = App::new();

    if let Some(config) = load_config() {
        app.theme = if config.default_theme.to_lowercase() == "light" {
            Theme::Light
        } else {
            Theme::Dark
        };
        app.margin_width = config.text_size as u16;
        app.text_alignment = config.text_alignment;
        app.startup_view = config.startup_view;
        app.case_sensitive_search = config.case_sensitive_search;
        app.show_status_bar = config.show_status_bar;
        app.wpm = config.target_wpm;
    }

    let books_dir = std::env::current_dir()?.join("books");
    if !books_dir.exists() {
        std::fs::create_dir_all(&books_dir)?;

        let guide_path = books_dir.join("Starter Guide.txt");
        let guide_content = "Welcome to Terminal Book Reader!\n\n\
To get started, you can place EPUB or TXT files in the 'books' directory.\n\
Alternatively, press Tab to switch to the Downloader pane and search Project Gutenberg.\n\n\
Shortcuts:\n\
- Tab: Switch between panes (Library/Downloader and Reader)\n\
- j/k or Down/Up: Navigate lists and scroll text\n\
- Enter: Open a book or download it\n\
- /: Search library or online store\n\
- : (colon): Command mode\n\
- Ctrl+h: View all shortcuts\n\
- q: Quit application\n";
        let _ = std::fs::write(&guide_path, guide_content);
    }

    let mut to_visit = vec![books_dir.clone()];
    while let Some(dir) = to_visit.pop() {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    to_visit.push(path);
                } else if path.is_file() {
                    if let Some(ext) = path.extension() {
                        if ext == "epub" || ext == "txt" {
                            if let Ok(mut book) = book::loader::load_book_metadata(&path) {
                                if let Some(parent) = path.parent() {
                                    if parent != books_dir {
                                        if let Some(name) = parent.file_name() {
                                            book.category = name.to_str().map(|s| s.to_string());
                                        }
                                    }
                                }
                                if let Some(saved) =
                                    crate::storage::bookmarks::load_book_state(&book.id)
                                {
                                    book.current_chapter = saved.current_chapter;
                                    book.current_position = saved.current_position;
                                    book.bookmarks = saved.bookmarks;
                                    book.time_spent_secs = saved.time_spent_secs;
                                    book.last_read = saved.last_read;
                                }
                                app.books.push(book);
                            }
                        }
                    }
                }
            }
        }
    }
    if !app.books.is_empty() {
        if app.startup_view == "last_read" {
            let filtered = app.filtered_books();
            let mut latest_idx = 0;
            let mut latest_time = String::new();
            for (i, b) in filtered.iter().enumerate() {
                if b.last_read > latest_time {
                    latest_time = b.last_read.clone();
                    latest_idx = i;
                }
            }
            app.selected_book_index = Some(latest_idx);
            app.open_selected_book();
        } else {
            app.selected_book_index = Some(0);
        }
    }

    let res = run_app(&mut terminal, &mut app);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> io::Result<()> {
    let mut should_draw = true;
    loop {
        if app.tick() {
            should_draw = true;
        }

        if should_draw {
            terminal.draw(|f| ui(f, app))?;
            should_draw = false;
        }

        while let Ok(msg) = app.rx.try_recv() {
            should_draw = true;
            match msg {
                app::BackgroundMessage::OnlineSearchResults(res) => {
                    app.online_books = res;
                    if app.online_books.is_empty() {
                        app.selected_online_index = None;
                        app.error_message =
                            Some("No books found matching your search.".to_string());
                    } else {
                        app.selected_online_index = Some(0);
                        app.error_message =
                            Some(format!("Found {} books.", app.online_books.len()));
                    }
                    app.is_loading = false;
                }
                app::BackgroundMessage::DownloadComplete(Ok(book)) => {
                    app.books.push(book.clone());
                    app.current_book = Some(book);
                    app.active_pane = ActivePane::Reader;
                    app.is_loading = false;
                    app.error_message = Some("Download complete!".to_string());
                }
                app::BackgroundMessage::DownloadComplete(Err(e)) => {
                    app.error_message = Some(format!("Download failed: {}", e));
                    app.is_loading = false;
                }
                app::BackgroundMessage::Error(e) => {
                    app.error_message = Some(e);
                    app.is_loading = false;
                }
            }
        }

        if event::poll(std::time::Duration::from_millis(50))? {
            should_draw = true;
            if let Event::Key(key) = event::read()? {
                if (key.code == KeyCode::Char('h') && key.modifiers.contains(KeyModifiers::CONTROL))
                    || key.code == KeyCode::Char('?')
                {
                    app.show_help = !app.show_help;
                    continue;
                }

                if app.show_help {
                    if key.code == KeyCode::Esc
                        || key.code == KeyCode::Char('q')
                        || key.code == KeyCode::Char('?')
                        || (key.code == KeyCode::Char('h')
                            && key.modifiers.contains(KeyModifiers::CONTROL))
                    {
                        app.show_help = false;
                    }
                    continue;
                }

                match app.mode {
                    AppMode::Normal => match key.code {
                        KeyCode::Char('P') => {
                            app.mode = AppMode::Preferences;
                            app.preferences_selected_index = 0;
                        }
                        KeyCode::Char('q') => {
                            if let Some(book) = &app.current_book {
                                let _ = storage::bookmarks::save_book_state(book);
                            }
                            app.should_quit = true;
                        }
                        KeyCode::Tab => app.toggle_pane(),
                        KeyCode::Char(':') => {
                            app.mode = AppMode::Command;
                            app.command_buffer.clear();
                            app.error_message = None;
                        }
                        KeyCode::Char('/') => {
                            app.mode = match app.active_pane {
                                ActivePane::Downloader => AppMode::OnlineSearch,
                                ActivePane::Library => AppMode::LibrarySearch,
                                _ => AppMode::Search,
                            };
                            app.command_buffer.clear();
                            app.error_message = None;
                        }
                        KeyCode::Char('s') => {
                            if matches!(app.active_pane, ActivePane::Library) {
                                app.library_sort = match app.library_sort {
                                    app::SortMode::Name => app::SortMode::Progress,
                                    app::SortMode::Progress => app::SortMode::Name,
                                };
                            }
                        }
                        KeyCode::Char('t') => {
                            app.toggle_theme();
                            save_config(app);
                        }
                        KeyCode::Char('+') | KeyCode::Char('=') => {
                            app.margin_width = app.margin_width.saturating_add(2);
                            save_config(app);
                        }
                        KeyCode::Char('-') | KeyCode::Char('_') => {
                            app.margin_width = app.margin_width.saturating_sub(2);
                            save_config(app);
                        }
                        KeyCode::Char('c') => {
                            if matches!(app.active_pane, ActivePane::Reader) {
                                if let Some(ref book) = app.current_book {
                                    app.toc_selected_index = book.current_chapter;
                                    app.mode = AppMode::Toc;
                                }
                            }
                        }
                        KeyCode::Char('m') => {
                            if matches!(app.active_pane, ActivePane::Reader) {
                                if let Some(ref mut book) = app.current_book {
                                    let bm = book::models::Bookmark {
                                        position: book.current_position,
                                        timestamp: chrono::Utc::now().to_rfc3339(),
                                        note: None,
                                    };
                                    book.bookmarks.push(bm);
                                    app.error_message = Some("Bookmark added".to_string());
                                }
                            }
                        }
                        KeyCode::PageDown | KeyCode::Char('f') => {
                            if matches!(app.active_pane, ActivePane::Reader) {
                                if let Some(ref mut book) = app.current_book {
                                    book.current_position += 20;
                                }
                            }
                        }
                        KeyCode::PageUp | KeyCode::Char('b') => {
                            if matches!(app.active_pane, ActivePane::Reader) {
                                if let Some(ref mut book) = app.current_book {
                                    book.current_position =
                                        book.current_position.saturating_sub(20);
                                }
                            }
                        }
                        KeyCode::Char('n') => {
                            if matches!(app.active_pane, ActivePane::Reader) {
                                if !app.search_query.is_empty() {
                                    app.find_next_search_result(true);
                                } else if let Some(ref mut book) = app.current_book {
                                    if book.current_chapter + 1 < book.chapters.len() {
                                        book.current_chapter += 1;
                                        book.current_position = 0;
                                    }
                                }
                            }
                        }
                        KeyCode::Char('N') => {
                            if matches!(app.active_pane, ActivePane::Reader) {
                                if !app.search_query.is_empty() {
                                    app.find_next_search_result(false);
                                }
                            }
                        }
                        KeyCode::Char('p') => {
                            if matches!(app.active_pane, ActivePane::Reader) {
                                if let Some(ref mut book) = app.current_book {
                                    if book.current_chapter > 0 {
                                        book.current_chapter -= 1;
                                        book.current_position = 0;
                                    }
                                }
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => match app.active_pane {
                            ActivePane::Library => app.next_book(),
                            ActivePane::Downloader => app.next_online_book(),
                            ActivePane::Reader => {
                                if let Some(ref mut book) = app.current_book {
                                    book.current_position += 1;
                                }
                            }
                        },
                        KeyCode::Up | KeyCode::Char('k') => match app.active_pane {
                            ActivePane::Library => app.previous_book(),
                            ActivePane::Downloader => app.previous_online_book(),
                            ActivePane::Reader => {
                                if let Some(ref mut book) = app.current_book {
                                    book.current_position = book.current_position.saturating_sub(1);
                                }
                            }
                        },
                        KeyCode::Enter => {
                            if matches!(app.active_pane, ActivePane::Library) {
                                app.open_selected_book();
                            } else if matches!(app.active_pane, ActivePane::Downloader) {
                                if let Some(idx) = app.selected_online_index {
                                    let url_and_id = if let Some(b) = app.online_books.get(idx) {
                                        b.epub_url().map(|url| (url, b.id))
                                    } else {
                                        None
                                    };

                                    if let Some((url, b_id)) = url_and_id {
                                        app.is_loading = true;
                                        app.error_message = Some("Downloading...".to_string());
                                        let tx = app.tx.clone();
                                        std::thread::spawn(move || {
                                            match reqwest::blocking::get(&url) {
                                                Ok(res) => {
                                                    if let Ok(bytes) = res.bytes() {
                                                        let path = std::env::current_dir()
                                                            .unwrap()
                                                            .join("books")
                                                            .join(format!("{}.epub", b_id));
                                                        if let Some(p) = path.parent() {
                                                            let _ = std::fs::create_dir_all(p);
                                                        }
                                                        std::fs::write(&path, bytes).unwrap();
                                                        match crate::book::loader::load_book(&path)
                                                        {
                                                            Ok(parsed_book) => {
                                                                let _ = tx.send(app::BackgroundMessage::DownloadComplete(Ok(parsed_book)));
                                                            }
                                                            Err(e) => {
                                                                let _ = tx.send(app::BackgroundMessage::DownloadComplete(Err(e)));
                                                            }
                                                        }
                                                    } else {
                                                        let _ =
                                                            tx.send(app::BackgroundMessage::Error(
                                                                "Failed to read bytes".into(),
                                                            ));
                                                    }
                                                }
                                                Err(e) => {
                                                    let _ = tx.send(app::BackgroundMessage::Error(
                                                        e.to_string(),
                                                    ));
                                                }
                                            }
                                        });
                                    } else {
                                        app.error_message = Some(
                                            "No EPUB format available for this book".to_string(),
                                        );
                                    }
                                }
                            }
                        }
                        _ => {}
                    },
                    AppMode::Command
                    | AppMode::Search
                    | AppMode::OnlineSearch
                    | AppMode::LibrarySearch => match key.code {
                        KeyCode::Enter => {
                            if matches!(app.mode, AppMode::Command) {
                                app.execute_command();
                            } else if matches!(app.mode, AppMode::LibrarySearch) {
                                app.library_filter = app.command_buffer.clone();
                                app.mode = AppMode::Normal;
                                app.command_buffer.clear();
                                app.selected_book_index = if app.filtered_books().is_empty() {
                                    None
                                } else {
                                    Some(0)
                                };
                            } else {
                                app.execute_search();
                            }
                        }
                        KeyCode::Tab => {
                            if matches!(app.mode, AppMode::Command) {
                                app.apply_suggestion();
                            }
                        }
                        KeyCode::Up => {
                            if matches!(app.mode, AppMode::Command) {
                                app.cycle_suggestion(false);
                            }
                        }
                        KeyCode::Down => {
                            if matches!(app.mode, AppMode::Command) {
                                app.cycle_suggestion(true);
                            }
                        }
                        KeyCode::Char(c) => {
                            app.command_buffer.push(c);
                            if matches!(app.mode, AppMode::LibrarySearch) {
                                app.library_filter = app.command_buffer.clone();
                                app.selected_book_index = if app.filtered_books().is_empty() {
                                    None
                                } else {
                                    Some(0)
                                };
                            }
                            if matches!(app.mode, AppMode::Command) {
                                app.update_suggestions();
                            }
                        }
                        KeyCode::Backspace => {
                            app.command_buffer.pop();
                            if matches!(app.mode, AppMode::LibrarySearch) {
                                app.library_filter = app.command_buffer.clone();
                                app.selected_book_index = if app.filtered_books().is_empty() {
                                    None
                                } else {
                                    Some(0)
                                };
                            }
                            if matches!(app.mode, AppMode::Command) {
                                app.update_suggestions();
                            }
                        }
                        KeyCode::Esc => {
                            app.mode = AppMode::Normal;
                            if matches!(app.mode, AppMode::LibrarySearch) {
                                app.library_filter.clear();
                                app.selected_book_index = if app.filtered_books().is_empty() {
                                    None
                                } else {
                                    Some(0)
                                };
                            }
                            app.command_buffer.clear();
                        }
                        _ => {}
                    },
                    AppMode::Preferences => match key.code {
                        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('P') => {
                            app.mode = AppMode::Normal;
                            save_config(app);
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            app.preferences_selected_index =
                                app.preferences_selected_index.saturating_sub(1);
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            if app.preferences_selected_index < 6 {
                                app.preferences_selected_index += 1;
                            }
                        }
                        KeyCode::Enter | KeyCode::Char(' ') => {
                            match app.preferences_selected_index {
                                0 => app.toggle_theme(),
                                1 => {
                                    app.text_alignment = match app.text_alignment.as_str() {
                                        "left" => "center".to_string(),
                                        "center" => "right".to_string(),
                                        _ => "left".to_string(),
                                    };
                                }
                                2 => {
                                    app.margin_width = if app.margin_width >= 20 {
                                        0
                                    } else {
                                        app.margin_width + 2
                                    }
                                }
                                3 => {
                                    app.startup_view = match app.startup_view.as_str() {
                                        "library" => "last_read".to_string(),
                                        _ => "library".to_string(),
                                    };
                                }
                                4 => app.case_sensitive_search = !app.case_sensitive_search,
                                5 => app.show_status_bar = !app.show_status_bar,
                                6 => app.wpm = if app.wpm >= 1000 { 100 } else { app.wpm + 50 },
                                _ => {}
                            }
                        }
                        KeyCode::Char('+') | KeyCode::Char('=') => {
                            if app.preferences_selected_index == 2 {
                                app.margin_width = app.margin_width.saturating_add(2);
                            }
                        }
                        KeyCode::Char('-') | KeyCode::Char('_') => {
                            if app.preferences_selected_index == 2 {
                                app.margin_width = app.margin_width.saturating_sub(2);
                            }
                        }
                        _ => {}
                    },
                    AppMode::Toc => match key.code {
                        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('c') => {
                            app.mode = AppMode::Normal;
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            app.toc_selected_index = app.toc_selected_index.saturating_sub(1);
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            if let Some(book) = &app.current_book {
                                if app.toc_selected_index + 1 < book.chapters.len() {
                                    app.toc_selected_index += 1;
                                }
                            }
                        }
                        KeyCode::Enter => {
                            if let Some(ref mut book) = app.current_book {
                                if app.toc_selected_index < book.chapters.len() {
                                    book.current_chapter = app.toc_selected_index;
                                    book.current_position = 0;
                                }
                            }
                            app.mode = AppMode::Normal;
                        }
                        _ => {}
                    },
                }
            } else if let Event::Mouse(mouse_event) = event::read()? {
                if matches!(app.mode, AppMode::Normal) {
                    match mouse_event.kind {
                        MouseEventKind::ScrollDown => {
                            if matches!(app.active_pane, ActivePane::Reader) {
                                if let Some(ref mut book) = app.current_book {
                                    book.current_position += 3;
                                }
                            } else if matches!(app.active_pane, ActivePane::Library) {
                                app.next_book();
                            } else {
                                app.next_online_book();
                            }
                        }
                        MouseEventKind::ScrollUp => {
                            if matches!(app.active_pane, ActivePane::Reader) {
                                if let Some(ref mut book) = app.current_book {
                                    book.current_position = book.current_position.saturating_sub(3);
                                }
                            } else if matches!(app.active_pane, ActivePane::Library) {
                                app.previous_book();
                            } else {
                                app.previous_online_book();
                            }
                        }
                        _ => {}
                    }
                } else if matches!(app.mode, AppMode::Toc) {
                    match mouse_event.kind {
                        MouseEventKind::ScrollDown => {
                            if let Some(book) = &app.current_book {
                                if app.toc_selected_index + 1 < book.chapters.len() {
                                    app.toc_selected_index += 1;
                                }
                            }
                        }
                        MouseEventKind::ScrollUp => {
                            app.toc_selected_index = app.toc_selected_index.saturating_sub(1);
                        }
                        _ => {}
                    }
                }
            }
        }

        if app.should_quit {
            save_config(app);
            return Ok(());
        }
    }
}

fn ui(f: &mut Frame, app: &App) {
    let main_chunks = if app.show_status_bar {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)].as_ref())
            .split(f.area())
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0)].as_ref())
            .split(f.area())
    };

    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)].as_ref())
        .split(main_chunks[0]);

    if matches!(app.active_pane, ActivePane::Downloader) {
        ui::downloader_panel::render(f, app, content_chunks[0]);
    } else {
        ui::library_panel::render(f, app, content_chunks[0]);
    }
    ui::reader::render(f, app, content_chunks[1]);

    if app.show_status_bar {
        ui::footer::render(f, app, main_chunks[1]);
    }

    if matches!(app.mode, AppMode::Toc) {
        ui::toc::render(f, app);
    }

    if matches!(app.mode, AppMode::Preferences) {
        ui::preferences::render(f, app);
    }

    if app.show_help {
        ui::help::render(f, app);
    }
}
