# rusty-axml

[![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/rusty-rs/rusty-axml/ci.yml?branch=main&style=for-the-badge)](https://github.com/rusty-rs/rusty-axml/actions/workflows/ci.yml)
[![Crates.io Version](https://img.shields.io/crates/v/rusty-axml?style=for-the-badge)](https://crates.io/crates/rusty-axml)
[![docs.rs](https://img.shields.io/docsrs/rusty-axml?style=for-the-badge)](https://docs.rs/rusty-axml/latest/rusty_axml/)

Rust parser for Android binary XML files

## About

Every APK contains many XML files like its manifest, strings, layouts... All of
these files are stored in the Android binary XML format. This crate provides
the necessary logic to decode these files into human-readable XML.

## Current status

As of version 0.2.0 only the parsing of Android manifest files is supported. The
work to support any and all AXML files is ongoing.

## Usage

We provide both a library and a binary crate.

### Library

The easiest way to get started is to use the `parse_from_apk()` or
`parse_From_axml()` functions. As the names indicate you can pass an APK or an
AXML file to get an `Axml` object in return.

See the [documentation on docs.rs](https://docs.rs/rusty-axml/latest/rusty_axml/)
for details on how to use this library.

### Binary crate

The binary crate is not compiled by default. To compile it, run

```
cargo build --release --features=cli
```

Then use either `cargo run` or the newly built `axmlparser` binary to convert
AXML files. Here are the possible CLI arguments (also available using `-h` or
`--help`):

  * `-a, --apk <APK>`: path to an APK
  * `-x, --xml <XML>`: path to an Android binary XML file
  * `-o, --output <OUTPUT>`: path to the output file to write the decoded content

If `--output` is not specified, `axmlparser` will print the decoded XML to the
standard output.
