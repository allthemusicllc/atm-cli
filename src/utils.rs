// utils.rs
//
// Copyright (c) 2020 All The Music, LLC
//
// This work is licensed under the Creative Commons Attribution 4.0 International License.
// To view a copy of this license, visit http://creativecommons.org/licenses/by/4.0/ or send
// a letter to Creative Commons, PO Box 1866, Mountain View, CA 94042, USA.

#![allow(unused_parens)]

use itertools::Itertools;

/// Calculate total melodies given number of distinct notes and desired length of melodies
///
/// # Arguments
///
/// * `num_notes`: number of distinct notes to generate melodies with
/// * `melody_length`: length of melodies to generate
///
/// # Examples
///
/// ```rust 
/// // Create MIDI note set with 6 notes
/// let note_set = "C:4,C:7,D:4,E:4,F:4,G:5".parse::<libatm::MIDINoteSet>().unwrap();
/// // Calculate total number of melodies of length 9 with these 6 notes
/// let num_melodies = atm::utils::gen_num_melodies(note_set.len() as u32, 9);
/// // 6 ^ 9 is 10,077,696
/// assert_eq!(10077696, num_melodies);
/// ```
pub fn gen_num_melodies(num_notes: u32, melody_length: u32) -> u64 {
    (num_notes as u64).pow(melody_length)
}

/// Generate melodies of length `length` containing the
/// notes in provided note set `notes`. In other words,
/// generate the cartesion product of `notes` with itself
/// `length` times.
///
/// # Arguments:
///
/// * `notes`: set of MIDI notes (see: [libatm::MIDINote](../../libatm/midi_note/struct.MIDINote.html))
/// * `length`: length of sequences to generate
///
/// # Examples
///
/// ```rust
/// // Create MIDI note set
/// let note_set = "C:4,C:4,D:4,E:4,F:4,G:5".parse::<libatm::MIDINoteVec>().unwrap();
/// // Create iterable over all possible melodies, which in this example would be
/// // 6 ^ 8 = 1,679,616 instances of `Vec<&libatm::MIDINote>`.
/// let melodies = atm::utils::gen_sequences(&note_set, 8);
/// assert_eq!(1679616usize, melodies.count())
/// ```
pub fn gen_sequences(
    notes: &libatm::MIDINoteVec,
    length: u32,
) -> itertools::MultiProduct<std::slice::Iter<libatm::MIDINote>> {
    (0..(length))
        .map(|_| notes.iter())
        .multi_cartesian_product()
}
