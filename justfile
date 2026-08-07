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

# Points each submodule's local origin remote at SSH instead of HTTPS.
submodules-ssh:
    git submodule foreach 'url=$(git remote get-url origin); case "$url" in https://github.com/bottlesdevs/*) new_url=$(echo "$url" | sed -E "s#https://github\.com/(bottlesdevs)/([^ \"]+?)(\.git)?\$#git@github.com:\1/\2.git#"); git remote set-url origin "$new_url" ;; esac'
    git submodule foreach 'git remote -v'
