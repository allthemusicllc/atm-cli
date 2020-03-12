// single.rs
//
// Copyright (c) 2020 All The Music, LLC
//
// This work is licensed under the Creative Commons Attribution 4.0 International License.
// To view a copy of this license, visit http://creativecommons.org/licenses/by/4.0/ or send
// a letter to Creative Commons, PO Box 1866, Mountain View, CA 94042, USA.

use crate::cli::CliDirective;

#[derive(structopt::StructOpt)]
pub struct SingleDirectiveArgs {
    #[structopt(flatten)]
    pub sequence: crate::cli::NoteSequenceArg,
    #[structopt(flatten)]
    pub target: crate::cli::TargetArg,
}

impl CliDirective for SingleDirectiveArgs {
    fn run(self) {
        // Get values from args
        let sequence = self.sequence.sequence;
        let target = self.target.target;
        // Generate MIDIFile from input melody
        println!("::: INFO: Generating MIDI file from pitch sequence");
        let mfile = libatm::MIDIFile::new(sequence, libatm::MIDIFormat::Format0, 1, 1);

        // Write MIDI file to target file path
        println!("::: INFO: Attempting to write MIDI file to {:?}", &target);
        match mfile.write_file(target.to_str().unwrap()) {
            Err(err) => panic!("Failed to write MIDI file to path {:?} ({})", &target, err),
            _ => println!("::: INFO: Successfully wrote MIDI file"),
        }
    }
}
