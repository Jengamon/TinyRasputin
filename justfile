python := "python"
build-dir := "build"
mode := "debug"
package-targets := "PACKAGE_TARGETS_" + mode
package-contents := "vendor .cargo/config Cargo.* commands.json justfile .package-list src " + env_var(package-targets)

export RUST_BACKTRACE := "1"

alias rebuild := rebuild-environment
alias clean := clean-environment
alias env := build-environment
alias build := package-build
alias run := package-run

# Runs tinyrasputin in a certain mode
package-run +FLAGS='': (package-build)
    cd {{build-dir}}/{{mode}} && just -d . --justfile justfile run {{FLAGS}}

# Builds tinyrasputin in a certain mode
package-build: (_build-dir-exists) (_copy-files) (_generate-package-listing)
    cd {{build-dir}}/{{mode}} && just -d . --justfile justfile build

_make-build-dir: 
    mkdir -p {{build-dir}}/{{mode}}
    echo "Created environment for {{mode}} build."

@_build-dir-exists:
    test -d {{build-dir}}/{{mode}}

_copy-files: (_build-dir-exists) (_copy_base_files)
    @if [ ! -z {{env_var(package-targets)}} ]; then \
        echo 'Copying over extra target files for {{mode}} build'; \
        cp -r -t {{build-dir}}/{{mode}} {{env_var(package-targets)}}; \
    fi

_copy_base_files: (_build-dir-exists)
    cp -rt {{build-dir}}/{{mode}} src
    cp package-justfile {{build-dir}}/{{mode}}/justfile
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

_generate-package-listing: (_build-dir-exists) (_copy-files)
    rm -rf {{build-dir}}/{{mode}}/.package-list
    cd {{build-dir}}/{{mode}} && find {{package-contents}} -type f -print > .package-list

# Erase build artifacts for a selected mode
clean-environment:
    rm -rf {{build-dir}}/{{mode}}

# Erase all build artifacts
clean-all:
    rm -rf {{build-dir}}

# Build the build directory for a certain mode
build-environment: (_select-cargo) (_clean-vendor)
    rm -rf {{build-dir}}/{{mode}}/.cargo
    mkdir {{build-dir}}/{{mode}}/.cargo
    cd {{build-dir}}/{{mode}} && cargo update
    cd {{build-dir}}/{{mode}} && cargo vendor vendor > .cargo/config

_create_command_json +FLAGS='':
    sed -e "s/MODE/{{mode}}/g" -e "s/FLAGS/{{FLAGS}}/g" commands-template.json > {{build-dir}}/{{mode}}/commands.json

# Build the packge that we will upload to the server in the specified run mode
package +FLAGS='': (_clean-package) (package-build) (_create_command_json FLAGS) (_generate-package-listing)
    @echo 'Packing tinyrasputin-{{mode}}.zip...'
    cd {{build-dir}}/{{mode}} && 7z a -r ../../tinyrasputin-{{mode}}.zip {{package-contents}}

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
    cd {{build-dir}}/{{mode}} && cargo deps --all-deps | dot -Tpng > ../../graph-{{mode}}.png

# Count the number of lines of code in the project
sloc:
    @echo "`wc -l src/**/*.rs src/*.rs` lines of code"