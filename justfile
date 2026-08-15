# Builds all packages.
build *args:
    cargo build {{args}} --package download-manager
    cargo build {{args}} --package http-client
    cargo build {{args}} --package fvs-rs
    cargo build {{args}} --package bottles-cli
    cargo build {{args}} --package bottles-core
    cargo build {{args}} --package bottles-server
    cargo build {{args}} --package next-plugin-registry
    cargo build {{args}} --package next-plugin-egs
    cargo build {{args}} --package next-plugin-gog
    cargo build {{args}} --package next-proto
    cargo build {{args}} --package next-config
    cargo build {{args}} --package next-ui
    cargo build {{args}} --package bottles-winebridge --target x86_64-pc-windows-gnu

# Runs cargo check on all packages.
check:
    cargo check --package download-manager
    cargo check --package http-client
    cargo check --package fvs-rs
    cargo check --package bottles-cli
    cargo check --package bottles-core
    cargo check --package bottles-server
    cargo check --package next-plugin-registry
    cargo check --package next-plugin-egs
    cargo check --package next-plugin-gog
    cargo check --package next-proto
    cargo check --package next-config
    cargo check --package next-ui
    cargo check --package bottles-winebridge --target x86_64-pc-windows-gnu

# Runs clippy on all packages.
clippy:
    cargo clippy --package download-manager
    cargo clippy --package http-client
    cargo clippy --package fvs-rs
    cargo clippy --package bottles-cli
    cargo clippy --package bottles-core
    cargo clippy --package bottles-server
    cargo clippy --package next-plugin-registry
    cargo clippy --package next-plugin-egs
    cargo clippy --package next-plugin-gog
    cargo clippy --package next-proto
    cargo clippy --package next-config
    cargo clippy --package next-ui
    cargo clippy --package bottles-winebridge --target x86_64-pc-windows-gnu

# Updates all submodules to the latest version.
update:
    cd crates/download-manager && git checkout main && git pull origin main && cd -
    cd crates/fvs2-rs && git checkout main && git pull origin main && cd -
    cd crates/bottles-cli && git checkout main && git pull origin main && cd -
    cd crates/bottles-core && git checkout main && git pull origin main && cd -
    cd crates/bottles-server && git checkout main && git pull origin main && cd -
    cd crates/next-plugin-registry && git checkout main && git pull origin main && cd -
    cd crates/next-plugin-egs && git checkout main && git pull origin main && cd -
    cd crates/next-plugin-gog && git checkout main && git pull origin main && cd -
    cd crates/next-proto && git checkout main && git pull origin main && cd -
    cd crates/next-config && git checkout main && git pull origin main && cd -
    cd crates/next-ui && git checkout main && git pull origin main && cd -
    cd crates/bottles-winebridge && git checkout main && git pull origin main && cd -

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
