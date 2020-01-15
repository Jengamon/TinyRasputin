python := "python"
build-dir := "build"
export RUST_BACKTRACE := "1"

alias rebuild := rebuild-package

# Run using a specified local mode (can only be called on built packages)
run mode +FLAGS='': (build mode)
    #!/usr/bin/env sh
    if [ "{{mode}}" = "release" ]; then
        cargo run --offline --frozen --release -q -- {{FLAGS}}
    else
        cargo run --offline --frozen -- {{FLAGS}}
    fi

# Build using specified local mode (can only be called on built packages)
build mode: (_package-complete mode)
    #!/usr/bin/env sh
    if [ "{{mode}}" = "release" ]; then
        cargo build --offline --frozen -q --release
    else
        cargo build --offline --frozen
    fi

# Runs tinyrasputin in a certain mode
package-run mode +FLAGS='': (package-build mode)
    #!/usr/bin/env sh
    cd {{build-dir}}/{{mode}}
    if [ "{{mode}}" = "release" ]; then
        cargo run --offline --frozen --release -q -- {{FLAGS}}
    else
        cargo run --offline --frozen -- {{FLAGS}}
    fi

# Builds tinyrasputin in a certain mode
package-build mode: (_build-dir-exists mode) (_copy-files mode) (_package-vendor-exists mode)
    #!/usr/bin/env sh
    cd {{build-dir}}/{{mode}}
    if [ "{{mode}}" = "release" ]; then
        cargo build --offline --frozen -q --release
    else
        cargo build --offline --frozen
    fi

@_make-build-dir mode: 
    mkdir -p {{build-dir}}/{{mode}}
    echo "Created environment for {{mode}} build."

@_build-dir-exists mode:
    test -d {{build-dir}}/{{mode}}

_copy-files mode: (_build-dir-exists mode) (_copy_base_files mode) (_create_command_json mode)
    #!/usr/bin/env sh
    echo 'Copying over extra target files for {{mode}} build'
    for target in $PACKAGE_TARGETS_{{mode}}; do
        echo Copying $target to {{build-dir}}/{{mode}};
        cp -r $target {{build-dir}}/{{mode}};
    done

@_copy_base_files mode: (_build-dir-exists mode)
    cp -r src {{build-dir}}/{{mode}}/src
    cp justfile {{build-dir}}/{{mode}}/justfile
    cp .env {{build-dir}}/{{mode}}/.env
    echo 'Renewed basic build environment for {{mode}} build'

# Select a Cargo file based off of the desired mode
@_select-cargo mode: (clean-target mode)  (_make-build-dir mode) (_copy_base_files mode)
    rm -rf {{build-dir}}/{{mode}}/Cargo.toml
    cat Cargo-header.toml Cargo-{{mode}}.toml  > {{build-dir}}/{{mode}}/Cargo.toml
    echo "Created Cargo.toml for {{mode}} build."

@_clean-package mode:
    rm -f tinyrasputin-{{mode}}.zip

@_clean-vendor mode:
    rm -rf {{build-dir}}/{{mode}}/vendor

# Checks if a package is theoretically complete
@_package-complete mode:
    test -d vendor
    test -f .cargo/config
    test -f Cargo.toml
    test -f Cargo.lock
    test -f commands.json
    if [ {{mode}} = "debug" ]; then \
        echo 'Base package coherent, checking for extra files...'; \
    fi
    for file in $PACKAGE_TARGETS_{{mode}}; do \
        if [ {{mode}} = "debug" ]; then \
            echo Checking for $file...; \
        fi; \
        test -e $file; \
    done

@_package-vendor-exists mode:
    test -d {{build-dir}}/{{mode}}/vendor
    test -f {{build-dir}}/{{mode}}/.cargo/config

# Erase build artifacts for a selected mode
@clean-target mode:
    rm -rf {{build-dir}}/{{mode}}

# Erase all build artifacts
@clean-all:
    rm -rf {{build-dir}}

# Build the build directory for a certain mode
build-environment mode: (_select-cargo mode) (_clean-vendor mode)
    #!/usr/bin/env sh
    rm -rf {{build-dir}}/{{mode}}/.cargo
    mkdir {{build-dir}}/{{mode}}/.cargo
    cd {{build-dir}}/{{mode}}
    cargo update
    cargo vendor vendor > .cargo/config

@_create_command_json mode:
    sed -e "s/MODE/{{mode}}/g" commands-template.json > {{build-dir}}/{{mode}}/commands.json

# Build the packge that we will upload to the server in the specified run mode
package mode: (_clean-package mode) (package-build mode)
    #!/usr/bin/env sh
    echo 'Packing tinyrasputin-{{mode}}.zip...'
    cd {{build-dir}}/{{mode}}
    for target in .cargo justfile src Cargo.* commands.json .env $PACKAGE_TARGETS_{{mode}} vendor; do
        echo Zipping {{build-dir}}/{{mode}}/$target...;
        7z a -r ../../tinyrasputin-{{mode}}.zip $target > nul;
    done

rebuild-package mode: (build-environment mode) (package mode)

@_package-exists mode:
    test -f tinyrasputin-{{mode}}.zip

_create-test-directory mode: (_package-exists mode) (package-build mode)
    rm -rf ../server_tinyrasputin
    7z x -bb0 -y -o../server_tinyrasputin -- tinyrasputin-{{mode}}.zip > nul

# Test the built package of the specified mode
test-package mode: (_create-test-directory mode) (_package-exists mode)
    cd .. && {{python}} engine.py

# Create a dependency graph for a mode
dep-graph mode: (_package-vendor-exists mode)
    #!/usr/bin/env
    cd {{build-dir}}/{{mode}}
    test -d .cargo
    cargo deps --all-deps | dot -Tpng > graph-{{mode}}.png

# Count the number of lines of code in the project
sloc:
    @echo "`wc -l src/**/*.rs src/*.rs` lines of code"