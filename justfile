package_targets := ".cargo benches justfile src vendor Cargo.toml Cargo.lock commands.json"
python := "python"
export RUST_BACKTRACE := "1"

# Run using a debug build
run-debug +FLAGS='': build-debug
    cargo run -- {{FLAGS}}

# Run using a release build
run-release +FLAGS='': build-release
    cargo run --offline --frozen --release -- {{FLAGS}}

# Build in debug mode
build-debug:
    cargo build --features "clap/suggestions clap/color" --lib

# Build in release mode
build-release:
    cargo build --offline --frozen --release

@_clean-package:
    rm -f tinyrasputin.zip

@_clean-vendor:
    rm -rf vendor

# Erase build artifacts
clean: _clean-package _clean-vendor
    rm -rf target

# Update the vendor directory
update-vendor: _clean-vendor
    cargo vendor

@_package: _clean-package
    echo 'Packing tinyrasputin.zip for release...'
    for target in {{package_targets}}; do \
        7z a -bb0 -bd tinyrasputin.zip $target > nul; \
    done

_create-test-directory: _package
    rm -rf ../server_tinyrasputin
    7z x -bb0 -y -o../server_tinyrasputin -- tinyrasputin.zip > nul

# Simulate what the package would do on the server
test-package: _create-test-directory
    cd .. && {{python}} engine.py

# Create a dependency graph
dep-graph:
    cargo deps --all-deps | dot -Tpng > graph.png

# Count the number of lines of code in the project
sloc:
    @echo "`wc -l src/**/*.rs` lines of code"