// gen.rs
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
    storage::StorageBackend,
};

/**************************
***** Utility Methods *****
**************************/

/// Generate melodies and write them to provided backend
fn write_melodies_to_backend<B: StorageBackend>(
    note_set: libatm::MIDINoteSet,
    melody_length: u32,
    mut backend: B,
) {
    // Convert set of notes to vec
    let notes = libatm::MIDINoteVec::from(note_set); 

    println!("::: INFO: Generating all melodies of length {} containing notes {:?}", melody_length, &notes);
    // For each melody
    for melody_ref in crate::utils::gen_sequences(&notes, melody_length) {
        // Copy notes into owned melody
        let melody = melody_ref.iter().map(|n| *n.clone()).collect::<libatm::MIDINoteVec>();
        // Show error if adding melody to backend failed
        if let Err(err) = backend.append_melody(melody, None) {
            println!("::: WARNING: Failed to add melody to storage backend ({:?})", err);
        }
    }

    // Finish writing to backend
    if let Err(err) = backend.finish() {
        println!("::: ERROR: Failed to finish writing to storage backend ({:?})", err);
        std::process::exit(1);
    } else {
        println!("::: INFO: Finished writing melodies to storage backend");
    }
}

/*****************************
***** GenSingleDirective *****
*****************************/

#[derive(structopt::StructOpt)]
pub struct GenSingleDirective {
    #[structopt(flatten)]
    pub note_vec: crate::cli::NoteVecArg,
    #[structopt(flatten)]
    pub target: crate::cli::TargetArg,
}

impl CliDirective for GenSingleDirective {
    fn run(self) {
        // Get values from args
        let note_vec = self.note_vec.note_vec;
        let target = &*self.target;
        // Generate MIDIFile from input melody
        println!("::: INFO: Generating MIDI file from pitch sequence");
        let mfile = libatm::MIDIFile::new(note_vec, libatm::MIDIFormat::Format0, 1, 1);

        // Write MIDI file to target file path
        println!("::: INFO: Attempting to write MIDI file to {:?}", target);
        match mfile.write_file(target) {
            Err(err) => println!("::: ERROR: Failed to write MIDI file to path {:?} ({})", target, err),
            _ => println!("::: INFO: Successfully wrote MIDI file"),
        }
    }
}

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
}

impl CliDirective for GenTarDirective {
    fn run(self) {
        // Create storage backend
        let backend = crate::storage::TarFile::new(
            &*self.target,
            crate::storage::MIDIHashPathGenerator,
        ).unwrap_or_else(|err| { 
            println!("::: ERROR: Failed to create storage backend ({:?})", err);
            std::process::exit(1);
        });

        // Write generated melodies to backend
        write_melodies_to_backend(self.note_set.note_set, *self.melody_length, backend);
    }
}

/**************************
***** GenTarPDirective *****
**************************/

/// Generate melodies and store them in Tar file with partitioning
/// (see: [TarFile](../storage/tar_file/struct.TarFile.html))
#[derive(structopt::StructOpt)]
pub struct GenTarPDirective {
    #[structopt(flatten)]
    pub note_set: crate::cli::NoteSetArg,
    #[structopt(flatten)]
    pub melody_length: crate::cli::MelodyLengthArg,
    #[structopt(flatten)]
    pub target: crate::cli::TargetArg,
    #[structopt(flatten)]
    pub partition_args: crate::cli::PartitionArgs,
}

impl CliDirective for GenTarPDirective {
    fn run(self) {
        // Create path generator
        let path_generator = crate::storage::PartitionPathGenerator::new(
            (&*self.note_set).len() as f32,
            *self.melody_length as i32,
            self.partition_args.max_files,
            self.partition_args.partition_depth,
        ).unwrap_or_else(|err| {
            println!("::: ERROR: Failed to initialize path partitioning scheme ({:?})", err);
            std::process::exit(1);
        });
        // Create storage backend
        let backend = crate::storage::TarFile::new(
            &*self.target,
            path_generator,
        ).unwrap_or_else(|err| { 
            println!("::: ERROR: Failed to create storage backend ({:?})", err);
            std::process::exit(1);
        });

        // Write generated melodies to backend
        write_melodies_to_backend(self.note_set.note_set, *self.melody_length, backend);
    }
}

/****************************
***** GenTarGzDirective *****
****************************/

/// Error type for converting `&str` to
/// [flate2::Compression](../../flate2/struct.Compression.html)
#[derive(Debug, thiserror::Error)]
pub enum CompressionArgError {
    #[error(transparent)]
    NotInteger(#[from] std::num::ParseIntError),
    #[error("Compression level must be between 0 and 9 (found {input})")]
    ValueOutOfRange { input: u32 },
}

fn try_compression_from_str(arg: &str) -> Result<Compression, CompressionArgError> {
    let compression_level = arg.parse::<u32>()?;
    if compression_level > 9 {
        return Err(CompressionArgError::ValueOutOfRange { input: compression_level });
    }
    Ok(Compression::new(compression_level))
}

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
}

