package_targets := ".cargo benches justfile src vendor Cargo.toml Cargo.lock commands.json"

run_debug +FLAGS='': build_debug
    cargo run -- {{FLAGS}}

run_release +FLAGS='': build_release
    cargo run --offline --frozen --release -- {{FLAGS}}

build_debug:
    cargo build --features "clap/suggestions clap/color" --lib

build_release:
    cargo build --offline --frozen --release

@_clean_package:
    rm -f tinyrasputin.zip

@_clean_vendor:
    rm -rf vendor

@clean: _clean_package _clean_vendor
    rm -rf target

update_vendor: _clean_vendor
    cargo vendor

@_package: _clean_package
    echo 'Packing tinyrasputin.zip for release...'
    for target in {{package_targets}}; do \
        7z a -bb0 -bd tinyrasputin.zip $target > nul; \
    done

# Test if the package would build on the server
test_package: _package
    rm -rf test
    7z x -bb0 -y -otest -- tinyrasputin.zip > nul
    just -d test -f test/justfile build_release

sloc:
    @echo "`wc -l src/**/*.rs` lines of code"