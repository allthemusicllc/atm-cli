// utils.rs
//
// Copyright (c) 2020 All The Music, LLC
//
// This work is licensed under the Creative Commons Attribution 4.0 International License.
// To view a copy of this license, visit http://creativecommons.org/licenses/by/4.0/ or send
// a letter to Creative Commons, PO Box 1866, Mountain View, CA 94042, USA.

#![allow(unused_parens)]

use itertools::Itertools;

/// Generate all permutations (with replacement) of given length
/// from the given sequence of MIDI notes.
///
/// # Arguments:
///
/// * `notes`: sequence of MIDI notes (see: [libatm::MIDINote](../../libatm/struct.MIDINote.html))
/// * `length`: length of sequences to generate
///
/// # Examples
///
/// ```rust
/// // Create MIDI note sequence
/// let sequence = "C:4,C:4,D:4,E:4,F:4,G:5".parse::<libatm::MIDINoteSequence>().unwrap();
/// // Create iterable over all permutations, which in this example would be
/// // 6^8 = 1,679,616 instances of `Vec<&libatm::MIDINote>`.
/// let permutations = atm::utils::gen_sequences(&sequence.notes, 8);
/// ```
pub fn gen_sequences(
    notes: &[libatm::MIDINote],
    length: u32,
) -> itertools::MultiProduct<std::slice::Iter<libatm::MIDINote>> {
    (0..(length))
        .map(|_| notes.iter())
        .multi_cartesian_product()
}
