// directives.rs
//
// Copyright (c) 2020 All The Music, LLC
//
// This work is licensed under the Creative Commons Attribution 4.0 International License.
// To view a copy of this license, visit http://creativecommons.org/licenses/by/4.0/ or send
// a letter to Creative Commons, PO Box 1866, Mountain View, CA 94042, USA.

pub mod single;

pub use single::*;

// fn parse_length_argument<'a>(matches: &clap::ArgMatches<'a>) -> u32 {
//     let length = matches.value_of("LENGTH").unwrap();
//     let length = length.parse::<u32>().unwrap();
//     if length == 0 {
//         panic!("Length must be greater than 0")
//     }
//     length 
// }
// 
// fn parse_max_count_argument<'a>(matches: &clap::ArgMatches<'a>, num_notes: f32, length: i32) -> usize {
//     let max_count = matches.value_of("COUNT");
//     let max_count = match max_count {
//         None => (num_notes.powi(length) as usize),
//         Some(count) => {
//             let count = count.parse::<usize>().unwrap();
//             if count == 0 {
//                 panic!("Count must be greater than 0");
//             }
//             count
//         }
//     };
//     max_count
// }
// 
// fn parse_batch_size_argument<'a>(matches: &clap::ArgMatches<'a>) -> u32 {
//     let batch_size = matches.value_of("BATCH_SIZE").unwrap();
//     let batch_size = batch_size.parse::<u32>().unwrap();
//     if batch_size == 0 {
//         panic!("Batch size must be greater than 0");
//     }
//     batch_size
// }
// 
// /***************************/
// /***** Batch Directive *****/
// /***************************/
// 
// #[derive(Debug)]
// pub struct BatchDirectiveArgs {
//     pub sequence: libatm::MIDINoteSequence,
//     pub length: u32,
//     pub target: String,
//     pub partition_depth: u32,
//     pub max_files: f32,
//     pub partition_size: u32,
//     pub batch_size: u32,
//     pub max_count: usize,
//     pub update: u64,
// }
// 
// impl<'a> From<&clap::ArgMatches<'a>> for BatchDirectiveArgs {
//     fn from(matches: &clap::ArgMatches<'a>) -> BatchDirectiveArgs {
//         // Generate libatm::MIDINoteSequence from notes argument
//         let sequence = parse_sequence_argument(matches);
// 
//         // Parse length argument as integer
//         let length = parse_length_argument(matches);
// 
//         // Parse target argument
//         let target = parse_target_argument(matches);
// 
//         // Parse partition_depth argument as integer
//         let partition_depth = parse_partition_depth_argument(matches);
// 
//         // Parse max_files argument and set default if not provided
//         let max_files = parse_max_files_argument(matches);
// 
//         // Calculate partition size (# of notes) from given arguments (see: gen_partition_size)
//         let partition_size = crate::utils::gen_partition_size(
//             sequence.notes.len() as f32,
//             length as i32,
//             max_files,
//             partition_depth as i32,
//         );
// 
//         // Parse max_count argument and set default if not provided
//         let max_count = parse_max_count_argument(matches, sequence.notes.len() as f32, length as i32);
// 
//         // Parse batch_size argument
//         let batch_size = parse_batch_size_argument(matches);
// 
//         // Parse update argument and set default if not provided
//         let update = matches.value_of("PB_UPDATE");
//         let update: u64 = match update {
//             None => 1000,
//             Some(duration) => duration.parse::<u64>().unwrap(),
//         };
//         if update == 0 {
//             panic!("Update must be greater than 0");
//         }
// 
//         BatchDirectiveArgs {
//             sequence,
//             length,
//             target,
//             partition_depth,
//             max_files,
//             partition_size,
//             batch_size,
//             max_count,
//             update,
//         }
//     }
// }
// 
// pub fn atm_batch(args: BatchDirectiveArgs) {
//     // Initialize progress bar and set refresh rate
//     let mut pb = pbr::ProgressBar::new(args.max_count as u64);
//     pb.set_max_refresh_rate(Some(std::time::Duration::from_millis(args.update)));
//     // Initialize output archive
//     let mut archive = crate::utils::BatchedMIDIArchive::new(
//         &args.target,
//         args.partition_depth,
//         args.max_files,
//         args.partition_size,
//         args.batch_size,
//     );
//     // For each generated sequence
//     for (idx, notes) in crate::utils::gen_sequences(&args.sequence.notes, args.length).enumerate() {
//         // if reached max count, finish
//         if idx == args.max_count {
//             archive.finish().unwrap();
//             break;
//         }
//         // Clone libatm::MIDINoteSequence from Vec<&libatm::MIDINote>
//         let seq = libatm::MIDINoteSequence::new(
//             notes
//                 .iter()
//                 .map(|note| *note.clone())
//                 .collect::<Vec<libatm::MIDINote>>(),
//         );
//         // Create MIDIFile from libatm::MIDINoteSequence
//         let mfile = libatm::MIDIFile::new(seq, libatm::MIDIFormat::Format0, 1, 1);
//         // Add MIDIFile to archive
//         archive.push(mfile).unwrap();
//         // Increment progress bar
//         pb.inc();
//     }
//     // Stop progress bar
//     pb.finish_println("");
//     // Finish archive if not already finished
//     if let crate::utils::BatchedMIDIArchiveState::Open = archive.state {
//         archive.finish().unwrap();
//     }
// }
// 
// /*******************************/
// /***** Partition Directive *****/
// /*******************************/
// 
// #[derive(Debug)]
// pub struct PartitionDirectiveArgs {
//     pub sequence: libatm::MIDINoteSequence,
//     pub partition_depth: u32,
//     pub max_files: f32,
//     pub partition_size: u32,
// }
// 
// impl<'a> From<&clap::ArgMatches<'a>> for PartitionDirectiveArgs {
//     fn from(matches: &clap::ArgMatches<'a>) -> PartitionDirectiveArgs {
//         // Generate libatm::MIDINoteSequence from notes argument
//         let sequence = parse_sequence_argument(matches);
// 
//         // Parse partition_depth argument as integer
//         let partition_depth = parse_partition_depth_argument(matches);
// 
//         // Parse max_files argument and set default if not provided
//         let max_files = parse_max_files_argument(matches);
// 
//         // Calculate partition size (# of notes) from given arguments (see: gen_partition_size)
//         let partition_size = crate::utils::gen_partition_size(
//             sequence.notes.len() as f32,
//             sequence.notes.len() as i32,
//             max_files,
//             partition_depth as i32,
//         );
// 
//         PartitionDirectiveArgs {
//             sequence,
//             partition_depth,
//             max_files,
//             partition_size,
//         }
//     }
// }
// 
// pub fn atm_partition(args: PartitionDirectiveArgs) {
//     println!("::: INFO: Generating MIDI file from pitch sequence");
//     // Create MIDIFile from sequence
//     let mfile = libatm::MIDIFile::new(args.sequence, libatm::MIDIFormat::Format0, 1, 1);
//     // Generate MIDI sequence hash
//     let hash = mfile.gen_hash();
//     println!("::: INFO: Generating partition(s)");
//     // Generate partitions
//     let path = crate::utils::gen_path(&hash, args.partition_size, args.partition_depth);
//     // Print full path with partitions
//     println!("::: INFO: Path for sequence is {}/{}.mid", &path, &hash);
// }