impl CliDirective for GenTarGzDirective {
    fn run(self) {
        // Create storage backend
        let backend = crate::storage::TarGzFile::new(
            &*self.target,
            crate::storage::MIDIHashPathGenerator,
            self.compression_level,
        ).unwrap_or_else(|err| { 
            println!("::: ERROR: Failed to create storage backend ({:?})", err);
            std::process::exit(1);
        });

        // Write generated melodies to backend
        write_melodies_to_backend(self.note_set.note_set, *self.melody_length, backend);
    }
}

/****************************
***** GenTarGzPDirective *****
****************************/

/// Generate melodies and store them in Gzip-compressed Tar file with partitioning
/// (see: [TarGzFile](../storage/tar_gz_file/struct.TarGzFile.html))
#[derive(structopt::StructOpt)]
pub struct GenTarGzPDirective {
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

impl CliDirective for GenTarGzPDirective {
    fn run(self) {
        // Create path generator
        let path_generator = crate::storage::PartitionPathGenerator::new(  
            (&*self.note_set).len() as f32,
            *self.melody_length as i32,
            self.partition_args.max_files,
            self.partition_args.partition_depth,
        ).unwrap_or_else(|err| {
            println!("::: ERROR: Failed to initialize path partitioning scheme ({:?})", err);
            std::process::exit(1);
        });
        // Create storage backend
        let backend = crate::storage::TarGzFile::new(
            &*self.target,
            path_generator,
            self.compression_level,
        ).unwrap_or_else(|err| { 
            println!("::: ERROR: Failed to create storage backend ({:?})", err);
            std::process::exit(1);
        });

        // Write generated melodies to backend
        write_melodies_to_backend(self.note_set.note_set, *self.melody_length, backend);
    }
}

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
        // Create storage backend
        let backend = crate::storage::BatchTarFile::new(
            &*self.target,
            *self.batch_size,
            (&*self.note_set).len() as f32,
            *self.melody_length as i32,
            self.partition_args.max_files,
            self.partition_args.partition_depth,
            self.batch_compression,
            self.batch_mode,
        ).unwrap_or_else(|err| { 
            println!("::: ERROR: Failed to create storage backend ({:?})", err);
            std::process::exit(1);
        });

        // Write generated melodies to backend
        write_melodies_to_backend(self.note_set.note_set, *self.melody_length, backend);
    }
}

/***********************
***** GenDirective *****
***********************/

#[derive(structopt::StructOpt)]
#[structopt(about="Generate melodies (MIDI files) and store them in a file/files")]
pub enum GenDirective {
    #[structopt(
        name="batch",
        about=concat!("Generate melodies and store them in Tar file, where each entry ",
                      "is a Gzip-compressed Tar file. Use for the largest datasets, where storing ",
                      "the generated melodies in a single file isn't feasible."))]
    GenBatch(GenBatchDirective),
    #[structopt(name="single", about="Generate single melody (MIDI file).")]
    GenSingle(GenSingleDirective),
    #[structopt(
        name="tar",
        about=concat!("Generate melodies and store them in Tar file. ",
                      "Use for smaller datasets where output file size is ",
                      "less of a concern."))]
    GenTar(GenTarDirective),
    #[structopt(
        name="tar_p",
        about=concat!("Generate melodies and store them in Tar file pursuant ",
                      "to partitioning scheme. Use for datasets where output ",
                      "file size is less of a concern."))]
    GenTarP(GenTarPDirective),
    #[structopt(
        name="tar_gz",
        about=concat!("Generate melodies and store them in Gzip-compressed Tar file. ",
                      "Use for smaller datasets where output file size is ",
                      "more of a concern (see: compression_level)."))]
    GenTarGz(GenTarGzDirective),
    #[structopt(
        name="tar_gz_p",
        about=concat!("Generate melodies and store them in Gzip-compressed Tar file ",
                      "pursuant to partitioning scheme. Use for datasets where output ",
                      "file size is more of a concern (see: compression_level)."))]
    GenTarGzP(GenTarGzPDirective),
}

impl CliDirective for GenDirective {
    fn run(self) {
        match self {
            Self::GenBatch(d) => d.run(),
            Self::GenSingle(d) => d.run(),
            Self::GenTar(d) => d.run(),
            Self::GenTarP(d) => d.run(),
            Self::GenTarGz(d) => d.run(),
            Self::GenTarGzP(d) => d.run(),
        }
    }
}
