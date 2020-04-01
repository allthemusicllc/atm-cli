// estimate_tar_gz.rs
//
// Copyright (c) 2020 All The Music, LLC
//
// This work is licensed under the Creative Commons Attribution 4.0 International License.
// To view a copy of this license, visit http://creativecommons.org/licenses/by/4.0/ or send
// a letter to Creative Commons, PO Box 1866, Mountain View, CA 94042, USA.

use flate2::{
    Compression,
    write::GzEncoder,
};
use humansize::{FileSize, file_size_opts as options};

use crate::{
    cli::CliDirective,
    directives::gen::try_compression_from_str,
    storage::{
        IntoInner,
        MIDIHashPathGenerator,
        StorageBackend,
        tar_archive::TarArchive,
    },
};

/********************************
***** EsimateTarGzDirective *****
********************************/

const MAX_SIM_NUM_MELODIES: u64 = 200000;

fn estimate_tar_gz_size(
    notes: &libatm::MIDINoteVec,
    melody_length: u32,
    num_melodies: u64,
    compression_level: Compression
) -> u64 {
    // Create gzip-compressed tar archive
    let mut archive = TarArchive::new(
        GzEncoder::new(std::io::BufWriter::new(Vec::new()), compression_level),
        MIDIHashPathGenerator
    );

    // For each melody
    for (idx, melody_ref) in crate::utils::gen_sequences(notes, melody_length).enumerate() {
        if idx as u64 == num_melodies { break; }
        // Copy notes into owned melody
        let melody = melody_ref.iter().map(|n| *n.clone()).collect::<libatm::MIDINoteVec>();
        // Append melody to archive
        archive.append_melody(melody, None).unwrap();
    }

    archive
        .into_inner()   // Finish archive
        .unwrap()       // GzEncoder
        .finish()       // Finish GzEncoder
        .unwrap()       // std::io::BufWriter
        .into_inner()   // Finish writing to buffer
        .unwrap()       // Vec<u8>
        .len() as u64   // Number of bytes in vector as u64
}

#[derive(structopt::StructOpt)]
pub struct EstimateTarGzDirective {
    #[structopt(flatten)]
    pub note_set: crate::cli::NoteSetArg,
    #[structopt(flatten)]
    pub melody_length: crate::cli::MelodyLengthArg,
    #[structopt(
        short="C",
        long="compress",
        help="Compression level [0-9, default: 6]",
        parse(try_from_str = try_compression_from_str))]
    pub compression_level: Option<Compression>,
}

impl CliDirective for EstimateTarGzDirective {
    fn run(self) {
        let notes = libatm::MIDINoteVec::from(self.note_set.note_set);
        let num_notes = notes.len() as u32;
        let melody_length = self.melody_length.into();
        let compression_level = self.compression_level.unwrap_or(Compression::new(6));

        // Generate total number of melodies
        let num_melodies = crate::utils::gen_num_melodies(num_notes, melody_length);
        // Generate number of melodies to simulate:
        // if total number of melodies is less than MAX, simulate all of them
        let sim_num_melodies = if num_melodies <= MAX_SIM_NUM_MELODIES {
            num_melodies
        // otherwise, use minimum of 20% of total number and MAX
        } else {
            std::cmp::min((num_melodies as f64 * 0.20).floor() as u64, MAX_SIM_NUM_MELODIES)
        };
        // Generate size estimate for simulation count
        let mut sim_size_estimate = estimate_tar_gz_size(&notes, melody_length, sim_num_melodies, compression_level);
        // Align to 512 bytes
        if let Some(padding) = sim_size_estimate.checked_rem_euclid(512) {
            sim_size_estimate = sim_size_estimate + padding;
        }
        // Generate total size estimate
        let file_size = if sim_num_melodies == num_melodies {
            sim_size_estimate
        } else if sim_num_melodies == MAX_SIM_NUM_MELODIES {
                ((num_melodies as f64 / MAX_SIM_NUM_MELODIES as f64).ceil() as u64) * sim_size_estimate
        } else {
            sim_size_estimate * 5
        };

        println!(
            concat!("Number of distinct notes:               {num_notes}\n",
                    "Length of melodies (notes):             {melody_length}\n",
                    "Compression level:                      {compression_level:?}\n",
                    "Total number of melodies:               {num_melodies}\n",
                    "Number of melodies used in simulation:  {sim_num_melodies}\n",
                    "Simulated output size:                  {sim_size_estimate}\n",
                    "Estimated approximate output file size: {file_size}\n",
                    "Caveats: Estimate calculated by creating a gzip-compressed tar file in memory \
                    containing {sim_num_melodies} melodies, and extrapolating from that size. Assumes underlying \
                    drive has block size of 512 bytes (see: 'estimate tar')."),
            num_notes=num_notes,
            melody_length=melody_length,
            compression_level=compression_level,
            num_melodies=num_melodies,
            sim_num_melodies=sim_num_melodies,
            sim_size_estimate=sim_size_estimate.file_size(options::CONVENTIONAL).unwrap(),
            file_size=file_size.file_size(options::CONVENTIONAL).unwrap(),
        );
    }
}
