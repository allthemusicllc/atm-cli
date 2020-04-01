// estimate_tar.rs
//
// Copyright (c) 2020 All The Music, LLC
//
// This work is licensed under the Creative Commons Attribution 4.0 International License.
// To view a copy of this license, visit http://creativecommons.org/licenses/by/4.0/ or send
// a letter to Creative Commons, PO Box 1866, Mountain View, CA 94042, USA.

use humansize::{FileSize, file_size_opts as options};

use crate::cli::CliDirective;

/*******************************
***** EstimateTarDirective *****
*******************************/

const ENTRY_SIZE: u64 = 1024;

const CAVEATS: &'static str = "\
Estimate assumes underlying drive has block size of 512 bytes. Some RAID arrays \
can have a stripe size up to 128KB (or higher), and some modern file systems have block \
sizes of 4KB, which could affect file size. For example, if the output dataset is 3KB, \
but the block or stripe size is 4KB, then the file will take 4KB of space on disk.";

#[derive(structopt::StructOpt)]
pub struct EstimateTarDirective {
    #[structopt(flatten)]
    pub num_notes: crate::cli::NumNotesArg,
    #[structopt(flatten)]
    pub melody_length: crate::cli::MelodyLengthArg,
}

impl CliDirective for EstimateTarDirective {
    fn run(self) {
        let num_notes: u32 = self.num_notes.into();
        let melody_length: u32 = self.melody_length.into();

        // Generate total number of melodies
        let num_melodies = crate::utils::gen_num_melodies(num_notes, melody_length);

        println!(
            concat!("Number of distinct notes:               {num_notes}\n",
                    "Length of melodies (notes):             {melody_length}\n",
                    "Total number of melodies:               {num_melodies}\n",
                    "Estimated approximate output file size: {file_size}\n",
                    "Caveats: {caveats}"),
            num_notes=num_notes,
            melody_length=melody_length,
            num_melodies=num_melodies,
            file_size=(num_melodies * ENTRY_SIZE).file_size(options::CONVENTIONAL).unwrap(),
            caveats=CAVEATS,
        );
    }
}
