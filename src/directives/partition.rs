// partition.rs
//
// Copyright (c) 2020 All The Music, LLC
//
// This work is licensed under the Creative Commons Attribution 4.0 International License.
// To view a copy of this license, visit http://creativecommons.org/licenses/by/4.0/ or send
// a letter to Creative Commons, PO Box 1866, Mountain View, CA 94042, USA.

use crate::{
    cli::{CliDirective, PartitionArgs},
    storage::{PartitionPathGenerator, PathGenerator},
};

/*****************************
***** PartitionDirective *****
*****************************/

/// Generate the partition(s) for a MIDI pitch sequence within a partitioning scheme.
/// If no partition depth is provided, will default to a depth of 1.
#[derive(structopt::StructOpt)]
pub struct PartitionDirective {
    #[structopt(flatten)]
    pub note_vec: crate::cli::NoteVecArg,
    #[structopt(flatten)]
    pub partition: PartitionArgs,
}

impl CliDirective for PartitionDirective {
    fn run(self) {
        let note_vec = self.note_vec.note_vec;
        let melody_length = note_vec.len() as u32;
        let max_files = self.partition.max_files;
        let partition_depth = self.partition.partition_depth.unwrap_or(1);

        let path_generator = PartitionPathGenerator::new(melody_length, melody_length, max_files, partition_depth);
        match path_generator {
            Ok(path_generator) => {
                let mfile = libatm::MIDIFile::new(note_vec, libatm::MIDIFormat::Format0, 1, 1);
                match path_generator.gen_path_for_file(&mfile) {
                    Ok(path) => println!("{}", path),
                    Err(err) => {
                        println!("::: ERROR: Failed to generate path for melody ({})", err);
                        std::process::exit(1);
                    },
                }
            },
            Err(err) => {
                println!("::: ERROR: Failed to initialize partition generator ({})", err);
                std::process::exit(2);
            },
        }
    }
}
