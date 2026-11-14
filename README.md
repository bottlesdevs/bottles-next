# Bottles Next

Central repository for the Bottles Next project.

# Usage

Clone the repository:

SSH:

```bash
git clone --recursive git@github.com:bottlesdevs/bottles-next.git
```

HTTPS:

```bash
git clone --recursive https://github.com/bottlesdevs/bottles-next.git
```

# Build

## Prerequisites

- Rust via [rustup](https://rustup.rs). `rust-toolchain.toml` installs the
  `x86_64-pc-windows-gnu` target needed for WineBridge on first use.
- [`just`](https://github.com/casey/just)
- `protoc`, `pkg-config`, OpenSSL headers, CMake and Perl
- The MinGW-w64 cross compiler, used to build WineBridge for Windows
- Go 1.25 or newer, used to build the `fvs2d` daemon
- FUSE 3 (`fusermount3`), used by `fvs2d` at runtime

Debian and Ubuntu:

```bash
sudo apt install build-essential pkg-config libssl-dev protobuf-compiler cmake perl \
    gcc-mingw-w64-x86-64 golang-go fuse3
```

Fedora:

```bash
sudo dnf install gcc pkgconf-pkg-config openssl-devel protobuf-compiler cmake perl \
    mingw64-gcc golang fuse3
```

Nix users can use the development shell instead; see [NIX.md](NIX.md).

## Building and running

To build the project, you can use the `just` command:

```bash
just build
```

The UI and CLI start `fvs2d` for Virgo storage, so it must be on `PATH`.
`just fvs2d` builds it into cargo's `bin` directory:

```bash
just fvs2d
just run
```

To try the UI without `fvs2d`, build it without Virgo support:
`just run --no-default-features`.

A debug build of the whole workspace needs roughly 10–25 GB of disk space.

More recipes can be found in the `justfile`.


## Use of Generative AI
Some maintainers use generative AI tools as assistants while working in Bottles, in the spirit of Open Source, we want to be transparent about how, specifically:

- Code comments and documentation
- Boilerplate and repetitive code
- Issue triage (spotting duplicates, outdated reports, grouping similar issues)

Tools vary between contributors (currently mostly Claude and Codex): each AI-assisted commit states the tool and model used in its `Assisted-by` trailer.

### What we don't use it for
Architecture, complex logic, the security and sandboxing model and user experience are designed and written by the maintainers, manually.

### Human review
Every line of generated code, documentation and comments are reviewed by a maintainer before it is merged.

Also, starting from the 10th Sep 2026, the following commit pattern must be used for contributions made with or helped with the AI:

```plain
feat: add support for X

Assisted-by: <tool>:<model-version>
AI-Scope: what the AI generated in this commit, and the prompt used (or a short summary of it)
```

Trivial completions (single lines, renames, formatting) don't need to be marked.

Not following this layout will lead to a closed Pull Request.

# License

GPL-3.0

Coding agents must also follow [AGENTS.md](AGENTS.md) before changing files,
creating commits, or opening pull requests.
