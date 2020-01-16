python := "python"
build-dir := "build"
mode := "debug"
package-targets := "PACKAGE_TARGETS_" + mode
base-package := "src " + env_var(package-targets)
package-contents := "vendor .cargo/config Cargo.* commands.json .package-list justfile " + base-package

export RUST_BACKTRACE := "1"

alias rebuild := rebuild-environment
alias clean := clean-environment
alias env := build-environment
alias build := local-build
alias run := local-run
alias test := local-test

# Runs tinyrasputin in a certain mode locally
local-run +FLAGS='': (local-build)
    if [ {{mode}} = release ]; then \
        cargo run --offline --release --manifest-path {{build-dir}}/{{mode}}/Cargo.toml -- {{FLAGS}}; \
    else \
        cargo run --offline --manifest-path {{build-dir}}/{{mode}}/Cargo.toml -- {{FLAGS}}; \
    fi

# Builds tinyrasputin in a certain mode locally
local-build: (_copy-files)
    if [ {{mode}} = release ]; then \
        cargo build --offline --release --manifest-path {{build-dir}}/{{mode}}/Cargo.toml; \
    else \
        cargo build --offline --manifest-path {{build-dir}}/{{mode}}/Cargo.toml; \
    fi

# Tests tinyrasputin in a certain mode locally
local-test +FLAGS='': (local-build)
    if [ {{mode}} = release ]; then \
        cargo test --offline --release --manifest-path {{build-dir}}/{{mode}}/Cargo.toml -- {{FLAGS}}; \
    else \
        cargo test --offline --manifest-path {{build-dir}}/{{mode}}/Cargo.toml -- {{FLAGS}}; \
    fi

# Builds tinyrasputin in a certain mode
package-build: (_select-cargo) (_copy-files) (_generate-package-listing)
    cd {{build-dir}}/{{mode}} && just -d . --justfile justfile mode={{mode}} build

# Tests tiny rasputin in a certain mode as it would run in package mode
package-test +FLAGS='': (package-build)
    cd {{build-dir}}/{{mode}} && just -d . --justfile justfile mode={{mode}} test {{FLAGS}}

_make-build-dir: 
    mkdir -p {{build-dir}}/{{mode}}
    echo "Created environment for {{mode}} build."

_build-dir-exists:
    test -d {{build-dir}}/{{mode}}

_vendor-exists: (_build-dir-exists)
    test -d {{build-dir}}/{{mode}}/vendor

_copy-files: (_build-dir-exists)
    cp -rt {{build-dir}}/{{mode}} {{base-package}}
    cp package-justfile {{build-dir}}/{{mode}}/justfile
    @echo 'Renewed basic build environment for {{mode}} build'

# Select a Cargo file based off of the desired mode
_select-cargo: (_build-dir-exists) (_copy-files)
    rm -rf {{build-dir}}/{{mode}}/Cargo.toml
    cat Cargo-header.toml Cargo-{{mode}}.toml  > {{build-dir}}/{{mode}}/Cargo.toml
    @echo "Created Cargo.toml for {{mode}} build."

_clean-package:
    rm -f tinyrasputin-{{mode}}.zip

_clean-vendor:
    rm -rf {{build-dir}}/{{mode}}/vendor

_generate-package-listing: (_vendor-exists) (_copy-files) (_create-command-json)
    rm -rf {{build-dir}}/{{mode}}/.package-list
    cd {{build-dir}}/{{mode}} && find {{package-contents}} -type f -print > .package-list

# Erase build artifacts for a selected mode
clean-environment:
    rm -rf {{build-dir}}/{{mode}}

# Erase all build artifacts
clean-all:
    rm -rf {{build-dir}}

# Build the build directory for a certain mode
build-environment: (clean-environment) (_make-build-dir) (_select-cargo) (_clean-vendor)
    rm -rf {{build-dir}}/{{mode}}/.cargo
    mkdir {{build-dir}}/{{mode}}/.cargo
    cd {{build-dir}}/{{mode}} && cargo update
    cd {{build-dir}}/{{mode}} && cargo vendor vendor > .cargo/config

_create-command-json +FLAGS='': (_build-dir-exists)
    sed -e "s/MODE/{{mode}}/g" -e "s/FLAGS/{{FLAGS}}/g" commands-template.json > {{build-dir}}/{{mode}}/commands.json

# Build the packge that we will upload to the server in the specified run mode
package +FLAGS='': (_clean-package) (_create-command-json FLAGS) (package-build) (_generate-package-listing)
    @echo 'Packing tinyrasputin-{{mode}}.zip...'
    cd {{build-dir}}/{{mode}} && 7z a -r ../../tinyrasputin-{{mode}}.zip {{package-contents}}

# Build the environment then repackage
rebuild-environment: (build-environment) (package)

_package-exists:
    test -f tinyrasputin-{{mode}}.zip

_create-test-directory: (_package-exists)
    rm -rf ../server_tinyrasputin
    7z x -bb0 -y -o../server_tinyrasputin -- tinyrasputin-{{mode}}.zip > nul

# Test the built package of the specified mode
test-package: (_create-test-directory) (_package-exists)
    cd .. && {{python}} engine.py

# Create a dependency graph for a mode
dep-graph: (_build-dir-exists)
    cd {{build-dir}}/{{mode}} && cargo deps --all-deps | dot -Tpng > ../../graph-{{mode}}.png

# Count the number of lines of code in the project
sloc:
    @echo "`wc -l src/**/*.rs src/*.rs` lines of code"