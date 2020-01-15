# TinyRasputin

TinyRasputin is a poker playing bot meant for MIT Pokerbots 2020, so it is explicitly designed to work with their engine.


Our project uses "Just" to build, so it is relatively simple to understand.
To use the project, you just need to understand 2 of the build targets:

- build-vendor {debug|release}
- package {debug|release}
- run {debug|release}

You have to run the commands in that order and with the same mode.

Of course, you can run build-vendor on one machine, package it using the package command, and should be able to extract it and run it on another machine, which is my use case, and why I do it this way.

The reason that it is done in this way is because generating the vendor directory is non-trivial, so we just want to make as small a package as possible. The release package is always built with debug_assertions off, and the debug package is always built with it on, so that is how conditional compilation is done from the same codebase.

To actually generate vendor-{debug|release} we select the appropriate
Cargo-{debug|release} and add it to Cargo-header.toml, so that any
shared tables between the files stay shared. We then proceed as normal.


Inside .env is a list of all files to be packed alongside vendor-{debug|release} as variables named PACKAGE_TARGETS_{debug|release}.