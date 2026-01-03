# Bottles Next
Central repository for the Bottles Next project.

# Usage
After cloning the repository, download the submodules:

```bash
git submodule update --init --recursive
```

This project contains the following submodules:
- [next-cli](https://github.com/bottlesdevs/next-cli)
- [next-core](https://github.com/bottlesdevs/next-core)
- [next-server](https://github.com/bottlesdevs/next-server)
- [next-winebridge](https://github.com/bottlesdevs/next-winebridge)
- [next-docs](https://github.com/bottlesdevs/next-docs)

# Dependencies

- rust toolchain
- rustup target add x86_64-pc-windows-gnu
- mingw64-gcc and mingw64-binutils

# Build
To build the project, you can use the `just` command:

```bash
just build
```

# License
GPL-3.0
