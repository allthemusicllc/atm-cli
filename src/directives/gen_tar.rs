// gen_tar.rs
//
// Copyright (c) 2020 All The Music, LLC
//
// This work is licensed under the Creative Commons Attribution 4.0 International License.
// To view a copy of this license, visit http://creativecommons.org/licenses/by/4.0/ or send
// a letter to Creative Commons, PO Box 1866, Mountain View, CA 94042, USA.

use crate::{
    cli::CliDirective,
    directives::gen::write_melodies_to_backend,
};

/**************************
***** GenTarDirective *****
**************************/

/// Generate melodies and store them in Tar file
/// (see: [TarFile](../storage/tar_file/struct.TarFile.html))
#[derive(structopt::StructOpt)]
pub struct GenTarDirective {
    #[structopt(flatten)]
    pub note_set: crate::cli::NoteSetArg,
    #[structopt(flatten)]
    pub melody_length: crate::cli::MelodyLengthArg,
    #[structopt(flatten)]
    pub target: crate::cli::TargetArg,
    #[structopt(flatten)]
    pub partition_args: crate::cli::PartitionArgs,
}

impl CliDirective for GenTarDirective {
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
                let backend = crate::storage::TarFile::new(
                    target,
                    path_generator,
                ).unwrap_or_else(|err| { 
                    println!("::: ERROR: Failed to create storage backend ({:?})", err);
                    std::process::exit(1);
                });
                // Write generated melodies to backend
                write_melodies_to_backend(note_set, melody_length, backend);
            },
            // Don't use partitioning scheme
            None => {
                // Create storage backend
                let backend = crate::storage::TarFile::new(
                    target,
                    crate::storage::MIDIHashPathGenerator,
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
