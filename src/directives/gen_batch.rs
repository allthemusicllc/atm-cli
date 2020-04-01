// gen_batch.rs
//
// Copyright (c) 2020 All The Music, LLC
//
// This work is licensed under the Creative Commons Attribution 4.0 International License.
// To view a copy of this license, visit http://creativecommons.org/licenses/by/4.0/ or send
// a letter to Creative Commons, PO Box 1866, Mountain View, CA 94042, USA.

use std::str::FromStr;

use flate2::Compression;

use crate::{
    cli::CliDirective,
    directives::gen::{
        try_compression_from_str,
        write_melodies_to_backend,
    },
};

/****************************
***** GenBatchDirective *****
****************************/

/// Generate melodies and store them in nested Gzip-compressed Tar files
/// (see: [BatchTarFile](../storage/batch_far_file/struct.BatchTarFile.html))
#[derive(structopt::StructOpt)]
pub struct GenBatchDirective {
    #[structopt(flatten)]
    pub note_set: crate::cli::NoteSetArg,
    #[structopt(flatten)]
    pub melody_length: crate::cli::MelodyLengthArg,
    #[structopt(flatten)]
    pub target: crate::cli::TargetArg,
    #[structopt(flatten)]
    pub partition_args: crate::cli::PartitionArgs,
    #[structopt(
        short="m",
        long="mode",
        help="Permissions to use for entries in top-level Tar file [default: 644]",
        parse(try_from_str = u32::from_str))]
    pub batch_mode: Option<u32>,
    #[structopt(
        short="C",
        long="compress",
        help="Compression level [0-9, default: 6]",
        parse(try_from_str = try_compression_from_str))]
    pub batch_compression: Option<Compression>,
    #[structopt(flatten)]
    pub batch_size: crate::cli::BatchSize,
}

impl CliDirective for GenBatchDirective {
    fn run(self) {
        let partition_depth = match self.partition_args.partition_depth {
            Some(partition_depth) => partition_depth,
            None => {
                println!("::: ERROR: Must provide partition depth");
                std::process::exit(1);
            },
        };
        let note_set: libatm::MIDINoteSet = self.note_set.into();
        let melody_length = self.melody_length.into();
        let target: std::path::PathBuf = self.target.into();

        // Create storage backend
        let backend = crate::storage::BatchTarFile::new(
            target,
            self.batch_size.into(),
            note_set.len() as u32,
            melody_length,
            self.partition_args.max_files,
            partition_depth,
            self.batch_compression,
            self.batch_mode,
        ).unwrap_or_else(|err| { 
            println!("::: ERROR: Failed to create storage backend ({:?})", err);
            std::process::exit(1);
        });

        // Write generated melodies to backend
        write_melodies_to_backend(note_set, melody_length, backend);
    }
}
