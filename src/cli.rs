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

macro_rules! impl_deref {
    ($struct:ty, $field:ident, $target:ty) => {
        impl std::ops::Deref for $struct {
            type Target = $target;

            fn deref(&self) -> &Self::Target {
                &self.$field
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ParseBatchSizeArgError {
    #[error(transparent)]
    NotInteger(#[from] std::num::ParseIntError),
    #[error("Batch size must be greater than 0")]
    InvalidBatchSize,
}

fn try_batch_from_str(arg: &str) -> Result<u32, ParseBatchSizeArgError> {
    let batch_size = arg.parse::<u32>()?;
    if batch_size == 0 {
        return Err(ParseBatchSizeArgError::InvalidBatchSize);
    }
    Ok(batch_size)
}

#[derive(structopt::StructOpt)]
pub struct BatchSize {
    #[structopt(
        short="s",
        long="batch-size",
        default_value="25",
        help="Number of melodies per batch",
        parse(try_from_str=try_batch_from_str))]
    pub batch_size: u32,
}

impl_deref! { BatchSize, batch_size, u32 }

#[derive(Debug, thiserror::Error)]
pub enum ParseMelodyLengthArgError {
    #[error(transparent)]
    NotInteger(#[from] std::num::ParseIntError),
    #[error("Length must be greater than 0")]
    InvalidLength,
}

fn try_length_from_str(arg: &str) -> Result<u32, ParseMelodyLengthArgError> {
    let length = arg.parse::<u32>()?;
    if length == 0 {
        return Err(ParseMelodyLengthArgError::InvalidLength);
    }
    Ok(length)
}

#[derive(structopt::StructOpt)]
pub struct MelodyLengthArg {
    #[structopt(
        help="Length of melodies (pitch sequences) to generate",
        parse(try_from_str=try_length_from_str))]
    pub melody_length: u32,
}

impl_deref! { MelodyLengthArg, melody_length, u32 }

#[derive(structopt::StructOpt)]
pub struct NoteSetArg {
    #[structopt(
        value_name="notes",
        help=concat!("Comma-separated set of NOTE:OCTAVE pairs ",
                     "(i.e., 'C:4,D:4,E:4,F:4,G:4,A:4,B:4,C:5')"),
        parse(try_from_str = libatm::MIDINoteSet::from_str))]
    pub note_set: libatm::MIDINoteSet,
}

impl_deref! { NoteSetArg, note_set, libatm::MIDINoteSet }

#[derive(structopt::StructOpt)]
pub struct NoteVecArg {
    #[structopt(
        value_name="notes",
        help=concat!("Comma-separated sequence of NOTE:OCTAVE pairs ",
                     "(i.e., 'C:4,D:4,E:4,F:4,G:4,A:4,B:4,C:5')"),
        parse(try_from_str = libatm::MIDINoteVec::from_str))]
    pub note_vec: libatm::MIDINoteVec,
}

impl_deref! { NoteVecArg, note_vec, libatm::MIDINoteVec }

#[derive(Debug, thiserror::Error)]
pub enum ParseMaxFilesArgError {
    #[error(transparent)]
    NotFloat(#[from] std::num::ParseIntError),
    #[error("Max files per directory must be between 1 and 4096, inclusive (found {input})")]
    ValueOutOfRange { input: u32, },
}

fn try_maxf_from_str(arg: &str) -> Result<f32, ParseMaxFilesArgError> {
    let max_files = arg.parse::<u32>()?;
    if max_files <= 0 || max_files > 4096 {
        return Err(ParseMaxFilesArgError::ValueOutOfRange { input: max_files });
    }
    Ok(max_files as f32)
}

#[derive(Debug, thiserror::Error)]
pub enum ParsePartitionDepthArgError {
    #[error(transparent)]
    NotInteger(#[from] std::num::ParseIntError),
    #[error("Partition depth must be between 0 and 4, inclusive (found {input})")]
    ValueOutOfRange { input: u32, },
}

fn try_pdepth_from_str(arg: &str) -> Result<u32, ParsePartitionDepthArgError> {
    let partition_depth = arg.parse::<u32>()?;
    if partition_depth == 0 || partition_depth > 4 {
        return Err(ParsePartitionDepthArgError::ValueOutOfRange { input: partition_depth });
    }
    Ok(partition_depth)
}

#[derive(structopt::StructOpt)]
pub struct PartitionArgs {
    #[structopt(
        short,
        long,
        default_value="4096",
        help="Maximum number of files per directory",
        parse(try_from_str=try_maxf_from_str))]
    pub max_files: f32,
    #[structopt(
        short="p",
        long = "partitions",
        required=true,
        help = concat!("Partition depth to use for output directory structure.  For ",
                     "example, if set to 2 the ouput directory structure would look ",
                     "like <root>/<branch>/<hash>.mid"),
        parse(try_from_str=try_pdepth_from_str))]
    pub partition_depth: u32, 
}

#[derive(structopt::StructOpt)]
pub struct TargetArg {
    #[structopt(
        help="File output path (directory must exist)",
        parse(from_str))]
    pub target: std::path::PathBuf,
}

impl_deref! { TargetArg, target, std::path::PathBuf }

/******************************
***** CLI Directive Trait *****
******************************/

/// Trait to implement command line directive. Typical implementation
/// will parse the user-provided command line arguments (if any) and run
/// a command or set of commands.
pub trait CliDirective {
    fn run(self);
}

/**************
***** CLI *****
**************/

#[derive(structopt::StructOpt)]
#[structopt(
    about = concat!("Tools for generating and working with MIDI files. ",
                    "This app was created as part of an effort to generate ",
                    "by brute-force billions of melodies, and is tailored for that use case."),
    author = "All The Music, LLC",
    version = env!("CARGO_PKG_VERSION"),)]
pub enum Cli {
    Gen(crate::directives::GenDirective),
}

impl CliDirective for Cli {
    fn run(self) {
        match self {
            Self::Gen(d) => d.run(),
        }
    }
}
