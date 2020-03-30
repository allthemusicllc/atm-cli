// gen_single.rs
//
// Copyright (c) 2020 All The Music, LLC
//
// This work is licensed under the Creative Commons Attribution 4.0 International License.
// To view a copy of this license, visit http://creativecommons.org/licenses/by/4.0/ or send
// a letter to Creative Commons, PO Box 1866, Mountain View, CA 94042, USA.

use crate::cli::CliDirective;

/*****************************
***** GenSingleDirective *****
*****************************/

#[derive(Debug, structopt::StructOpt)]
pub struct GenSingleDirective {
    #[structopt(flatten)]
    pub note_vec: crate::cli::NoteVecArg,
    #[structopt(flatten)]
    pub target: crate::cli::TargetArg,
}

impl CliDirective for GenSingleDirective {
    fn run(self) {
        // Get values from args
        let note_vec = self.note_vec.into();
        let target: std::path::PathBuf = self.target.into();
        // Generate MIDIFile from input melody
        println!("::: INFO: Generating MIDI file from pitch sequence");
        let mfile = libatm::MIDIFile::new(note_vec, libatm::MIDIFormat::Format0, 1, 1);

        // Write MIDI file to target file path
        println!("::: INFO: Attempting to write MIDI file to {:?}", target);
        match mfile.write_file(&target) {
            Err(err) => println!("::: ERROR: Failed to write MIDI file to path {:?} ({})", &target, err),
            _ => println!("::: INFO: Successfully wrote MIDI file"),
        }
    }
}

