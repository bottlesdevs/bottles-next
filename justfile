cli := "target/debug/bottles-cli"
winebridge := "target/x86_64-pc-windows-gnu/debug/bottles-winebridge.exe"
catalog_port := "8731"
catalog_url := "http://127.0.0.1:" + catalog_port + "/macos-components.json"
data_dir := env_var("HOME") / "Library/Application Support/com.usebottles.bottles-next"
wine_runner := "5d1f4a90-6b1e-4c8a-9f2d-3a7b1c9e4f60"
gptk_runner := "039ff0b0-4f29-4a00-a4d4-89db82c9fff3"

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

# --- macOS runners ----------------------------------------------------------
# See MACOS.md for what these verify and why WineBridge needs Wine >= 7.13.

# Serves catalogs/ for --component-catalog; runs until interrupted.
catalog-serve port=catalog_port:
    @echo "serving catalogs/ at http://127.0.0.1:{{port}}/macos-components.json"
    python3 -m http.server {{port}} --bind 127.0.0.1 --directory catalogs

# Reports whether a Wine executable is new enough to host WineBridge.
wine-check wine:
    #!/usr/bin/env bash
    set -euo pipefail
    reported=$("{{wine}}" --version 2>&1 | head -1)
    numbers=${reported#wine-}
    major=${numbers%%.*}
    rest=${numbers#*.}
    minor=${rest%%[^0-9]*}
    echo "reported : $reported"
    echo "parsed   : ${major}.${minor}"
    if [ "$major" -gt 7 ] || { [ "$major" -eq 7 ] && [ "$minor" -ge 13 ]; }; then
        echo "verdict  : OK, WineBridge is supported (>= 7.13)"
    else
        echo "verdict  : TOO OLD, WineBridge cannot start (needs >= 7.13)"
        exit 1
    fi

# Builds WineBridge and hand-places it as an internal component (see MACOS.md).
winebridge-install:
    #!/usr/bin/env bash
    set -euo pipefail
    cargo build --package bottles-winebridge --target x86_64-pc-windows-gnu
    install -d "{{data_dir}}/components/winebridge/dev"
    cp "{{winebridge}}" "{{data_dir}}/components/winebridge/dev/"
    echo "placed at {{data_dir}}/components/winebridge/dev"

# Downloads one runner from the local catalog. Defaults to Wine Stable.
runner-install runner=wine_runner:
    #!/usr/bin/env bash
    set -euo pipefail
    cargo build --package bottles-cli
    python3 -m http.server {{catalog_port}} --bind 127.0.0.1 --directory catalogs >/dev/null 2>&1 &
    server=$!
    trap 'kill $server 2>/dev/null || true' EXIT
    until curl -sf "{{catalog_url}}" >/dev/null; do sleep 0.2; done
    # `refresh` reports the unconfigured dependency catalog; the runner listing is the real check.
    {{cli}} --component-catalog "{{catalog_url}}" addons refresh || true
    {{cli}} --component-catalog "{{catalog_url}}" addons runners download "{{runner}}"
    {{cli}} --component-catalog "{{catalog_url}}" addons runners list

# Runs the Wine end-to-end: bottle, then real program launch (~185 MB once).
macos-e2e: winebridge-install
    #!/usr/bin/env bash
    set -euo pipefail
    cargo build --package bottles-cli
    python3 -m http.server {{catalog_port}} --bind 127.0.0.1 --directory catalogs >/dev/null 2>&1 &
    server=$!
    trap 'kill $server 2>/dev/null || true' EXIT
    until curl -sf "{{catalog_url}}" >/dev/null; do sleep 0.2; done
    cli="{{cli}} --component-catalog {{catalog_url}}"

    $cli addons refresh || true
    if ! $cli addons runners list | grep -q "{{wine_runner}}.*Downloaded"; then
        echo "==> downloading Wine Stable"
        $cli addons runners download "{{wine_runner}}"
    fi

    echo "==> creating bottle e2e-wine"
    $cli bottle create e2e-wine --runner "{{wine_runner}}" >/dev/null
    program=$($cli bottle manage e2e-wine program add Proof 'C:\windows\syswow64\cmd.exe' \
        --arg /c --arg 'echo bottles-next-e2e > C:\proof.txt' | tail -1)

    echo "==> launching $program"
    log=$(mktemp)
    ($cli bottle manage e2e-wine program launch "$program" >"$log" 2>&1 &)
    for _ in $(seq 60); do grep -qE '^[0-9]+$' "$log" && break; sleep 2; done
    pid=$(grep -E '^[0-9]+$' "$log" | tail -1 || true)
    sleep 5

    proof=$(find "{{data_dir}}/bottles" -name proof.txt 2>/dev/null | head -1)
    if [ -n "$proof" ] && grep -q bottles-next-e2e "$proof"; then
        echo "PASS launched pid ${pid:-?}, guest wrote $(basename "$proof"): $(tr -d '\r\n' <"$proof")"
    else
        echo "FAIL no proof file; launch output:"; tail -20 "$log"; exit 1
    fi

# Verifies GPTK is refused with the version error, not os error 66 (~250 MB once).
macos-e2e-gate: winebridge-install
    #!/usr/bin/env bash
    set -euo pipefail
    cargo build --package bottles-cli
    python3 -m http.server {{catalog_port}} --bind 127.0.0.1 --directory catalogs >/dev/null 2>&1 &
    server=$!
    trap 'kill $server 2>/dev/null || true' EXIT
    until curl -sf "{{catalog_url}}" >/dev/null; do sleep 0.2; done
    cli="{{cli}} --component-catalog {{catalog_url}}"

    $cli addons refresh || true
    if ! $cli addons runners list | grep -q "{{gptk_runner}}.*Downloaded"; then
        echo "==> downloading Game Porting Toolkit"
        $cli addons runners download "{{gptk_runner}}"
    fi

    echo "==> creating bottle e2e-gptk (creation is expected to succeed)"
    $cli bottle create e2e-gptk --runner "{{gptk_runner}}" >/dev/null
    program=$($cli bottle manage e2e-gptk program add Proof 'C:\windows\syswow64\cmd.exe' \
        --arg /c --arg 'echo unreachable > C:\proof.txt' | tail -1)

    echo "==> launching, expecting the version gate to refuse"
    log=$(mktemp)
    if $cli bottle manage e2e-gptk program launch "$program" >"$log" 2>&1; then
        echo "FAIL launch succeeded on wine-7.7, which should be impossible"; exit 1
    fi
    if grep -q "requires wine-7.13" "$log"; then
        echo "PASS $(grep -m1 'requires wine-7.13' "$log")"
    else
        echo "FAIL launch failed without the version explanation:"; tail -20 "$log"; exit 1
    fi

# Removes the bottles and WineBridge component created by the macos-e2e recipes.
macos-e2e-clean:
    #!/usr/bin/env bash
    set -euo pipefail
    for name in e2e-wine e2e-gptk; do
        id=$({{cli}} bottle list 2>/dev/null | awk -v n="$name" '$2 == n { print $1 }')
        [ -z "$id" ] && continue
        pkill -f bottles-winebridge.exe 2>/dev/null || true
        rm -rf "{{data_dir}}/bottles/$id"
        echo "removed bottle $name ($id)"
    done
    rm -rf "{{data_dir}}/components/winebridge"
    rm -f "{{data_dir}}/components/index.toml" "{{data_dir}}/components/catalog.json"
    echo "done; downloaded runners kept (just macos-runners-clean drops them)"

# Deletes every downloaded runner component, reclaiming their disk space.
macos-runners-clean:
    #!/usr/bin/env bash
    set -euo pipefail
    runners="{{data_dir}}/components/runners"
    [ -d "$runners" ] || { echo "no runners installed"; exit 0; }
    du -sh "$runners"
    rm -rf "$runners"
    rm -f "{{data_dir}}/components/index.toml"
    echo "removed $runners"

# Points each submodule's local origin remote at SSH instead of HTTPS.
submodules-ssh:
    git submodule foreach 'url=$(git remote get-url origin); case "$url" in https://github.com/bottlesdevs/*) new_url=$(echo "$url" | sed -E "s#https://github\.com/(bottlesdevs)/([^ \"]+?)(\.git)?\$#git@github.com:\1/\2.git#"); git remote set-url origin "$new_url" ;; esac'
    git submodule foreach 'git remote -v'
