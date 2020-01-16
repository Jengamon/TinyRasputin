python := "python"
build-dir := "build"

# Run mode tells the build system about the flags to enable for the package and changes the cargo profile
# Package mode stitches together the correct Cargo.toml from our files and puts it in the proper directory
package-mode := "debug"
run-mode := package-mode

package-targets := "PACKAGE_TARGETS_" + package-mode
base-package := "src " + env_var(package-targets)
package-contents := "vendor .cargo/config Cargo.* commands.json .package-list justfile " + base-package

export RUST_BACKTRACE := "1"

alias clean := clean-environment
alias env := build-environment
alias build := local-build
alias run := local-run
alias test := local-test

# Runs tinyrasputin in a certain mode locally
local-run +FLAGS='': (local-build)
    if [ {{run-mode}} = release ]; then \
        cargo run --offline --release --manifest-path {{build-dir}}/{{run-mode}}/Cargo.toml -- {{FLAGS}}; \
    else \
        cargo run --offline --manifest-path {{build-dir}}/{{run-mode}}/Cargo.toml -- {{FLAGS}}; \
    fi

# Builds tinyrasputin in a certain mode locally
local-build: (_copy-files run-mode)
    if [ {{run-mode}} = release ]; then \
        cargo build --offline --release --manifest-path {{build-dir}}/{{run-mode}}/Cargo.toml; \
    else \
        cargo build --offline --manifest-path {{build-dir}}/{{run-mode}}/Cargo.toml; \
    fi

# Tests tinyrasputin in a certain mode locally
local-test +FLAGS='': (local-build)
    if [ {{run-mode}} = release ]; then \
        cargo test --offline --release --manifest-path {{build-dir}}/{{run-mode}}/Cargo.toml -- {{FLAGS}}; \
    else \
        cargo test --offline --manifest-path {{build-dir}}/{{run-mode}}/Cargo.toml -- {{FLAGS}}; \
    fi

# Builds tinyrasputin in a certain package-mode
package-build: (_select-cargo package-mode) (_copy-files package-mode) (_generate-package-listing package-mode)
    cd {{build-dir}}/{{package-mode}} && just -d . --justfile justfile mode={{run-mode}} build

# Tests tiny rasputin in a certain package-mode as it would run in package package-mode
package-test +FLAGS='': (package-build)
    cd {{build-dir}}/{{package-mode}} && just -d . --justfile justfile mode={{run-mode}} test {{FLAGS}}

_make-build-dir mode: 
    mkdir -p {{build-dir}}/{{mode}}
    echo "Created environment for {{package-mode}} build."

_build-dir-exists mode:
    test -d {{build-dir}}/{{mode}}

_vendor-exists mode: (_build-dir-exists mode)
    test -d {{build-dir}}/{{mode}}/vendor

_copy-files mode: (_build-dir-exists mode)
    cp -rt {{build-dir}}/{{mode}} {{base-package}}
    cp package-justfile {{build-dir}}/{{mode}}/justfile
    @echo 'Renewed basic build environment for {{mode}} build'

# Select a Cargo file based off of the desired package-mode
_select-cargo mode: (_build-dir-exists mode) (_copy-files mode)
    rm -rf {{build-dir}}/{{mode}}/Cargo.toml
    cat Cargo-header.toml Cargo-{{mode}}.toml  > {{build-dir}}/{{mode}}/Cargo.toml
    @echo "Created Cargo.toml for {{mode}} build."

_clean-package mode:
    rm -f tinyrasputin-{{mode}}.zip

_clean-vendor mode:
    rm -rf {{build-dir}}/{{mode}}/vendor

_generate-package-listing mode: (_vendor-exists mode) (_copy-files mode)
    rm -rf {{build-dir}}/{{mode}}/.package-list
    cd {{build-dir}}/{{mode}} && find {{package-contents}} -type f -print > .package-list

# Erase build artifacts for a selected package-mode
clean-environment mode: (_clean-package mode)
    rm -rf {{build-dir}}/{{mode}}

# Erase all build artifacts
clean-all: (_clean-package "debug") (_clean-package "release")
    rm -rf {{build-dir}}
    rm -rf target

# Build the build directory for a certain package-mode
build-environment: (clean-environment package-mode) (_make-build-dir package-mode) (_select-cargo package-mode) (_clean-vendor package-mode)
    rm -rf {{build-dir}}/{{package-mode}}/.cargo
    mkdir {{build-dir}}/{{package-mode}}/.cargo
    cd {{build-dir}}/{{package-mode}} && cargo update
    cd {{build-dir}}/{{package-mode}} && cargo vendor vendor > .cargo/config

_create-command-json mode +FLAGS='': (_build-dir-exists mode)
    sed -e "s/MODE/{{package-mode}}/g" -e "s/FLAGS/{{FLAGS}}/g" commands-template.json > {{build-dir}}/{{package-mode}}/commands.json

# Build the packge that we will upload to the server in the specified run package-mode
package +FLAGS='': (_clean-package package-mode) (_create-command-json package-mode FLAGS) (package-build) (_generate-package-listing package-mode)
    @echo 'Packing tinyrasputin-{{package-mode}}.zip...'
    cd {{build-dir}}/{{package-mode}} && 7z a -r ../../tinyrasputin-{{package-mode}}.zip {{package-contents}}

# Build the environment then repackage
repackage: (build-environment) (package)

_package-exists mode:
    test -f tinyrasputin-{{mode}}.zip

_create-test-directory mode: (_package-exists mode)
    rm -rf ../server_tinyrasputin
    7z x -bb0 -y -o../server_tinyrasputin -- tinyrasputin-{{mode}}.zip > nul

# Test the built package of the specified package-mode
test-package mode: (_create-test-directory mode) (_package-exists mode)
    cd .. && {{python}} engine.py

# Create a dependency graph for a package-mode
dep-graph mode: (_build-dir-exists mode)
    cd {{build-dir}}/{{mode}} && cargo deps --all-deps | dot -Tpng > ../../graph-{{package-mode}}.png

# Count the number of lines of code in the project
sloc:
    @echo "`wc -l src/**/*.rs src/*.rs` lines of code"