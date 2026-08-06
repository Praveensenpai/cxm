# cxm (Codex Manager)

> Ultra-fast Codex Account Manager & Instant Switcher written in Rust.

`cxm` allows you to switch between multiple Codex accounts instantly without logging out and jump directly into previous chat sessions.

---

## ⚡ Features

* 🚀 **Instant Account Switching**: Switch Codex accounts in <10ms without logging out.
* 💬 **Session Explorer**: Browse and jump directly into previous Codex sessions via `codex resume`.
* 🔑 **Automatic JWT Parsing**: Decodes account email dynamically from your Codex `auth.json` token.
* 📊 **Live Quota & Usage Display**: Shows plan type and remaining usage % next to accounts.
* ⚡ **Smart 5-Minute Quota Cache**: Caches usage metrics locally in `~/.codex-accounts/.quota_cache.json` for instant UI execution.
* 🔄 **Cache Bypass**: Supports `--no-cache` (`-n`) to force refreshing live quota on demand.
* ➕ **Seamless New Session Flow**: Backs up your active session so you can log into a new account with zero setup.
* 🗑️ **In-Menu Account Deletion**: Safely delete unused account profiles directly from the interactive TUI menu.
* 🎯 **Interactive TUI Selector**: Select saved accounts or jump into sessions using `inquire`.
* 🛠️ **CLI Subcommands**: Streamlined CLI support for scripting (`cxm <account>`, `cxm sessions`, `cxm new`, `cxm save`, `cxm list`).
* 🐚 **Shell Autocompletions**: Native autocompletion support for `bash`, `zsh`, and `fish`.
* 📦 **Single Standalone Binary**: Zero runtime dependencies.

---

## 📥 Installation

### Quick One-Liner (Pre-compiled Binary)

```bash
curl -sSL -H "Accept: application/vnd.github.v3.raw" https://api.github.com/repos/Praveensenpai/cxm/contents/install.sh | bash
```

### Build from Source

```bash
git clone https://github.com/Praveensenpai/cxm.git
cd cxm
chmod +x install.sh
./install.sh
```

---

## ⚡ Shell Autocompletions Setup

`install.sh` automatically installs completions for `bash`, `zsh`, and `fish`.

If installing manually from source or via `cargo install`, generate completions for your shell:

### Bash
```bash
mkdir -p ~/.local/share/bash-completion/completions
cxm completions bash > ~/.local/share/bash-completion/completions/cxm
```

### Zsh
```bash
mkdir -p ~/.zsh/completion
cxm completions zsh > ~/.zsh/completion/_cxm
```

### Fish
```bash
mkdir -p ~/.config/fish/completions
cxm completions fish > ~/.config/fish/completions/cxm.fish
```

---

## 🚀 Usage

### 1. Interactive Menu (Default)
Run `cxm` with no arguments to open the interactive selection menu (switch accounts, jump to sessions, save, log into new account, or delete accounts):

```bash
cxm
```

### 2. Direct Account Switch
Switch to a saved account directly by name or email:

```bash
cxm user@example.com
```

### 3. Resume Previous Sessions
Browse previous chat sessions and resume instantly:

```bash
cxm sessions # (or cxm s)
```

### 4. Bypass Quota Cache
Force fetching fresh live quota directly from the backend API:

```bash
cxm -n
# or
cxm list --no-cache
```

### 5. Log in to a New Account
Back up your current session and prepare a fresh session to log into a new account:

```bash
cxm new # (or cxm add)
```

### 6. Save Current Account Session
Save your currently active Codex login session:

```bash
cxm save
```

### 7. List Accounts
List all saved account profiles with quota and cache timestamps:

```bash
cxm list
```

---

## 📜 License

MIT © [Praveensenpai](https://github.com/Praveensenpai)
