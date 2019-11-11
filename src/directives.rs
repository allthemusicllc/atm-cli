// directives.rs
//
// Copyright (c) 2019 All The Music, LLC
//
// This work is licensed under the Creative Commons Attribution 4.0 International License.
// To view a copy of this license, visit http://creativecommons.org/licenses/by/4.0/ or send
// a letter to Creative Commons, PO Box 1866, Mountain View, CA 94042, USA.

use std::io::Read;

/*********************************************/
/***** Argument Parsing Helper Functions *****/
/*********************************************/

fn parse_sequence_argument<'a>(matches: &clap::ArgMatches<'a>) -> libatm::MIDINoteSequence {
    let sequence = matches.value_of("NOTES").unwrap();
    let sequence = sequence.parse::<libatm::MIDINoteSequence>().unwrap();
    if sequence.notes.len() == 0 {
        panic!("Sequence must contain at least one note");
    }
    sequence
}

fn parse_target_argument<'a>(matches: &clap::ArgMatches<'a>) -> String {
    matches.value_of("TARGET").unwrap().to_string()
}

fn parse_partition_depth_argument<'a>(matches: &clap::ArgMatches<'a>) -> u32 {
    let partition_depth = matches.value_of("PARTITION_DEPTH").unwrap();
    let partition_depth = partition_depth.parse::<u32>().unwrap();
    if partition_depth == 0 || partition_depth > 4 {
        panic!("Partition depth must be between 0 and 5 (exclusive)");
    }
    partition_depth
}


fn parse_max_files_argument<'a>(matches: &clap::ArgMatches<'a>) -> f32 {
    let max_files = matches.value_of("MAX_FILES");
    let max_files = match max_files {
        None => 4096.0,
        Some(files) => files.parse::<f32>().unwrap(),
    };
    if max_files <= 0.0 || max_files > 4096.0 {
        panic!("Max files must be between 1 and 4096 (inclusive)");
    }
    max_files
}

fn parse_length_argument<'a>(matches: &clap::ArgMatches<'a>) -> u32 {
    let length = matches.value_of("LENGTH").unwrap();
    let length = length.parse::<u32>().unwrap();
    if length == 0 {
        panic!("Length must be greater than 0")
    }
    length 
}

fn parse_max_count_argument<'a>(matches: &clap::ArgMatches<'a>, num_notes: f32, length: i32) -> usize {
    let max_count = matches.value_of("COUNT");
    let max_count = match max_count {
        None => (num_notes.powi(length) as usize),
        Some(count) => {
            let count = count.parse::<usize>().unwrap();
            if count == 0 {
                panic!("Count must be greater than 0");
            }
            count
        }
    };
    max_count
}

fn parse_batch_size_argument<'a>(matches: &clap::ArgMatches<'a>) -> u32 {
    let batch_size = matches.value_of("BATCH_SIZE").unwrap();
    let batch_size = batch_size.parse::<u32>().unwrap();
    if batch_size == 0 {
        panic!("Batch size must be greater than 0");
    }
    batch_size
}

/****************************/
/***** Single Directive *****/
/****************************/

#[derive(Debug)]
pub struct SingleDirectiveArgs {
    pub sequence: libatm::MIDINoteSequence,
    pub target: String,
}

impl<'a> From<&clap::ArgMatches<'a>> for SingleDirectiveArgs {
    fn from(matches: &clap::ArgMatches<'a>) -> SingleDirectiveArgs {
        // Generate libatm::MIDINoteSequence from notes argument
        let sequence = parse_sequence_argument(matches);

        // Parse target argument
        let target = parse_target_argument(matches);

        SingleDirectiveArgs { sequence, target }
    }
}

pub fn atm_single(args: SingleDirectiveArgs) {
    println!("::: INFO: Generating MIDI file from pitch sequence");
    // Create MIDIFile from sequence
    let mfile = libatm::MIDIFile::new(args.sequence, libatm::MIDIFormat::Format0, 1, 1);
    println!(
        "::: INFO: Attempting to write MIDI file to path {}",
        &args.target
    );
    // Attempt to write file to target path
    if let Err(err) = mfile.write_file(&args.target) {
        panic!(
            "Failed to write MIDI file to path {} ({})",
            &args.target, err
        );
    } else {
        println!("::: INFO: Successfully wrote MIDI file");
    }
}

/***************************/
/***** Batch Directive *****/
/***************************/

#[derive(Debug)]
pub struct BatchDirectiveArgs {
    pub sequence: libatm::MIDINoteSequence,
    pub length: u32,
    pub target: String,
    pub partition_depth: u32,
    pub max_files: f32,
    pub partition_size: u32,
    pub batch_size: u32,
    pub max_count: usize,
    pub update: u64,
}

