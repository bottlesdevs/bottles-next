build *args:
    cargo build {{args}} --package bottles-cli
    cargo build {{args}} --package bottles-core
    cargo build {{args}} --package bottles-server
    cargo build {{args}} --package bottles-winebridge --target x86_64-pc-windows-gnu

check:
    cargo check --package bottles-cli
    cargo check --package bottles-core
    cargo check --package bottles-server
    cargo check --package bottles-winebridge --target x86_64-pc-windows-gnu

clippy:
    cargo clippy --package bottles-cli
    cargo clippy --package bottles-core
    cargo clippy --package bottles-server
    cargo clippy --package bottles-winebridge --target x86_64-pc-windows-gnu

bridge prefix:
    cargo build --package bottles-winebridge --target x86_64-pc-windows-gnu
    cp target/x86_64-pc-windows-gnu/debug/bottles-winebridge.exe {{prefix}}
    # Execute bottles-winebridge.exe inside prefix

cli *args:
    cargo run --package bottles-cli {{args}}

server:
    cargo run --package bottles-server

update:
    cd next-core && git pull origin main && cd ..
    cd next-cli && git pull origin main && cd ..
    cd next-docs && git pull origin main && cd ..
    cd next-server && git pull origin main && cd ..
    cd next-winebridge && git pull origin main && cd ..
    git commit -am "chore: update submodules"
