# cxm (Codex Manager)

> Ultra-fast Codex Account Manager & Instant Switcher written in Rust.

`cxm` allows you to switch between multiple Codex accounts instantly without logging out.

---

## ⚡ Features

* 🚀 **Instant Account Switching**: Switch Codex accounts in <10ms without logging out.
* 🔑 **Automatic JWT Parsing**: Decodes account email dynamically from your Codex `auth.json` token.
* 🎯 **Interactive TUI Selector**: Select saved accounts with arrow keys using `inquire`.
* 🛠️ **CLI Subcommands**: Full CLI support for scripting (`cxm switch`, `cxm save`, `cxm list`, `cxm remove`).
* 📦 **Single Standalone Binary**: Zero runtime dependencies.

---

## 📥 Installation

### Quick One-Liner (Pre-compiled Binary)

```bash
curl -fsSL https://raw.githubusercontent.com/Praveensenpai/cxm/main/install.sh | bash
```

### Build from Source

```bash
git clone https://github.com/Praveensenpai/cxm.git
cd cxm
chmod +x install.sh
./install.sh
```

---

## 🚀 Usage

### 1. Interactive Switcher (Default)
Run `cxm` with no arguments to open the interactive selection menu:

```bash
cxm
```

### 2. Save Current Account Session
Save your currently active Codex login session:

```bash
# Auto-detects email from auth token
cxm save

# Or specify a custom alias
cxm save work-account
```

### 3. Switch Account
Switch to a saved account directly by name or email:

```bash
cxm switch user@example.com
```

### 4. List Accounts
List all saved account profiles:

```bash
cxm list
```

### 5. Remove Account
Delete a saved account profile:

```bash
cxm remove user@example.com
```

---

## 📜 License

MIT © [Praveensenpai](https://github.com/Praveensenpai)
