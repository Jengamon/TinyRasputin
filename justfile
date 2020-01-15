python := "python"
export RUST_BACKTRACE := "1"

# Run using a specified build
run mode +FLAGS='': (build mode)
    #!/usr/bin/env sh
    if [ "{{mode}}" = "release" ]; then
        cargo run --offline --frozen --release -q -- {{FLAGS}}
    else
        cargo run --offline --frozen -- {{FLAGS}}
    fi

# Build in specified mode
build mode: (_vendor-exists mode)
    #!/usr/bin/env sh
    if [ "{{mode}}" = "release" ]; then
        cargo build --offline --frozen -q --release
    else
        cargo build --offline --frozen
    fi

@_cargo_exists:
    test -f Cargo.toml

_clean-package mode:
    rm -f tinyrasputin-{{mode}}.zip

_clean-vendor mode:
    rm -rf vendor-{{mode}}

# Select a Cargo file based off of the desired mode
_select-cargo mode: (clean mode)
    rm -rf Cargo.toml
    cat Cargo-header.toml Cargo-{{mode}}.toml  > Cargo.toml

@_vendor-exists mode: (_cargo_exists)
    test -d vendor-{{mode}}

# Erase build artifacts for a selected mode
clean mode:
    rm -rf target

# Erase all build artifacts
clean-all: (clean "debug") (_clean-package "debug") (clean "release") (_clean-package "release") (_clean-vendor "debug") (_clean-vendor "release")

_update-config mode: (_select-cargo mode)
    rm -rf .cargo
    mkdir .cargo
    cargo update

# Build the vendor directory for a certain mode
build-vendor mode: (_update-config mode)
    cargo vendor --locked vendor-{{mode}} > .cargo/config

@_create_command_json mode:
    sed -e "s/MODE/{{mode}}/g" commands-template.json > commands.json

# Build the packge that we will upload to the server in the specified run mode
package mode: (_update-config mode) (_create_command_json mode) (build mode)
    #!/usr/bin/env sh
    echo 'Packing tinyrasputin-{{mode}}.zip...'
    for target in $PACKAGE_TARGETS_{{mode}} vendor-{{mode}}; do
        7z a -bb0 -bd tinyrasputin-{{mode}}.zip $target > nul;
    done

_create-test-directory mode:
    rm -rf ../server_tinyrasputin
    7z x -bb0 -y -o../server_tinyrasputin -- tinyrasputin-{{mode}}.zip > nul

# Test the built package of the specified mode
test-package mode: (_create-test-directory mode)
    cd .. && {{python}} engine.py

# Create a dependency graph for a mode
dep-graph mode: (_update-config mode) (_vendor-exists mode)
    cargo deps --all-deps | dot -Tpng > graph-{{mode}}.png

# Count the number of lines of code in the project
sloc:
    @echo "`wc -l src/**/*.rs src/*.rs` lines of code"