impl<'a> From<&clap::ArgMatches<'a>> for BatchDirectiveArgs {
    fn from(matches: &clap::ArgMatches<'a>) -> BatchDirectiveArgs {
        // Generate libatm::MIDINoteSequence from notes argument
        let sequence = parse_sequence_argument(matches);

        // Parse length argument as integer
        let length = parse_length_argument(matches);

        // Parse target argument
        let target = parse_target_argument(matches);

        // Parse partition_depth argument as integer
        let partition_depth = parse_partition_depth_argument(matches);

        // Parse max_files argument and set default if not provided
        let max_files = parse_max_files_argument(matches);

        // Calculate partition size (# of notes) from given arguments (see: gen_partition_size)
        let partition_size = crate::utils::gen_partition_size(
            sequence.notes.len() as f32,
            length as i32,
            max_files,
            partition_depth as i32,
        );

        // Parse max_count argument and set default if not provided
        let max_count = parse_max_count_argument(matches, sequence.notes.len() as f32, length as i32);

        // Parse batch_size argument
        let batch_size = parse_batch_size_argument(matches);

        // Parse update argument and set default if not provided
        let update = matches.value_of("PB_UPDATE");
        let update: u64 = match update {
            None => 1000,
            Some(duration) => duration.parse::<u64>().unwrap(),
        };
        if update == 0 {
            panic!("Update must be greater than 0");
        }

        BatchDirectiveArgs {
            sequence,
            length,
            target,
            partition_depth,
            max_files,
            partition_size,
            batch_size,
            max_count,
            update,
        }
    }
}

pub fn atm_batch(args: BatchDirectiveArgs) {
    // Initialize progress bar and set refresh rate
    let mut pb = pbr::ProgressBar::new(args.max_count as u64);
    pb.set_max_refresh_rate(Some(std::time::Duration::from_millis(args.update)));
    // Initialize output archive
    let mut archive = crate::utils::BatchedMIDIArchive::new(
        &args.target,
        args.partition_depth,
        args.max_files,
        args.partition_size,
        args.batch_size,
    );
    // For each generated sequence
    for (idx, notes) in crate::utils::gen_sequences(&args.sequence.notes, args.length).enumerate() {
        // if reached max count, finish
        if idx == args.max_count {
            archive.finish().unwrap();
            break;
        }
        // Clone libatm::MIDINoteSequence from Vec<&libatm::MIDINote>
        let seq = libatm::MIDINoteSequence::new(
            notes
                .iter()
                .map(|note| *note.clone())
                .collect::<Vec<libatm::MIDINote>>(),
        );
        // Create MIDIFile from libatm::MIDINoteSequence
        let mfile = libatm::MIDIFile::new(seq, libatm::MIDIFormat::Format0, 1, 1);
        // Add MIDIFile to archive
        archive.push(mfile).unwrap();
        // Increment progress bar
        pb.inc();
    }
    // Stop progress bar
    pb.finish_println("");
    // Finish archive if not already finished
    if let crate::utils::BatchedMIDIArchiveState::Open = archive.state {
        archive.finish().unwrap();
    }
}

/*******************************/
/***** Partition Directive *****/
/*******************************/

#[derive(Debug)]
pub struct PartitionDirectiveArgs {
    pub sequence: libatm::MIDINoteSequence,
    pub partition_depth: u32,
    pub max_files: f32,
    pub partition_size: u32,
}

impl<'a> From<&clap::ArgMatches<'a>> for PartitionDirectiveArgs {
    fn from(matches: &clap::ArgMatches<'a>) -> PartitionDirectiveArgs {
        // Generate libatm::MIDINoteSequence from notes argument
        let sequence = parse_sequence_argument(matches);

        // Parse partition_depth argument as integer
        let partition_depth = parse_partition_depth_argument(matches);

        // Parse max_files argument and set default if not provided
        let max_files = parse_max_files_argument(matches);

        // Calculate partition size (# of notes) from given arguments (see: gen_partition_size)
        let partition_size = crate::utils::gen_partition_size(
            sequence.notes.len() as f32,
            sequence.notes.len() as i32,
            max_files,
            partition_depth as i32,
        );

        PartitionDirectiveArgs {
            sequence,
            partition_depth,
            max_files,
            partition_size,
        }
    }
}

pub fn atm_partition(args: PartitionDirectiveArgs) {
    println!("::: INFO: Generating MIDI file from pitch sequence");
    // Create MIDIFile from sequence
    let mfile = libatm::MIDIFile::new(args.sequence, libatm::MIDIFormat::Format0, 1, 1);
    // Generate MIDI sequence hash
    let hash = mfile.gen_hash();
    println!("::: INFO: Generating partition(s)");
    // Generate partitions
    let path = crate::utils::gen_path(&hash, args.partition_size, args.partition_depth);
    // Print full path with partitions
    println!("::: INFO: Path for sequence is {}/{}.mid", &path, &hash);
}

/***************************/
/***** Split Directive *****/
/***************************/

#[derive(Debug)]
pub struct SplitDirectiveArgs {
    pub chunk_size: Option<u64>,
    pub num_chunks: Option<u32>,
    pub prefix: String,
    pub source: String,
    pub target: String,
}

