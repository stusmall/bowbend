# Building bowbend
Currently building is only tested and supported on Ubuntu Linux.

## cargo xtask
This project uses [cargo xtask](https://github.com/matklad/cargo-xtask) pattern for managing builds. Think of this 
as a way of writing makefiles in rust.  The goal is to contain as much logic around managing builds and lints as 
possible. This allows developers to easily reproduce actions from CI with minimal set up.

The commands current supported are:

* build - This will build the project. It builds the core project and all language specific bindings. For python the 
  resulting packages can be found in `target/wheels/`.  It can accept two different parameters:  release or asan. They 
  aren't mutually exclusive.
    * release - This produces a release build.
    * asan - This builds the project using [address sanitizer](https://clang.llvm.org/docs/AddressSanitizer.html). This
      is [an unstable rust feature](https://github.com/rust-lang/rust/issues/47174) so requires a nightly compiler. 
      Since this codebase relies heavily on FFI and low level networking there is a fair amount of unsafe code. 
      Address sanitizer is a must for projects to help validate unsafe code. These builds are used run through our 
      integrations tests in CI to help find soundness issues.
* clean - This cleans the project. It does what you expect it to.
* format - Confirms that all files match out formatting rules.  This will not automatically a fix errors, the 
  formatting tools should be invoked directly for that.
* lint - Runs a series of lints and type checks for all languages used in the project. It currently used clippy, mypy,
  and pylint
* spellcheck - This will spell check the workspace as best as it can.  Today it only supports spellchecking on Rust 
  doc strings.
* test - Runs all unit tests.  It does not try to run integration tests.


## Required tools

The best way to keep up to date on tool versions needed is to read the CI pipeline definition.  When installing 3rd 
party software we aim to be as explicit about versions as possible.  Documentation tends to get out of date, but we 
can ensure CI is always up-to-date and tested.

At a high level you will want to make sure you have:
* A modern stable rust compiler.  We don't set a `rust-toolchain` file to make it easier to work with newer 
  compilers to test new optimizations or make use of unstable testing tools.  The minimum supported rust version is 
  defined in the github action workflow and is always tested again.
* Python3 and maturin.  Maturin is our Python build tool.  We this will help us manage all our other python 
  dependencies for us.
* gcc.  It is used in the process of prepping generated header files for consumption by cffi.

## Quick start
* Create a new Python virtual environment: `python3 -m venv venv`
* Activate it: `source venv/bin/activate`
* Install cffi: `pip install cffi==1.15.1`
* Build the project: `cargo xtask build`

## Quick development options
When quickly iterating and testing changing, `maturin develop` is a useful tool.  It will rebuild the python wheel 
and install it *but* this tool requires a word of caution.  It will not rebuild the header file for the ffi crate.  
This stale data can cause surprising test failures, so proceed with caution with working in the FFI layer.