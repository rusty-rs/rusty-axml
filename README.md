# AXMLParser

[![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/rusty-rs/rusty-axml/ci.yml?branch=main&style=for-the-badge)](https://github.com/rusty-rs/rusty-axml/actions/workflows/ci.yml)
[![Crates.io Version](https://img.shields.io/crates/v/rusty-axml?style=for-the-badge)](https://crates.io/crates/rusty-axml)
[![docs.rs](https://img.shields.io/docsrs/rusty-axml?style=for-the-badge)](https://docs.rs/rusty-axml/latest/rusty_axml/)

Every APK has a manifest file, which is usually in binary format. This project
decodes this manifest into human-readable XML.

### Usage

```
./AXMLParser [AXML|APK]
```

The argument can be either the manifest directly (in binary format) or an APK
file, in which case the manifest will first be extracted from the APK.

### To do

- when extracting from an APK, also decode other resources (e.g.,
  `strings.xml`) which would allow us to resolve some static references.
- when printing decoded XML to `stdout`, pretty-print it instead of just
  dumping it on one line.