impl<'a> From<&clap::ArgMatches<'a>> for SplitDirectiveArgs {
    fn from(matches: &clap::ArgMatches<'a>) -> SplitDirectiveArgs {
        // Parse chunk size argument
        let chunk_size = match matches.value_of("CHUNK_SIZE") {
            None => None,
            Some(chunk_size) => {
                let chunk_size = chunk_size.parse::<u64>().unwrap();
                if chunk_size == 0 {
                    panic!("Chunk size must be greater than 0");
                }
                Some(chunk_size)
            },
        };
        // Parse number of chunks argument
        let num_chunks = match matches.value_of("NUM_CHUNKS") {
            None => {
                if chunk_size == None {
                    panic!("Must provide either chunk size or number of chunks");
                } else { None }
            },
            Some(num_chunks) => {
                let num_chunks = num_chunks.parse::<u32>().unwrap();
                if num_chunks == 0 {
                    panic!("Number of chunks must be greater than 0");
                }
                Some(num_chunks)
            },
        };
        // Parse prefix argument
        let prefix = match matches.value_of("PREFIX") {
            None => String::from("split"),
            Some(prefix) => String::from(prefix),
        };
        // Parse source argument
        let source = matches.value_of("SOURCE").unwrap().to_string();
        // Parse target path argument
        let target  = parse_target_argument(matches);

        SplitDirectiveArgs {
            chunk_size,
            num_chunks,
            prefix,
            source,
            target,
        }
    }
}

#[doc(hidden)]
fn atm_split_gen_chunk_filename(prefix: &str, filename_base: &str, chunk_count: u32) -> String {
    format!("{}_{}_{}.tar", prefix, filename_base, chunk_count)
}

#[doc(hidden)]
fn atm_split_gen_chunk_archive(
    target: &std::path::Path,
    prefix: &str,
    filename_base: &str,
    chunk_count: u32
) -> tar::Builder<std::io::BufWriter<std::fs::File>> {
    let filepath = atm_split_gen_chunk_filename(prefix, filename_base, chunk_count);
    let filepath = target.join(&filepath);
    tar::Builder::new(std::io::BufWriter::new(std::fs::File::create(filepath.as_path()).unwrap()))
}

pub fn atm_split(args: SplitDirectiveArgs) {
    // Ensure source is file and exists
    let source = std::path::Path::new(&args.source);
    if !source.is_file() {
        panic!("Source must point to an existing TAR archive");
    }

    // Ensure target is existing directory
    let target = std::path::Path::new(&args.target);
    if !target.is_dir() {
        panic!("Target must point to an existing directory");
    }

    // Read size of source archive
    let source_size = source.metadata().unwrap().len();
    println!("::: INFO: Source TAR archive is {} bytes", source_size);

    // Calculate output chunks (maximum) size
    let chunk_maximum_size: u64 = match args.chunk_size {
        None => (((source_size as f64) / (args.num_chunks.unwrap() as f64)).round() as u64),
        Some(chunk_size) => {
            if chunk_size >= source_size {
                panic!(
                    "Chunk size must be less than source TAR archive size ({} >= {})",
                    chunk_size,
                    source_size
                );
            } else { chunk_size }
        },
    };
    println!("::: INFO: Maximum chunk size will be {} bytes", chunk_maximum_size);

    // Generate output archives base filename from source archive file stem
    let chunk_filename_base = source.file_stem().unwrap().to_str().unwrap();

    // Read source as TAR archive
    let source = std::fs::File::open(source).unwrap();
    let mut source = tar::Archive::new(source);

    // Initialize loop variable state
    let mut current_chunk_size: u64 = 0;
    let mut chunk_count: u32 = 0;
    let mut archive_chunk = atm_split_gen_chunk_archive(
        &target,
        &args.prefix,
        chunk_filename_base,
        chunk_count
    );

    // For each entry in the source archive
    for entry in source.entries().unwrap() {
        // Unwrap archive entry
        let mut entry = entry.unwrap();
        // Copy header and check entry size
        let mut entry_header = entry.header().clone();
        let entry_size = entry_header.entry_size().unwrap();
        // If adding entry would make chunk large than maximum chunk size
        if current_chunk_size + entry_size > chunk_maximum_size {
            println!("::: INFO: Flushing chunk {} to disk", chunk_count);
            // Flush current chunk to disk
            archive_chunk.finish().unwrap();
            // Increment chunk count
            chunk_count = chunk_count + 1;
            // Generate new archive
            archive_chunk = atm_split_gen_chunk_archive(
                &target,
                &args.prefix,
                chunk_filename_base,
                chunk_count
            );
            // Reset current chunk size
            current_chunk_size = 0;
        }
        // Extract entry path
        let entry_path  = entry.path().unwrap().to_path_buf();
        // Add entry to archive chunk
        archive_chunk.append_data(
            &mut entry_header,
            entry_path,
            entry.by_ref()
        ).unwrap();
        // Increment current chunk size by size of header plus entry size
        // (each aligned to 512 bytes).
        current_chunk_size = current_chunk_size + 512 + (if entry_size > 512 {entry_size} else {512});
    }

    // Flush final chunk to disk
    println!("::: INFO: Flushing final chunk to disk");
    archive_chunk.finish().unwrap();
}
