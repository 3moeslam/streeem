# Installing Streeem

Streeem is a Rust TUI that hosts multiple terminals in a staggered grid. It runs on macOS (Apple Silicon and Intel) and Linux. The four supported install paths are listed in order of friction (lowest first) so most users only need to read the first one that applies to them.

---

## 1. macOS — Homebrew (recommended once the tap is published)

> ⚠ This path requires the maintainer to have set up the [`3moeslam/homebrew-streeem`](https://github.com/3moeslam/homebrew-streeem) tap and pushed at least one tagged release. If `brew tap` fails with a 404, fall back to path 2 or 3.

```sh
brew tap 3moeslam/streeem
brew install streeem
```

After install:

```sh
streeem --name shell "${SHELL:-bash} -i"
```

To upgrade later:

```sh
brew upgrade streeem
```

To uninstall:

```sh
brew uninstall streeem
brew untap 3moeslam/streeem
```

---

## 2. Any OS — download a prebuilt binary from GitHub Releases

Each tagged release on [`3moeslam/streeem`](https://github.com/3moeslam/streeem/releases) ships:

- `streeem-aarch64-apple-darwin.tar.xz` — macOS Apple Silicon
- `streeem-x86_64-apple-darwin.tar.xz` — macOS Intel
- `streeem-installer.sh` — universal `curl | sh` installer that picks the right one

The installer is the easiest path:

```sh
curl --proto '=https' --tlsv1.2 -fsSL \
  https://github.com/3moeslam/streeem/releases/latest/download/streeem-installer.sh | sh
```

This installs to `~/.cargo/bin/streeem` (no `sudo` needed) and prints PATH instructions if needed.

If you'd rather install manually:

```sh
# Pick the right tarball for your machine:
curl -fsSL -o streeem.tar.xz \
  https://github.com/3moeslam/streeem/releases/latest/download/streeem-aarch64-apple-darwin.tar.xz
tar -xJf streeem.tar.xz
sudo install -m 0755 streeem-aarch64-apple-darwin/streeem /usr/local/bin/streeem
streeem --version
```

Verify the SHA-256 against `streeem-aarch64-apple-darwin.tar.xz.sha256` in the release if you care about supply-chain integrity.

---

## 3. Any OS with Rust — `cargo install` from source

If the target machine has a Rust toolchain (`rustc` + `cargo`), you can build directly from the git repo:

```sh
cargo install --git https://github.com/3moeslam/streeem --bin streeem
```

This compiles streeem in `~/.cargo/registry`, then installs the resulting binary to `~/.cargo/bin/streeem`. Make sure `~/.cargo/bin` is on your PATH (the rustup installer adds it automatically).

To install a specific version:

```sh
cargo install --git https://github.com/3moeslam/streeem --tag v0.2.1 --bin streeem
```

Don't have Rust? One-line install:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

---

## 4. From a local clone — build and install yourself

If you have the streeem source tree (e.g., from a `git clone`):

```sh
git clone https://github.com/3moeslam/streeem
cd streeem

# Install into ~/.local/bin (no sudo needed):
./scripts/install-local.sh

# Or system-wide into /usr/local/bin (requires sudo):
./scripts/install-local.sh --system

# Or any other prefix:
./scripts/install-local.sh --prefix /opt/streeem/bin
```

The script:

1. Runs `cargo build -p streeem --release` (creates `target/release/streeem`).
2. Copies the binary to the chosen install location.
3. Verifies `streeem --version` works from the install location.
4. Prints PATH-update instructions if needed.

To **build and run without installing** (useful for development):

```sh
./scripts/run.sh                      # default: shell + demo tile
./scripts/run.sh 'cargo watch -x test'         # custom command
./scripts/run.sh --release 'bash -i'  # release profile, custom command
```

---

## Verifying the install

```sh
streeem --version
streeem --help
```

Run with one tile to make sure everything works:

```sh
streeem --name shell "${SHELL:-bash} -i"
```

You should see a single tile filling the terminal, with a `$` prompt at the right size for the tile and a status bar at the bottom reading something like:

```
type to focused tile  •  Esc Esc:command mode  •  Ctrl+Q:quit
```

---

## Keybindings (quick reference)

While running streeem:

| | |
|---|---|
| **Anything** (typed) | Forwarded to the focused tile's PTY |
| `Esc` (single) | Forwarded to the tile (vim/less get it instantly) |
| `Esc` `Esc` (within 500 ms) | Enter command mode |
| `Ctrl+Q` | Quit (always works) |

In command mode (after Esc Esc):

| | |
|---|---|
| `a` | Spawn a new tile running `$SHELL -i` |
| `x` | Drop the focused tile |
| `n` / `p` | Cycle focus to next / previous tile |
| `f` | Toggle follow-tail on the focused tile |
| `q` | Quit |
| `Esc` | Exit command mode (back to typing into the tile) |

Command mode auto-exits after 5 seconds of no input.

---

## Troubleshooting

**`streeem: command not found`**
The install dir isn't on your PATH. Run `which streeem` to see what's expected, then add the install directory to your shell's startup file:

```sh
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

(Substitute `~/.local/bin` or wherever you actually installed.)

**`streeem` opens but nothing renders**
The terminal may not be reporting size correctly. Resize the window once — streeem polls the size every tick and will reflow on the next. If the issue persists, run with a single tile (`streeem 'bash -i'`) to confirm it isn't a layout bug. Diagnostics are written to `/tmp/streeem.log`.

**Output looks garbled or cursor positions wrong**
TUI apps that use the kitty graphics protocol (sixel images, kitty's image extensions) aren't supported — streeem uses the `vt100` emulator which covers xterm-256color but not those extensions. For Claude Code / Codex / vim / htop, output should be correct; for image-based apps it won't be.

**`brew install streeem` says formula not found**
The Homebrew tap isn't published yet (or you didn't run `brew tap 3moeslam/streeem` first). Use the GitHub Releases installer (path 2) or `cargo install --git` (path 3) instead.

**Apple Gatekeeper warns "streeem can't be opened because Apple cannot check it"**
The binary isn't notarised. Either:
- Right-click → Open in Finder, then "Open Anyway" in System Settings → Privacy & Security, or
- Install via Homebrew (path 1) which Gatekeeper trusts, or
- `xattr -d com.apple.quarantine /usr/local/bin/streeem` to remove the quarantine attribute manually.

For a tool you built from source yourself (path 3 or 4), Gatekeeper won't complain — the warning only appears on downloaded binaries.

---

## Uninstalling

```sh
# Homebrew:
brew uninstall streeem
brew untap 3moeslam/streeem

# Manual install (path 2 or 4):
rm /usr/local/bin/streeem      # or wherever it was installed

# cargo install (path 3):
cargo uninstall streeem
```

The diagnostic log at `/tmp/streeem.log` can be deleted at any time:

```sh
rm /tmp/streeem.log
```
