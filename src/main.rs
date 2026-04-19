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
            if let Event::Mouse(mouse_event) = event::read()? {
                if app.show_help {
                    match mouse_event.kind {
                        MouseEventKind::ScrollDown => {
                            app.help_scroll = app.help_scroll.saturating_add(3)
                        }
                        MouseEventKind::ScrollUp => {
                            app.help_scroll = app.help_scroll.saturating_sub(3)
                        }
                        _ => {}
                    }
                } else if matches!(app.mode, AppMode::Normal) {
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
