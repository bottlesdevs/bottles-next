build:
    cargo build --package bottles-cli
    cargo build --package bottles-core
    cargo build --package bottles-server
    cargo build --package bottles-winebridge --target x86_64-pc-windows-gnu

bridge prefix:
    cargo build --package bottles-winebridge --target x86_64-pc-windows-gnu
    cp target/x86_64-pc-windows-gnu/debug/bottles-winebridge.exe {{prefix}}
    # Execute bottles-winebridge.exe inside prefix

cli *args:
    cargo run --package bottles-cli {{args}}

server:
    cargo run --package bottles-server
