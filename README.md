# noil

**noil** is a structured, text-buffer-based file operation tool – think of it
like [`oil.nvim`](https://github.com/stevearc/oil.nvim), but for any editor,
terminal, or automated process.

Edit filesystem operations like it's plain text, and then apply them in a
controlled, explicit way.

![demo](assets/demo.gif)

---

## Interactive Mode (TBA)

I am planning an interactive TUI mode, where you don't have to care about tags,
like in `oil`. For now the normal editor is quite useful though, and allows all
types of editors to easily move, edit files and so on.

---

## ✨ Features

- Edit your file tree like a normal buffer
- Preview, format, and apply changes
- Integrates with `$EDITOR`
- CLI first, editor agnostic
- No surprises: nothing is applied until you say so

---

## 🛠️ Usage

### 1. Basic CLI

```bash
# Preview file tree and tags
noil . 

# Edit in your $EDITOR
noil edit .

# Format an existing buffer (e.g. from within your editor)
cat something.noil | noil fmt > something.noil

# Apply changes from a buffer
cat something.noil | noil apply
```

noil will ask you if you want to apply your changes before doing any operations.

---

## ✍️ Syntax

Each line follows this format:

```
<operation> <tag?> : <filepath>
```

### Supported operations:

| Operation | Meaning                                                | Tag Required? |
| --------: | ------------------------------------------------------ | ------------- |
|     `ADD` | Add new file                                           | ❌ No         |
|    `COPY` | Copy file with given tag                               | ✅ Yes        |
|  `DELETE` | Delete file with given tag                             | ✅ Yes        |
|    `MOVE` | Move file with given tag                               | ✅ Yes        |
|    `OPEN` | Open a file with a given tag (requires --chooser-file) | ❌ No         |
| _(blank)_ | Reference existing file (default)                      | ✅ Yes        |

---

### Example

```
         abc   :   /etc/nginx
COPY     abc   :   /tmp/nginx-copy
DELETE   123   :   /etc/nginx
ADD            :   /new/file.txt
OPEN           :   /new/file.txt
```

You can use short, unique tags (like `abc`, `ng1`, etc.) to refer to files.
`noil` will generate these tags when you run `noil .`.

---

## 🧽 Formatting

Want to clean up alignment and spacing?

```bash
cat my-buffer.noil | noil fmt
```

Or automatically format inside your editor with the following config for
[Helix](https://helix-editor.com):

```toml
# .config/helix/languages.toml
[[language]]
name = "noil"
scope = "source.noil"
injection-regex = "noil"
file-types = ["noil"]
auto-format = true
indent = { tab-width = 3, unit = "  " }
formatter = { command = "noil", args = ["fmt"] }

[[grammar]]
name = "noil"
source = { git = "https://git.kjuulh.io/kjuulh/tree-sitter-noil.git", rev = "2f295629439881d0b9e89108a1296881d0daf7b9" }

# .config/helix/config.toml
# Optional extra command Space + o will open noil allowing edits and the OPEN command
[keys.normal.space]
o = [
  ":sh rm -f /tmp/unique-file-kjuulh",
  # DISCLAIMER: Until noil has a proper interactive mode, we cannot ask for confirmation, as such we always commit changes, you don't get to have a preview unlike the normal cli option
  ":insert-output noil edit '%{buffer_name}' --chooser-file=/tmp/unique-file-kjuulh --commit --quiet < /dev/tty",
  ":insert-output echo \"x1b[?1049h\" > /dev/tty",
  ":open %sh{cat /tmp/unique-file-kjuulh}",
  ":redraw",
]
```

### Edit options

When using `noil edit .` a few additional options are available

- `--chooser-file`: A chooser file is a newline delimited file where each line
  corresponds to a relative file to be opened or manipulated by the user. Only
  items with `OPEN` command will be added to the file
- `--commit`: commit files without asking for confirmation
- `--quiet`: don't print results

---

## 🔒 Safety First

No changes are ever made unless you explicitly apply them with:

```bash
# Closing the file, will trigger an apply, asking for prompt like normal
noil edit .

noil apply < my-buffer.noil
```

You will be prompted before anything is modified.

---

## 🧠 Philosophy

noil gives you full control over file operations in a composable and
editor-friendly way. Think Git index, but for actual file moves and deletions —
human-editable, patchable, and grep-able.

---

## 📦 Installation

**Build from crates**:

```bash
cargo install noil
```

**Build from source**:

```bash
cargo install --git https://git.kjuulh.io/kjuulh/noil.git
```

Or clone locally and run with `cargo run`.

---

## 📋 License

MIT
