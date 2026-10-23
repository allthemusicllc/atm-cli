// partition.rs
//
// Copyright (c) 2020 All The Music, LLC
//
// This work is licensed under the Creative Commons Attribution 4.0 International License.
// To view a copy of this license, visit http://creativecommons.org/licenses/by/4.0/ or send
// a letter to Creative Commons, PO Box 1866, Mountain View, CA 94042, USA.

use crate::{
    cli::{CliDirective, PartitionArgs},
    storage::{PartitionPathGenerator, PartitionPathGeneratorError, PathGenerator},
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
    pub num_notes: crate::cli::NumDistinctNotesArg,
    #[structopt(flatten)]
    pub partition: PartitionArgs,
}

impl PartitionDirective {
    fn path_generator(&self) -> Result<PartitionPathGenerator, PartitionPathGeneratorError> {
        let melody_length = self.note_vec.note_vec.len() as u32;
        let max_files = self.partition.max_files;
        let partition_depth = self.partition.partition_depth.unwrap_or(1);

        PartitionPathGenerator::new(
            self.num_notes.num_notes,
            melody_length,
            max_files,
            partition_depth,
        )
    }
}

impl CliDirective for PartitionDirective {
    fn run(self) {
        let path_generator = self.path_generator();
        let note_vec = self.note_vec.note_vec;
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

#[cfg(test)]
mod tests {
    use super::*;
    use structopt::StructOpt;

    #[test]
    fn partition_uses_explicit_note_set_size() {
        let directive = PartitionDirective::from_iter_safe([
            "atm",
            "--num-notes",
            "2",
            "--max-files",
            "2",
            "--partitions",
            "2",
            "C:4,D:4,D:4",
        ])
        .unwrap();

        let path_generator = directive.path_generator().unwrap();
        let melody = libatm::MIDIFile::new(
            directive.note_vec.note_vec,
            libatm::MIDIFormat::Format0,
            1,
            1,
        );
        let path = path_generator.gen_path_for_file(&melody).unwrap();
        assert_eq!(std::path::Path::new(&path).components().count(), 3);
    }

    #[test]
    fn partition_requires_note_set_size() {
        let result = PartitionDirective::from_iter_safe([
            "atm",
            "--max-files",
            "2",
            "--partitions",
            "2",
            "C:4,D:4,D:4",
        ]);

        assert!(result.is_err());
    }
}
