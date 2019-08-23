# carguix

## Overview

This tool generates Guix package definition for Rust crates from `crates.io`.

The generated definitions have been tested with `guix` commit `439378fd9f`.

## Example

Here is the result of `carguix num-traits` (after formatting).

```scheme
(define-public rust-num-traits-0.2.8
  (package
    (name "rust-num-traits")
    (version "0.2.8")
    (source
      (origin
        (method url-fetch)
        (uri (crate-uri "num-traits" version))
        (file-name
          (string-append name "-" version ".tar.gz"))
        (sha256
          (base32
            "0clvrm34rrqc8p6gq5ps5fcgws3kgq5knh7nlqxf2ayarwks9abb"))))
    (build-system cargo-build-system)
    (arguments
      (list #:cargo-inputs
            (list (list "rust-autocfg-0.1.6" rust-autocfg-0.1.6))))
    (home-page #f)
    (synopsis #f)
    (description #f)
    (license #f)))

(define-public rust-autocfg-0.1.6
  (package
    (name "rust-autocfg")
    (version "0.1.6")
    (source
      (origin
        (method url-fetch)
        (uri (crate-uri "autocfg" version))
        (file-name
          (string-append name "-" version ".tar.gz"))
        (sha256
          (base32
            "0x8q946yy321rlpxhqf3mkd965x8kbjs2jwcw55dsmxlf7xwhwdn"))))
    (build-system cargo-build-system)
    (arguments (list #:cargo-inputs (list)))
    (home-page #f)
    (synopsis #f)
    (description #f)
    (license #f)))
```

## Prerequisites

You need `guix` to be available in your command line since this tool calls `guix hash`.

## Quickstart

Create the file `gnu/packages/rust-ripgrep.scm` with this content.

```scheme
(define-module
  (gnu packages rust-ripgrep)
  #:use-module ((guix licenses) #:prefix license:)
  #:use-module (gnu packages)
  #:use-module (guix packages)
  #:use-module (guix download)
  #:use-module (guix utils)
  #:use-module (guix build-system cargo))

```

Run the following command to populate the file with `ripgrep` and its dependencies definition.

```
RUST_LOG=carguix=info cargo run --release -- -u ripgrep >> gnu/packages/rust-ripgrep.scm
```

Build `ripgrep` with guix.

```
guix build rust-ripgrep
```

## Synopsis

```
carguix 0.1.0
Gérald Lelong <gerald.lelong@easymov.fr>
Generate Guix package definition for Rust crates

USAGE:
    carguix [FLAGS] [OPTIONS] <crate_name>

FLAGS:
    -h, --help      Prints help information
    -u, --update    Update crates.io index

OPTIONS:
    -v, --version <version>    Generate package definition for specific version of the crate (default: earliest)

ARGS:
    <crate_name>
```
