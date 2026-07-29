build *args:
    cargo build {{args}} --package bottles-cli
    cargo build {{args}} --package bottles-core
    cargo build {{args}} --package bottles-winebridge --target x86_64-pc-windows-gnu

check:
    cargo check --package bottles-cli
    cargo check --package bottles-core
    cargo check --package bottles-winebridge --target x86_64-pc-windows-gnu

clippy:
    cargo clippy --package bottles-cli
    cargo clippy --package bottles-core
    cargo clippy --package bottles-winebridge --target x86_64-pc-windows-gnu

update:
    cd crates/next-core && git checkout main && git pull origin main && cd -
    cd crates/next-cli && git checkout main && git pull origin main && cd -
    cd crates/next-docs && git checkout main && git pull origin main && cd -
    cd crates/next-winebridge && git checkout main && git pull origin main && cd -
    cd crates/next-proto && git checkout main && git pull origin main && cd -
    cd crates/download-manager && git checkout main && git pull origin main && cd -
    cd crates/fvs2-rs && git checkout main && git pull origin main && cd -
    git commit -am "chore: update submodules"
