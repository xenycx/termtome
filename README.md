# BookRat 🐀📚

A fast, feature-rich, and highly customizable Terminal User Interface (TUI) E-book reader built with Rust. 

BookRat brings the joy of reading directly to your terminal. Whether you're a developer who never wants to leave their command line or someone who appreciates minimalistic, distraction-free environments, BookRat provides a robust reading experience with advanced features like in-book searching, progress tracking, and custom themes.

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)
![Status](https://img.shields.io/badge/status-active-success.svg)

## ✨ Features

BookRat is packed with everything you need for a comfortable reading experience in the terminal:

### 📖 Reading Experience
- **Broad Format Support**: Seamlessly parse and read EPUB files.
- **Reading Preferences**: Toggle between Light and Dark modes or create your own custom color schemes.
- **Adjustable Display**: Dynamically adjust reading layouts, text sizing simulation, and window splitting.
- **Progress Tracking**: Real-time stats on your chapter progress, time spent reading, and estimated reading speed (WPM).

### 🔍 Search & Navigation
- **In-Chapter Search**: Quickly find text using the `/` command, complete with result highlighting and `n`/`N` navigation.
- **Table of Contents**: Native TOC view to instantly jump between sections and chapters.
- **Smart Bookmarks**: Remembers your last read position, time spent, and preferred settings per book.

### 📚 Library Management
- **Organize Your Collection**: Support for categories, tags, and rich book metadata (Author, Title, Date, Description).
- **Library Filtering**: Sort your books by name, date, or progress, and quickly filter your library.
- **Full Library Control**: Import new books, delete old ones, and manage your collection directly from the TUI.

### ⌨️ Controls & UI
- **Keyboard-First Design**: Vim-like keybindings for fast and intuitive navigation.
- **Mouse Support**: Built-in terminal mouse support for scrolling and clicking.
- **Responsive UI**: Automatically handles terminal resizing and reflows text beautifully.

## 🚀 Getting Started

### Prerequisites

To build and run BookRat, you will need:
- [Rust](https://www.rust-lang.org/tools/install) (1.70 or newer recommended)
- Cargo (comes with Rust)

### Installation

1. **Clone the repository:**
   ```bash
   git clone https://github.com/xenycx/terminal-book-reader.git
   cd terminal-book-reader
   ```

2. **Build the project:**
   ```bash
   cargo build --release
   ```

3. **Install globally (Optional):**
   ```bash
   cargo install --path .
   ```
   *This allows you to run `terminal-book-reader` (or `bookrat` depending on your alias) from anywhere on your system.*

### Quick Start

Run the application directly using Cargo:

```bash
cargo run --release
```

Or, if installed globally, simply type the executable name in your terminal. You can pass the path to an EPUB file directly:

```bash
terminal-book-reader path/to/your/book.epub
```

## 🎮 Keybindings & Controls

BookRat is designed to be fully navigable via keyboard. (Press `?` inside the app to view the help menu anytime).

| Key | Action |
| :--- | :--- |
| `j` / `Down` | Scroll Down |
| `k` / `Up` | Scroll Up |
| `h` / `Left` | Previous Chapter / Go Back |
| `l` / `Right`| Next Chapter / Go Forward |
| `/` | Start Search |
| `n` / `N` | Next/Previous Search Result |
| `Tab` | Switch between Library and Reading view |
| `q` / `Esc` | Quit Application |

*(Note: Mouse scrolling and clicking are also supported if your terminal emulator allows it.)*

## 🛠️ Configuration & Data

By default, BookRat stores your imported books, reading progress, and configuration files in your system's standard user data directory. 

- **Linux:** `~/.local/share/terminal-book-reader/`
- **macOS:** `~/Library/Application Support/terminal-book-reader/`
- **Windows:** `C:\Users\{User}\AppData\Roaming\terminal-book-reader\`

## 🤝 Contributing

Contributions, issues, and feature requests are welcome! 
Feel free to check [issues page](https://github.com/xenycx/terminal-book-reader/issues).

1. Fork the Project
2. Create your Feature Branch (`git checkout -b feature/AmazingFeature`)
3. Commit your Changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the Branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

## 📝 License

Distributed under the MIT License. See `LICENSE` for more information.
