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
alias build := package-build
alias run := package-run
alias test := package-test

# Runs tinyrasputin in a certain mode
package-run +FLAGS='': (package-build)
    cd {{build-dir}}/{{mode}} && just -d . --justfile justfile run {{FLAGS}}

# Builds tinyrasputin in a certain mode
package-build: (_build-dir-exists) (_copy-files) (_generate-package-listing)
    cd {{build-dir}}/{{mode}} && just -d . --justfile justfile build

package-test +FLAGS='': (package-build)
    cd {{build-dir}}/{{mode}} && just -d . --justfile justfile test {{FLAGS}}

_make-build-dir: 
    mkdir -p {{build-dir}}/{{mode}}
    echo "Created environment for {{mode}} build."

@_build-dir-exists:
    test -d {{build-dir}}/{{mode}}

@_vendor-exists: (_build-dir-exists)
    test -d {{build-dir}}/{{mode}}/vendor

_copy-files: (_build-dir-exists) (_copy-justfile)
    cp -rt {{build-dir}}/{{mode}} {{base-package}}

_copy-justfile: (_build-dir-exists)
    cp package-justfile {{build-dir}}/{{mode}}/justfile
    @echo 'Renewed basic build environment for {{mode}} build'

# Select a Cargo file based off of the desired mode
_select-cargo: (clean-environment) (_make-build-dir) (_copy-files)
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
build-environment: (_select-cargo) (_clean-vendor)
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