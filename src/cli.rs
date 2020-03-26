// cli.rs
//
// Copyright (c) 2020 All The Music, LLC
//
// This work is licensed under the Creative Commons Attribution 4.0 International License.
// To view a copy of this license, visit http://creativecommons.org/licenses/by/4.0/ or send
// a letter to Creative Commons, PO Box 1866, Mountain View, CA 94042, USA.

use std::str::FromStr;

/*******************************
***** Shared CLI Arguments *****
*******************************/

#[derive(structopt::StructOpt)]
pub struct NoteSetArg {
    #[structopt(
        help=concat!("Comma-separated list NOTE:OCTAVE pairs ",
                     "(i.e., 'C:4,D:4,E:4,F:4,G:4,A:4,B:4,C:5')"),
        parse(try_from_str = libatm::MIDINoteSet::from_str))]
    pub sequence: libatm::MIDINoteSet,
}

#[derive(structopt::StructOpt)]
pub struct TargetArg {
    #[structopt(
        help="File output path (directory must exist)",
        parse(from_str))]
    pub target: std::path::PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum ParseMaxFilesArgError {
    #[error(transparent)]
    NotFloat(#[from] std::num::ParseFloatError),
    #[error("Max files per directory must be between 1 and 4096, inclusive (found {input})")]
    ValueOutOfRange { input: f32, },
}

#[derive(structopt::StructOpt)]
pub struct MaxFilesArg {
    #[structopt(
        help="Maximum number of files per directory (default: 4096)",
        parse(try_from_str=MaxFilesArg::try_from_str))]
    pub max_files: f32,
}

impl MaxFilesArg {
    pub fn try_from_str(arg: &str) -> Result<f32, ParseMaxFilesArgError> {
        let max_files = arg.parse::<f32>()?;
        if max_files <= 0.0 || max_files > 4096.0 {
            return Err(ParseMaxFilesArgError::ValueOutOfRange { input: max_files });
        }
        Ok(max_files)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ParsePartitionDepthArgError {
    #[error(transparent)]
    NotInteger(#[from] std::num::ParseIntError),
    #[error("Partition depth must be between 0 and 4, inclusive (found {input})")]
    ValueOutOfRange { input: u32, },
}

#[derive(structopt::StructOpt)]
pub struct PartitionDepthArg {
    #[structopt(
        long = "partitions",
        help = concat!("Partition depth to use for output directory structure.  For",
                     "example, if set to 2 the ouput directory structure would look",
                     "like <part1>/<part2>/<hash>.mid"),
        parse(try_from_str=PartitionDepthArg::try_from_str))]
    pub partition_depth: u32, 
}

impl PartitionDepthArg {
    pub fn try_from_str(arg: &str) -> Result<u32, ParsePartitionDepthArgError> {
        let partition_depth = arg.parse::<u32>()?;
        if partition_depth == 0 || partition_depth > 4 {
            return Err(ParsePartitionDepthArgError::ValueOutOfRange { input: partition_depth });
        }
        Ok(partition_depth)
    }
}

/******************************
***** CLI Directive Trait *****
******************************/

pub trait CliDirective {
    fn run(self);
}

#[derive(structopt::StructOpt)]
#[structopt(
    about = concat!("Tools for generating and working with MIDI files. ",
                    "This app was created as part of an effort to generate ",
                    "by brute-force billions of melodies, and is tailored for that use case."),
    author = "All The Music, LLC",
    version = env!("CARGO_PKG_VERSION"),)]
pub enum Cli {
    Single(crate::directives::SingleDirective),
}

impl CliDirective for Cli {
    fn run(self) {
        match self {
            Self::Single(directive) => directive.run(),
        }
    }
}
