// gen_tar_gz.rs
//
// Copyright (c) 2020 All The Music, LLC
//
// This work is licensed under the Creative Commons Attribution 4.0 International License.
// To view a copy of this license, visit http://creativecommons.org/licenses/by/4.0/ or send
// a letter to Creative Commons, PO Box 1866, Mountain View, CA 94042, USA.

use flate2::Compression;

use crate::{
    cli::CliDirective,
    directives::gen::{
        try_compression_from_str,
        write_melodies_to_backend,
    },
};

/****************************
***** GenTarGzDirective *****
****************************/

/// Generate melodies and store them in Gzip-compressed Tar file
/// (see: [TarGzFile](../storage/tar_gz_file/struct.TarGzFile.html))
#[derive(structopt::StructOpt)]
pub struct GenTarGzDirective {
    #[structopt(flatten)]
    pub note_set: crate::cli::NoteSetArg,
    #[structopt(flatten)]
    pub melody_length: crate::cli::MelodyLengthArg,
    #[structopt(flatten)]
    pub target: crate::cli::TargetArg,
    #[structopt(
        short="C",
        long="compress",
        help="Compression level [0-9, default: 6]",
        parse(try_from_str = try_compression_from_str))]
    pub compression_level: Option<Compression>,
    #[structopt(flatten)]
    pub partition_args: crate::cli::PartitionArgs,
}

impl CliDirective for GenTarGzDirective {
    fn run(self) {
        let note_set: libatm::MIDINoteSet = self.note_set.into();
        let melody_length = self.melody_length.into();
        let target: std::path::PathBuf = self.target.into();

        match self.partition_args.partition_depth {
            // Use partitioning scheme
            Some(partition_depth) => {
                // Create path generator
                let path_generator = crate::storage::PartitionPathGenerator::new(  
                    note_set.len() as f32,
                    melody_length as i32,
                    self.partition_args.max_files.into(),
                    partition_depth,
                ).unwrap_or_else(|err| {
                    println!("::: ERROR: Failed to initialize partitioning scheme ({:?})", err);
                    std::process::exit(1);
                });
                // Create storage backend
                let backend = crate::storage::TarGzFile::new(
                    target,
                    path_generator,
                    self.compression_level
                ).unwrap_or_else(|err| { 
                    println!("::: ERROR: Failed to create storage backend ({:?})", err);
                    std::process::exit(1);
                });
                // Write generated melodies to backend
                write_melodies_to_backend(note_set, melody_length, backend);
            },
            None => {
                // Create storage backend
                let backend = crate::storage::TarGzFile::new(
                    target,
                    crate::storage::MIDIHashPathGenerator,
                    self.compression_level,
                ).unwrap_or_else(|err| { 
                    println!("::: ERROR: Failed to create storage backend ({:?})", err);
                    std::process::exit(1);
                });
                // Write generated melodies to backend
                write_melodies_to_backend(note_set, melody_length, backend);
            },
        }
    }
}
