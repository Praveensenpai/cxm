# cxm (Codex Manager)

> Ultra-fast Codex Account Manager & Instant Switcher written in Rust.

`cxm` allows you to switch between multiple Codex accounts instantly without logging out.

---

## ⚡ Features

* 🚀 **Instant Account Switching**: Switch Codex accounts in <10ms without logging out.
* 🔑 **Automatic JWT Parsing**: Decodes account email dynamically from your Codex `auth.json` token.
* ➕ **Seamless New Session Flow**: Backs up your active session so you can log into a new account with zero setup.
* 🎯 **Interactive TUI Selector**: Select saved accounts with arrow keys using `inquire`.
* 🛠️ **CLI Subcommands**: Full CLI support for scripting (`cxm switch`, `cxm new`, `cxm save`, `cxm list`, `cxm remove`).
* 🐚 **Shell Autocompletions**: Native autocompletion support for `bash`, `zsh`, and `fish`.
* 📦 **Single Standalone Binary**: Zero runtime dependencies.

---

## 📥 Installation

### Quick One-Liner (Pre-compiled Binary)

```bash
curl -4 -sSL -H "Cache-Control: no-cache" https://raw.githubusercontent.com/Praveensenpai/cxm/main/install.sh | bash
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
Add the following to your `~/.zshrc` if not already present:
```zsh
fpath=(~/.zsh/completion $fpath)
autoload -U compinit && compinit
```

### Fish
```bash
mkdir -p ~/.config/fish/completions
cxm completions fish > ~/.config/fish/completions/cxm.fish
```

---

## 🚀 Usage

### 1. Interactive Switcher (Default)
Run `cxm` with no arguments to open the interactive selection menu:

```bash
cxm
```

### 2. Log in to a New Account
Back up your current session and prepare a fresh session to log into a new account:

```bash
cxm new # (or cxm add)
```
Then log in via `codex`, and run `cxm save` (or `cxm`) when finished to auto-detect and save your new account!

### 3. Save Current Account Session
Save your currently active Codex login session:

```bash
# Auto-detects email from auth token
cxm save

# Or specify a custom alias
cxm save work-account
```

### 4. Switch Account
Switch to a saved account directly by name or email:

```bash
cxm switch user@example.com
```

### 5. List Accounts
List all saved account profiles:

```bash
cxm list
```

### 6. Remove Account
Delete a saved account profile:

```bash
cxm remove user@example.com
```

---

## 📜 License

MIT © [Praveensenpai](https://github.com/Praveensenpai)
