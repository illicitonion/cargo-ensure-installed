cargo-ensure-installed
======================

[![Build Status](https://travis-ci.org/illicitonion/cargo-ensure-installed.svg?branch=master)](https://travis-ci.org/illicitonion/cargo-ensure-installed)
[![Latest Version](https://img.shields.io/crates/v/cargo-ensure-installed.svg)](https://crates.io/crates/cargo-ensure-installed)

Like `cargo install` but if you already have a suitable version, simply leaves it as-is.

## Installation

`cargo install cargo-ensure-installed`

## Usage

`cargo ensure-install --package=rustfmt --version=0.9.0`

Version may be any version requirement understood by [SemVer](https://github.com/steveklabnik/semver).

## License

This project is licensed under Apache 2.
