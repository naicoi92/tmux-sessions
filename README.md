<div align="center">

# 🖥️ tmux-sessions

**TUI Popup Manager for tmux Sessions and zoxide Directories**

[![Version](https://img.shields.io/github/v/release/naicoi92/tmux-sessions?style=flat-square&color=blue)](https://github.com/naicoi92/tmux-sessions/releases)
[![License](https://img.shields.io/github/license/naicoi92/tmux-sessions?style=flat-square&color=green)](LICENSE)
[![CI](https://img.shields.io/github/actions/workflow/status/naicoi92/tmux-sessions/ci.yml?style=flat-square&label=CI)](https://github.com/naicoi92/tmux-sessions/actions)

_Blazingly fast session switching with fuzzy finding, live previews, and seamless zoxide integration_

</div>

---

## 📸 Demo

[![BAvbFPs.png](https://iili.io/BAvbFPs.png)](https://freeimage.host/i/BAvbFPs)

**Features at a glance:**

- 🔍 **Fuzzy find** through all tmux sessions and zoxide directories
- 👁️ **Live preview** of window content and directory listings
- ⚡ **Instant switch** with Enter key
- 🎯 **Smart sorting** - current window → same session → others → zoxide

---

## ✨ Features

### 🎯 Smart Session Management

- **Automatic context detection** - Works seamlessly inside or outside tmux
- **Session + Window view** - See all your tmux windows organized by session
- **Quick actions** - Switch, kill, or create new sessions instantly

### 🔍 Fuzzy Finding

- Powered by [nucleo](https://github.com/helix-editor/nucleo) for blazing fast fuzzy matching
- Real-time filtering as you type
- Case-insensitive search with smart ranking

### 👁️ Live Preview

- **Tmux window preview** - See actual terminal content before switching
- **Directory preview** - Browse directory contents with file icons
- **Async loading** - Non-blocking preview generation with 2s timeout
- **Dual-pane capture** - Intelligently captures both primary and alternate screen

### 📂 Zoxide Integration

- Automatically includes your frequently used directories from [zoxide](https://github.com/ajeetdsouza/zoxide)
- One-key session creation from any zoxide directory
- Visual distinction between tmux windows and zoxide directories

### 🎨 Beautiful UI

- Built with [ratatui](https://github.com/ratatui-org/ratatui) for a modern terminal interface
- Tailwind-inspired color palette (slate backgrounds, cyan accents)
- File icons via [devicons](https://github.com/alexheretic/devicons)
- ANSI color support in previews

---

## 🚀 Quick Start

### Prerequisites

- [tmux](https://github.com/tmux/tmux) 3.2+ (required for `display-popup` support)
- [zoxide](https://github.com/ajeetdsouza/zoxide) (optional, for directory integration)

### One-line Install

```sh
curl -fsSL https://raw.githubusercontent.com/naicoi92/tmux-sessions/main/install.sh | sh
```

### Manual Install

1. Download the latest binary for your platform from [Releases](https://github.com/naicoi92/tmux-sessions/releases)

2. Extract and move to your PATH:

```sh
tar -xzf tmux-sessions_*_*.tar.gz
mv tmux-sessions ~/.local/bin/
# Optional: create short alias
ln -s ~/.local/bin/tmux-sessions ~/.local/bin/ts
```

1. Add key binding to tmux.conf:

```sh
# ~/.tmux.conf
bind-key -n M-w display-popup -h 80% -w 80% -E "ts"
```

1. Reload tmux config:

```sh
tmux source-file ~/.tmux.conf
```

1. Press `Alt+w` (or your chosen key) to launch!

---

## 📖 How It Works

### Three Operating Modes

| Context              | Behavior                                       |
| -------------------- | ---------------------------------------------- |
| **Outside tmux**     | Attaches to 'main' session (creates if needed) |
| **Inside tmux pane** | Opens popup window with session manager        |
| **Inside popup**     | Runs the TUI interface                         |

### Workflow

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│  Press M-w   │────▶│   Popup      │────▶│  Fuzzy Find  │
│  (in tmux)   │     │   Opens      │     │  & Preview   │
└──────────────┘     └──────────────┘     └──────┬───────┘
                                                 │
                       ┌─────────────────────────┘
                       ▼
              ┌─────────────────┐
              │  Press Enter    │
              │  to switch      │
              └────────┬────────┘
                       ▼
              ┌─────────────────┐
              │  Popup closes   │
              │  Session active │
              └─────────────────┘
```

### Key Bindings

| Key                 | Action                       |
| ------------------- | ---------------------------- |
| `↑/↓` or `Ctrl+j/k` | Navigate items               |
| `Ctrl+p`            | Toggle preview panel         |
| `Ctrl+r`            | Reload snapshot              |
| `Ctrl+d`            | Kill selected window/session |
| `Enter`             | Switch to selected item      |
| `Esc` or `q`        | Quit                         |

---

## 📦 Installation Methods

### Using install.sh (Recommended)

Works with any POSIX-compliant shell (sh, bash, zsh, dash):

```sh
# Install latest release
curl -fsSL https://raw.githubusercontent.com/naicoi92/tmux-sessions/main/install.sh | sh

# Or download and run locally
sh install.sh

# Install specific version
sh install.sh --version v1.0.0

# Install to custom directory
sh install.sh --prefix ~/bin

# Dry run - see what would be installed
sh install.sh --dry-run
```

### Package Managers

#### macOS (Homebrew - coming soon)

```sh
brew install naicoi92/tap/tmux-sessions
```

#### Arch Linux (AUR - coming soon)

```sh
yay -S tmux-sessions
```

#### Debian/Ubuntu (.deb)

Download `.deb` from releases:

```sh
sudo dpkg -i tmux-sessions_1.0.0_amd64.deb
```

### Build from Source

Requires Rust 1.70+:

```sh
git clone https://github.com/naicoi92/tmux-sessions.git
cd tmux-sessions
cargo build --release
sudo cp target/release/tmux-sessions /usr/local/bin/
```

---

## 🏗️ Dependencies

| Crate                                                 | Purpose                         | Version |
| ----------------------------------------------------- | ------------------------------- | ------- |
| [ratatui](https://crates.io/crates/ratatui)           | Terminal UI framework           | 0.30.0  |
| [crossterm](https://crates.io/crates/crossterm)       | Cross-platform terminal control | 0.29.0  |
| [ansi-to-tui](https://crates.io/crates/ansi-to-tui)   | ANSI sequence rendering         | 8.0.1   |
| [nucleo](https://crates.io/crates/nucleo)             | Fuzzy matching engine           | 0.5.0   |
| [lscolors](https://crates.io/crates/lscolors)         | LS_COLORS parsing               | 0.21.0  |
| [humansize](https://crates.io/crates/humansize)       | Human-readable file sizes       | 2.1.3   |
| [chrono](https://crates.io/crates/chrono)             | Date/time handling              | 0.4.44  |
| [nu-ansi-term](https://crates.io/crates/nu-ansi-term) | ANSI styling                    | 0.50.3  |
| [devicons](https://crates.io/crates/devicons)         | File type icons                 | 0.6.12  |

---

## ❓ FAQs

### Q: What are the system requirements?

**A:**

- tmux 3.2+ (for `display-popup` support)
- Linux or macOS
- Terminal with 256-color support (recommended)

### Q: Does it work outside tmux?

**A:** Yes! When run outside tmux, it automatically attaches to a 'main' session (creates one if needed).

### Q: How do I change the key binding?

**A:** Edit your `~/.tmux.conf`:

```sh
# Use Alt+s instead of Alt+w
bind-key -n M-s display-popup -h 80% -w 80% -E "ts"
```

### Q: Can I use it without zoxide?

**A:** Absolutely! Zoxide integration is optional. Without it, you'll only see tmux sessions.

### Q: Why is the preview not showing?

**A:** Previews require the window to have visible content. Some applications that use the alternate screen (like vim, less) may show limited preview.

### Q: How do I kill a session?

**A:** Navigate to any window in the session and press `Ctrl+d`. You'll be asked to confirm.

### Q: Can I create new sessions?

**A:** Yes! Navigate to a zoxide directory and press Enter - it will create a new session automatically.

### Q: Is Windows supported?

**A:** Not currently. The tool relies on tmux which is primarily Unix-based. WSL may work but is untested.

### Q: How do I debug issues?

**A:** Run with `--debug` flag:

```sh
tmux-sessions --debug
```

---

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

### Development Setup

```sh
git clone https://github.com/naicoi92/tmux-sessions.git
cd tmux-sessions

# Run tests
cargo test --all
```

### Project Structure

- `src/domain/` - Core business logic (pure, no I/O)
- `src/adapters/` - External integrations (tmux, zoxide)
- `src/app/` - Application controller and state machine
- `src/preview/` - Async preview generation
- `src/ui/` - ratatui rendering

See [AGENTS.md](AGENTS.md) for detailed architecture documentation.

---

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

## 🙏 Acknowledgments

- Built with [ratatui](https://github.com/ratatui-org/ratatui) - the Rust TUI framework
- Inspired by [tmux-sessionizer](https://github.com/jrmoulton/tmux-sessionizer) and [zoxide](https://github.com/ajeetdsouza/zoxide)
- Fuzzy matching powered by [nucleo](https://github.com/helix-editor/nucleo)

---

<div align="center">

**[⬆ Back to Top](#-tmux-sessions)**

Made with ❤️ by [naicoi92](https://github.com/naicoi92)

</div>
