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
pub struct NoteSequenceArg {
    #[structopt(
        help=concat!("Comma-separated list NOTE:OCTAVE pairs ",
                     "(i.e., 'C:4,D:4,E:4,F:4,G:4,A:4,B:4,C:5')"),
        parse(try_from_str = try_sequence_from_str))]
    pub sequence: libatm::MIDINoteSequence,
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
    #[error("Max files must be between 1 and 4096, inclusive (found {input})")]
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
                     "like <part1>/<part2>/<hash>.mid."),
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
    Single(crate::directives::SingleDirectiveArgs),
}

// pub struct Cli<'a, 'b> {
//     pub app: clap::App<'a, 'b>,
// }
// 
// impl<'a, 'b> Cli<'a, 'b> {
//     fn initialize_parser() -> clap::App<'a, 'b> {
//         // Sequence length argument
//         let length_argument = clap::Arg::with_name("LENGTH")
//             .short("L")
//             .long("length")
//             .takes_value(true)
//             .required(true)
//             .help("Length of MIDI pitch sequences to generate");
//         // Maximum file count argument
//         let max_count_argument = clap::Arg::with_name("COUNT")
//             .short("c")
//             .long("count")
//             .takes_value(true)
//             .help("Number of sequences to iterate through (default: NOTES.len() ^ LENGTH)");
//         // Batch size argument
//         let batch_size = clap::Arg::with_name("BATCH_SIZE")
//             .short("b")
//             .long("batch-size")
//             .takes_value(true)
//             .required(true)
//             .help("Number of files to batch (and zip) per archive entry");
//         // Command line app
//         clap::App::new("atm")
//             .version(env!("CARGO_PKG_VERSION"))
//             .author("All The Music, LLC")
//             .about(
//                 "Tools for generating and working with MIDI files.  \
//                 This app was created as part of an effort to generate \
//                 by brute-force billions of melodies, and is tailored for that use case."
//             )
//             .subcommand(clap::SubCommand::with_name("single")
//                         .about("Generate single MIDI file from provided MIDI pitch sequence")
//                         .arg(&note_sequence_argument)
//                         .arg(&target_argument))
//             .subcommand(clap::SubCommand::with_name("batch")
//                         .about(
//                             "Generate by brute-force MIDI files containing permutations \
//                              of a sequence of MIDI pitches",
//                         )
//                         .arg(&batch_size)
//                         .arg(&length_argument)
//                         .arg(&max_count_argument)
//                         .arg(&max_files_argument)
//                         .arg(&note_sequence_argument)
//                         .arg(&partition_depth_argument)
//                         .arg(clap::Arg::with_name("PB_UPDATE")
//                             .short("u")
//                             .long("update")
//                             .takes_value(true)
//                             .help("Refresh rate for the progress bar (default: 1000 ms)"))
//                         .arg(&target_argument))
//             .subcommand(clap::SubCommand::with_name("partition")
//                         .about("Generate the output path from the 'batch' directive for a given MIDI pitch sequence")
//                         .arg(&max_files_argument)
//                         .arg(&note_sequence_argument)
//                         .arg(&partition_depth_argument))
//     }
// 
//     pub fn new() -> Cli<'a, 'b> {
//         Cli {
//             app: Cli::initialize_parser(),
//         }
//     }
// 
//     pub fn run(self) {
//         let matches = self.app.get_matches();
//         match matches.subcommand_name() {
//             Some("single") => {
//                 crate::directives::atm_single(crate::directives::SingleDirectiveArgs::from(
//                     matches.subcommand_matches("single").unwrap(),
//                 ))
//             },
//             Some("batch") => {
//                 crate::directives::atm_batch(crate::directives::BatchDirectiveArgs::from(
//                     matches.subcommand_matches("batch").unwrap(),
//                 ))
//             },
//             Some("partition") => {
//                 crate::directives::atm_partition(crate::directives::PartitionDirectiveArgs::from(
//                     matches.subcommand_matches("partition").unwrap(),
//                 ))
//             },
//             Some(directive) => panic!(format!("Received unsupported directive '{}'", directive)),
//             None => panic!("Did not receive directive"),
//         }
//     }
// }
