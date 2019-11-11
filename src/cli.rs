// cli.rs
//
// Copyright (c) 2019 All The Music, LLC
//
// This work is licensed under the Creative Commons Attribution 4.0 International License.
// To view a copy of this license, visit http://creativecommons.org/licenses/by/4.0/ or send
// a letter to Creative Commons, PO Box 1866, Mountain View, CA 94042, USA.

extern crate clap;

pub struct Cli<'a, 'b> {
    pub app: clap::App<'a, 'b>,
}

impl<'a, 'b> Cli<'a, 'b> {
    fn initialize_parser() -> clap::App<'a, 'b> {
        // Note list argument
        let note_sequence_argument = clap::Arg::with_name("NOTES")
            .short("n")
            .long("notes")
            .takes_value(true)
            .required(true)
            .help(
                "Comma-separated list NOTE:OCTAVE pairs (i.e., \
                 'C:4,D:4,E:4,F:4,G:4,A:4,B:4,C:5')",
            );
        // Target (output) file argument
        let target_argument = clap::Arg::with_name("TARGET")
            .short("t")
            .long("target")
            .takes_value(true)
            .required(true)
            .help("File output path (directory must exist)");
        // Directory partition depth argument
        let partition_depth_argument = clap::Arg::with_name("PARTITION_DEPTH")
            .short("p")
            .long("partitions")
            .takes_value(true)
            .required(true)
            .help(
                "Partition depth to use for output directory structure.  For \
                 example, if set to 2 the ouput directory structure would look \
                 like <part1>/<part2>/<hash>.mid.",
            );
        // Maximum files per directory argument
        let max_files_argument = clap::Arg::with_name("MAX_FILES")
            .short("m")
            .long("max-files")
            .takes_value(true)
            .help("Maximum number of files per directory (default: 4096)");
        // Sequence length argument
        let length_argument = clap::Arg::with_name("LENGTH")
            .short("L")
            .long("length")
            .takes_value(true)
            .required(true)
            .help("Length of MIDI pitch sequences to generate");
        // Maximum file count argument
        let max_count_argument = clap::Arg::with_name("COUNT")
            .short("c")
            .long("count")
            .takes_value(true)
            .help("Number of sequences to iterate through (default: NOTES.len() ^ LENGTH)");
        // Batch size argument
        let batch_size = clap::Arg::with_name("BATCH_SIZE")
            .short("b")
            .long("batch-size")
            .takes_value(true)
            .required(true)
            .help("Number of files to batch (and zip) per archive entry");
        // Command line app
        clap::App::new("atm")
            .version(env!("CARGO_PKG_VERSION"))
            .author("All The Music, LLC")
            .about(
                "Tools for generating and working with MIDI files.  \
                This app was created as part of an effort to generate \
                by brute-force billions of melodies, and is tailored for that use case."
            )
            .subcommand(clap::SubCommand::with_name("single")
                        .about("Generate single MIDI file from provided MIDI pitch sequence")
                        .arg(&note_sequence_argument)
                        .arg(&target_argument))
            .subcommand(clap::SubCommand::with_name("batch")
                        .about(
                            "Generate by brute-force MIDI files containing permutations \
                             of a sequence of MIDI pitches",
                        )
                        .arg(&batch_size)
                        .arg(&length_argument)
                        .arg(&max_count_argument)
                        .arg(&max_files_argument)
                        .arg(&note_sequence_argument)
                        .arg(&partition_depth_argument)
                        .arg(clap::Arg::with_name("PB_UPDATE")
                            .short("u")
                            .long("update")
                            .takes_value(true)
                            .help("Refresh rate for the progress bar (default: 1000 ms)"))
                        .arg(&target_argument))
            .subcommand(clap::SubCommand::with_name("partition")
                        .about("Generate the output path from the 'batch' directive for a given MIDI pitch sequence")
                        .arg(&max_files_argument)
                        .arg(&note_sequence_argument)
                        .arg(&partition_depth_argument))
            .subcommand(clap::SubCommand::with_name("split")
                        .about("Split tar archive into equal-sized chunks")
                        .arg(clap::Arg::with_name("CHUNK_SIZE")
                            .short("c")
                            .long("chunk-size")
                            .takes_value(true)
                            .help("Approximate size of output chunks in bytes (incompatible with NUM_CHUNKS)"))
                        .arg(clap::Arg::with_name("NUM_CHUNKS")
                            .short("n")
                            .long("num-chunks")
                            .takes_value(true)
                            .help("Number of ouptut chunks (incompatible with CHUNK_SIZE)"))
                        .arg(clap::Arg::with_name("PREFIX")
                            .short("p")
                            .long("prefix")
                            .takes_value(true)
                            .help("Prefix to apply to filename of each output chunk (default: 'split')"))
                        .arg(clap::Arg::with_name("SOURCE")
                            .short("s")
                            .long("source")
                            .takes_value(true)
                            .required(true)
                            .help("Path to source TAR archive"))
                        .arg(&target_argument))
    }

    pub fn new() -> Cli<'a, 'b> {
        Cli {
            app: Cli::initialize_parser(),
        }
    }

    pub fn run(self) {
        let matches = self.app.get_matches();
        match matches.subcommand_name() {
            Some("single") => {
                crate::directives::atm_single(crate::directives::SingleDirectiveArgs::from(
                    matches.subcommand_matches("single").unwrap(),
                ))
            },
            Some("batch") => {
                crate::directives::atm_batch(crate::directives::BatchDirectiveArgs::from(
                    matches.subcommand_matches("batch").unwrap(),
                ))
            },
            Some("partition") => {
                crate::directives::atm_partition(crate::directives::PartitionDirectiveArgs::from(
                    matches.subcommand_matches("partition").unwrap(),
                ))
            },
            Some("split") => {
                crate::directives::atm_split(crate::directives::SplitDirectiveArgs::from(
                    matches.subcommand_matches("split").unwrap(),
                ))
            },
            Some(directive) => panic!(format!("Received unsupported directive '{}'", directive)),
            None => panic!("Did not receive directive"),
        }
    }
}
