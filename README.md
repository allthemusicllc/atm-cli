# atm-cli

## Overview

`atm-cli` is a command line tool for generating and working with MIDI files. It was purpose-built for
All the Music, LLC to assist in its mission to enable musicians to make all of their music
without the fear of frivolous copyright lawsuits. All code is released into the public domain
via the [Creative Commons Attribute 4.0 International License](http://creativecommons.org/licenses/by/4.0/).
If you're looking for a Rust library to generate and work with MIDI files, check out
[the `libatm` project](https://github.com/allthemusicllc/libatm), on which this tool relies. For
more information on All the Music, check out [allthemusic.info](http://allthemusic.info).  For more detailed
information about the code, check out the crate documentation [here](https://allthemusicllc.github.io/atm-cli/atm/index.html).

## Installation

`atm-cli` is written in [Rust](https://www.rust-lang.org/), and thus requires the Rust toolchain to compile.  Follow the instructions at <https://www.rust-lang.org/tools/install> to install the toolchain.  Once that is complete, clone the repo and compile the tool:

```bash
$ git clone https://github.com/allthemusicllc/atm-cli.git
$ cd atm-cli
$ cargo build --release
$ cargo run --release -- -h # show usage
```

## Getting Started

To generate a single MIDI file from a melody, use the `single` directive:

```bash
atm single -n 'C:4,D:4,E:4,F:4,G:4,A:4,B:4,C:5' -t test.mid
```

To brute-force a range of melodies with a given length, generated from a given input note sequence, use the `batch` directive:

```bash
atm batch -n 'C:4,D:4,E:4,F:4,G:4,A:4,B:4,C:5' -L 8 -b 20 -p 2 -t C4_D4_E4_F4_G4_A4_B4_C5.tar
```

After brute-force generating a range of melodies with the `batch` command, lookup the output batch for note sequence with the `partition` directive:

```bash
atm partition -n 'C:4,C:4,C:4,C:4,C:4,C:4,C:4,C:5' -p 2
```

You can download an existing dataset containing all 10-note melodies containing the 8 major keys in the Middle C octave from:

https://allthemusic.s3.amazonaws.com/datasets/20190727.C4_D4_E4_F4_G4_A4_B4_C5.L10.tar

## Usage

```bash
atm 0.1.0
All The Music, LLC
Tools for generating and working with MIDI files.  This app was created as part of an effort to generate by brute-force
billions of melodies, and is tailored for that use case.

USAGE:
    atm [SUBCOMMAND]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    batch        Generate by brute-force MIDI files containing permutations of a sequence of MIDI pitches
    help         Prints this message or the help of the given subcommand(s)
    partition    Generate the output path from the 'batch' directive for a given MIDI pitch sequence
    single       Generate single MIDI file from provided MIDI pitch sequence
```
