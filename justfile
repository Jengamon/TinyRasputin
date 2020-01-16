python := "python"
build-dir := "build"
mode := "debug"
package_targets := "PACKAGE_TARGETS_" + mode

export RUST_BACKTRACE := "1"

alias rebuild := rebuild-environment
alias clean := clean-environment
alias env := build-environment

# Runs tinyrasputin in a certain mode
package-run +FLAGS='': (package-build)
    #!/usr/bin/env sh
    cd {{build-dir}}/{{mode}}
    if [ "{{mode}}" = "release" ]; then
        cargo run --offline --frozen --release -q -- {{FLAGS}}
    else
        cargo run --offline --frozen -- {{FLAGS}}
    fi

# Builds tinyrasputin in a certain mode
package-build: (_build-dir-exists) (_copy-files)
    #!/usr/bin/env sh
    cd {{build-dir}}/{{mode}}
    echo 'Building on {{arch()}}:{{os()}}'
    if [ "{{mode}}" = "release" ]; then
        cargo build --offline --frozen -q --release
    else
        cargo build --offline --frozen
    fi

# Run using a specified local mode (can only be called on built packages)
run +FLAGS='': (build)
    #!/usr/bin/env sh
    if [ "{{mode}}" = "release" ]; then
        cargo run --offline --frozen --release -q -- {{FLAGS}}
    else
        cargo run --offline --frozen -- {{FLAGS}}
    fi

# Build using specified local mode (can only be called on built packages)
build: (_package-complete)
    #!/usr/bin/env sh
    if [ "{{mode}}" = "release" ]; then
        cargo build --offline --frozen -q --release
    else
        cargo build --offline --frozen
    fi

_make-build-dir: 
    mkdir -p {{build-dir}}/{{mode}}
    echo "Created environment for {{mode}} build."

@_build-dir-exists:
    test -d {{build-dir}}/{{mode}}

_copy-files: (_build-dir-exists) (_copy_base_files)
    @if [ ! -z {{env_var(package_targets)}} ]; then \
        echo 'Copying over extra target files for {{mode}} build'; \
        cp -r -t {{build-dir}}/{{mode}} {{env_var(package_targets)}}; \
    fi

_copy_base_files: (_build-dir-exists)
    cp -rt {{build-dir}}/{{mode}} src justfile .env
    @echo 'Renewed basic build environment for {{mode}} build'

# Select a Cargo file based off of the desired mode
_select-cargo: (clean-environment)  (_make-build-dir) (_copy_base_files) (_copy-files)
    rm -rf {{build-dir}}/{{mode}}/Cargo.toml
    cat Cargo-header.toml Cargo-{{mode}}.toml  > {{build-dir}}/{{mode}}/Cargo.toml
    @echo "Created Cargo.toml for {{mode}} build."

_clean-package:
    rm -f tinyrasputin-{{mode}}.zip

_clean-vendor:
    rm -rf {{build-dir}}/{{mode}}/vendor

# Checks if a package is theoretically complete
@_package-complete:
    test -d vendor
    test -f .cargo/config
    test -f Cargo.toml
    test -f Cargo.lock
    test -f commands.json
    if [ {{mode}} = "debug" ]; then \
        echo 'Base package coherent, checking for extra files...'; \
    fi
    for file in {{env_var(package_targets)}}; do \
        if [ {{mode}} = "debug" ]; then \
            echo Checking for $file...; \
        fi; \
        test -e "./$file"; \
    done

# Erase build artifacts for a selected mode
clean-environment:
    rm -rf {{build-dir}}/{{mode}}

# Erase all build artifacts
clean-all:
    rm -rf {{build-dir}}

# Build the build directory for a certain mode
build-environment: (_select-cargo) (_clean-vendor)
    #!/usr/bin/env sh
    rm -rf {{build-dir}}/{{mode}}/.cargo
    mkdir {{build-dir}}/{{mode}}/.cargo
    cd {{build-dir}}/{{mode}}
    cargo update
    cargo vendor vendor > .cargo/config

_create_command_json +FLAGS='':
    sed -e "s/MODE/{{mode}}/g" -e "s/FLAGS/{{FLAGS}}/g" commands-template.json > {{build-dir}}/{{mode}}/commands.json

# Build the packge that we will upload to the server in the specified run mode
package +FLAGS='': (_clean-package) (package-build) (_create_command_json FLAGS)
    echo 'Packing tinyrasputin-{{mode}}.zip...'
    cd {{build-dir}}/{{mode}} && 7z a -r ../../tinyrasputin-{{mode}}.zip `echo ".cargo justfile src Cargo.* commands.json .env vendor {{env_var(package_targets)}}"`

# Build the environment then repackage
rebuild-environment: (build-environment) (package)

@_package-exists:
    test -f tinyrasputin-{{mode}}.zip

_create-test-directory: (_package-exists)
    rm -rf ../server_tinyrasputin
    7z x -bb0 -y -o../server_tinyrasputin -- tinyrasputin-{{mode}}.zip > nul

# Test the built package of the specified mode
test-package: (_create-test-directory) (_package-exists)
    cd .. && {{python}} engine.py

# Create a dependency graph for a mode
dep-graph: (_build-dir-exists)
    #!/usr/bin/env sh
    cd {{build-dir}}/{{mode}}
    cargo deps --all-deps | dot -Tpng > ../../graph-{{mode}}.png

# Count the number of lines of code in the project
sloc:
    @echo "`wc -l src/**/*.rs src/*.rs` lines of code"