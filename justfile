# Builds all packages.
build *args:
    cargo build {{args}} --package bottles-cli
    cargo build {{args}} --package bottles-core
    cargo build {{args}} --package bottles-winebridge --target x86_64-pc-windows-gnu

# Runs cargo check on all packages.
check:
    cargo check --package bottles-cli
    cargo check --package bottles-core
    cargo check --package bottles-winebridge --target x86_64-pc-windows-gnu

# Runs clippy on all packages.
clippy:
    cargo clippy --package bottles-cli
    cargo clippy --package bottles-core
    cargo clippy --package bottles-winebridge --target x86_64-pc-windows-gnu

# Updates all submodules to the latest version.
update:
    cd crates/next-core && git checkout main && git pull origin main && cd -
    cd crates/next-cli && git checkout main && git pull origin main && cd -
    cd crates/next-docs && git checkout main && git pull origin main && cd -
    cd crates/next-winebridge && git checkout main && git pull origin main && cd -
    cd crates/next-proto && git checkout main && git pull origin main && cd -
    cd crates/download-manager && git checkout main && git pull origin main && cd -
    cd crates/fvs2-rs && git checkout main && git pull origin main && cd -

# Builds and opens documentation for a specific package.
doc package:
    cargo doc --no-deps --package {{package}}
    open target/doc/{{replace(package, "-", "_")}}/index.html

# Runs the Registry, every storefront plugin, and the gRPC server together.
# The Registry must be up before anything else dials it, so it starts
# first with a short head start. Ctrl+C stops everything.
serve:
    cargo build --package next-plugin-registry --package bottles-server --package next-plugin-egs --package next-plugin-gog
    cargo run --package next-plugin-registry &
    sleep 1
    cargo run --package next-plugin-egs &
    cargo run --package next-plugin-gog &
    cargo run --package bottles-server &
    wait
