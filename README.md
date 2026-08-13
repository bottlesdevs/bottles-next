# Bottles Next
Central repository for the Bottles Next project.

# Usage
After cloning the repository, download the submodules:

```bash
git submodule update --init --recursive
```

# Build
To build the project, you can use the `just` command:

```bash
just build
```

# macOS
Runner choice on macOS is constrained by a Wine limitation that affects
WineBridge. See [MACOS.md](MACOS.md), and `just macos-e2e` to verify a working
setup end to end.

# License
GPL-3.0
