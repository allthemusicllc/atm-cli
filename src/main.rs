// main.rs
//
// Copyright (c) 2019 All The Music, LLC
//
// This work is licensed under the Creative Commons Attribution 4.0 International License.
// To view a copy of this license, visit http://creativecommons.org/licenses/by/4.0/ or send
// a letter to Creative Commons, PO Box 1866, Mountain View, CA 94042, USA.

//! `atm-cli` is a command line tool for generating and working with MIDI files. It was purpose-built for
//! All the Music, LLC to assist in its mission to enable musicians to make all of their music
//! without the fear of frivolous copyright lawsuits. All code is released into the public domain
//! via the [Creative Commons Attribute 4.0 International License](http://creativecommons.org/licenses/by/4.0/).
//! If you're looking for a Rust library to generate and work with MIDI files, check out
//! [the `libatm` project](https://github.com/allthemusicllc/libatm), on which this tool relies. For
//! more information on All the Music, check out [allthemusic.info](http://allthemusic.info).

// Allow dead code
#![allow(unused_parens)]

extern crate clap;
extern crate itertools;
extern crate flate2;
extern crate libatm;
extern crate pbr;
extern crate tar;

use std::io::Write;

use itertools::Itertools;

/*****************************/
/***** Utility Functions *****/
/*****************************/

/// Calculate number of MIDI notes per partition.
///
/// # Arguments
///
/// * `num_notes`: number of possible notes (duplicates allowed)
/// * `length`: length of generated MIDI sequences (see: `gen_sequences`)
/// * `max_file_count`: maximum number of files per partition (recommended <= 4K)
/// * `partition_depth`: number of partitions
///
/// # Examples
///
/// ```rust
/// let partition_size = gen_partition_size(8, 12, 4096, 2);
///
/// // With these parameters, the calculation would be:
/// // https://www.wolframalpha.com/input/?i=ceil(log64(8%5E12%2F4096))
/// assert_eq!(4, partition_size)
/// ```
///
/// # Notes:
///
/// Most modern filesystems do not perform well with more than 4K files per folder,
/// and XFS is no exception.  In order to prevent overloading a single folder, we decided
/// to partition sequences by their hash, which is just the integer representation of
/// the MIDI notes in the sequence (see: `libatm::MIDIFile::gen_hash`), such that no folder
/// would contain more than `max_file_count` number of files. The formula works
/// as follows:
///
/// let N = num_notes<sup>length</sup> <b>(number of files to be generated)</b> </br>
/// let D = N / max_file_count <b>(maximum number of directories to generate)</b> </br>
/// let B = num_notes<sup>partition_depth</sup> <b>(logarithm base)</b> </br>
/// partition_size = <b>ceil(log<sub>B</sub>(D))</b>
pub fn gen_partition_size(
    num_notes: f32,
    length: i32,
    max_file_count: f32,
    partition_depth: i32,
) -> u32 {
    let num_sequences = num_notes.powi(length);
    let max_directories = num_sequences / max_file_count;
    let base = num_notes.powi(partition_depth);
    max_directories.log(base).ceil() as u32
}

/// Generate a path representing the partition for the given MIDI hash separated by
/// the OS path separator.
///
/// # Arguments
///
/// * `hash`: MIDI hash (see: `libatm::MIDIFile`)
/// * `partition_size`: number of MIDI notes per partition
/// * `parition_depth`: number of partitions
///
/// # Examples
///
/// ```rust
/// // Set hash of length 12 with 8 distinct notes
/// let hash = "606060606467717262616464";
/// // Set partition_depth to 2 and calculate partition size
/// let partition_depth = 2;
/// let partition_size = gen_partition_size(8, 12, 4096, partition_depth);
/// // Generate path
/// let path = gen_path(&hash, partitions_size, partition_depth);
///
/// assert_eq!("60606060/64677172", path);
/// ```
pub fn gen_path(hash: &str, partition_size: u32, partition_depth: u32) -> String {
    (0..partition_depth)
        .map(|part| {
            &hash[((partition_size * 2 * part) as usize)
                ..((partition_size * 2 * (part + 1)) as usize)]
        })
        .collect::<Vec<&str>>()
        .join(&std::path::MAIN_SEPARATOR.to_string())
}

/// Generate all permutations (with replacement) of given length
/// from the given sequence of MIDI notes.
///
/// # Arguments:
///
/// * `notes`: sequence of MIDI notes (see: [libatm::MIDINote](../libatm/struct.MIDINote.html))
/// * `length`: length of sequences to generate
///
/// # Examples
///
/// ```rust
/// // Create MIDI note sequence
/// let sequence = "C:4,C:4,D:4,E:4,F:4,G:5".parse::<libatm::MIDINoteSequence>().unwrap();
/// // Create iterable over all permutations, which in this example would be
/// // 6^8 = 1,679,616 instances of `Vec<&libatm::MIDINote>`.
/// let permutations = gen_sequences(&sequence, 8)
/// ```
pub fn gen_sequences(
    notes: &[libatm::MIDINote],
    length: u32,
) -> itertools::MultiProduct<std::slice::Iter<libatm::MIDINote>> {
    (0..(length))
        .map(|_| notes.iter())
        .multi_cartesian_product()
}

/// State of a [BatchedMIDIArchive](struct.BatchedMIDIArchive.html)
///
/// Tar archives as created by the [tar](../tar/index.html) crate are either `Open`
/// or `Closed` and, once `Closed`, cannot be modified by the program.
/// This enum is a simple way to track the state of the underlying tar archive
/// in a [BatchedMIDIArchive](struct.BatchedMIDIArchive.html).
#[derive(PartialEq)]
pub enum BatchedMIDIArchiveState {
    Open,
    Closed,
}

/********************************/
/***** Batched MIDI Archive *****/
/********************************/

/// Container for tar archive of MIDI files
///
/// `BatchedMIDIArchive` is a convenience wrapper around functionality
/// to create tar archives of batches of MIDI files. Most hard drives and OS's
/// align files to 512 bytes, and the official tar spec aligns headers and
/// entries the same way.  However, MIDI files generated by
/// [libatm::MIDIFile](../libatm/struct.MIDIFile.html), depending on sequence length,
/// tend to be much smaller (95 bytes for a 12-note sequence).  Thus, in order to maximize
/// disk space usage, this class bundles batches of MIDI files (compressed tar archives)
/// into each entry in the output tar archive.  During testing, All the Music was able to
/// compress up to 25 MIDI files per batch using the [flate2](../flate2/index.html) crate
/// with the [default compression level](../src/flate2/lib.rs.html#223-225).
///
/// # Examples
///
/// ```rust
/// // Assumes sequence length of 10, 8 possible notes, and partition depth of 2
/// let mut archive = BatchedMIDIArchive::new(
///     "archive.tar",
///     2,
///     4096,
///     partition_size: gen_partition_size(8.0, 10, 4096, 2),
///     20
/// );
/// let sequence = "C:4,D:4,E:4,C:4,D:4,E:4,C:4,D:4,E:4,C:4"
///     .parse::<libatm::MIDINoteSequence>()
///     .unwrap();
/// let mfile = libatm::MIDIFile::new(sequence, libatm::MIDIFormat::0, 1, 1);
/// archive.push(mfile).unwrap();
/// archive.finish().unwrap();
/// ```
pub struct BatchedMIDIArchive {
    /// Number of partitions
    pub partition_depth: u32,
    /// Maximum number of files per partition
    pub max_files: f32,
    /// Number of MIDI notes per partition
    pub partition_size: u32,
    /// Number of MIDI files per batch
    pub batch_size: u32,
    /// Whether archive is `Open` or `Closed`
    pub state: BatchedMIDIArchiveState,
    current_partition: String,
    file_count: u64,
    target_archive: tar::Builder<std::io::BufWriter<std::fs::File>>,
    batch_archive: tar::Builder<Vec<u8>>,
    batch_encoder: flate2::write::GzEncoder<Vec<u8>>,
}

impl BatchedMIDIArchive {
    fn gen_archive_from_buffer<W>(buffer: W) -> tar::Builder<W>
    where
        W: Write,
    {
        // Create tarball archive with HeaderMode::Deterministic
        // (see: https://docs.rs/tar/0.4.26/tar/enum.HeaderMode.html)
        let mut archive = tar::Builder::new(buffer);
        archive.mode(tar::HeaderMode::Deterministic);
        archive
    }

    fn gen_archive_as_vec(capacity: usize) -> tar::Builder<Vec<u8>> {
        // Create underlying buffer with specified capacity
        let buffer = match capacity {
            0 => Vec::new(),
            _ => Vec::with_capacity(capacity),
        };
        BatchedMIDIArchive::gen_archive_from_buffer(buffer)
    }

    fn gen_archive_as_file(target_path: &str) -> tar::Builder<std::io::BufWriter<std::fs::File>> {
        // Create underlying file at specified path
        let buffer = std::fs::File::create(target_path).unwrap();
        let buffer = std::io::BufWriter::new(buffer);
        BatchedMIDIArchive::gen_archive_from_buffer(buffer)
    }

    fn gen_encoder_from_buffer<W>(buffer: W) -> flate2::write::GzEncoder<W>
    where
        W: Write,
    {
        // Create gzip encoder with default compression level
        // (see: https://docs.rs/flate2/1.0.9/flate2/struct.Compression.html)
        flate2::write::GzEncoder::new(buffer, flate2::Compression::default())
    }

    fn gen_encoder(capacity: usize) -> flate2::write::GzEncoder<Vec<u8>> {
        // Create underlying buffer with specified capacity
        let buffer = match capacity {
            0 => Vec::new(),
            _ => Vec::with_capacity(capacity),
        };
        BatchedMIDIArchive::gen_encoder_from_buffer(buffer)
    }

    /// Create new `BatchedMIDIArchive`
    pub fn new(
        target_path: &str,
        partition_depth: u32,
        max_files: f32,
        partition_size: u32,
        batch_size: u32,
    ) -> BatchedMIDIArchive {
        // Create and initialize final archive file
        let target_archive = BatchedMIDIArchive::gen_archive_as_file(target_path);

        // Create and initialize batch archive/encoder
        // NOTE: Assumes each MIDIFile is <= 512 bytes in length
        // and each compressed batch of MIDI files will be <= 512 bytes
        // (due to TAR archives being aligned to 512 bytes)
        let batch_archive = BatchedMIDIArchive::gen_archive_as_vec((batch_size * 1024) as usize);
        let batch_encoder = BatchedMIDIArchive::gen_encoder(512);

        BatchedMIDIArchive {
            partition_depth,
            max_files,
            partition_size,
            batch_size,
            current_partition: String::new(),
            file_count: 0,
            state: BatchedMIDIArchiveState::Open,
            target_archive,
            batch_archive,
            batch_encoder,
        }
    }

    fn assert_open(&self) {
        if let BatchedMIDIArchiveState::Closed = self.state {
            panic!("Archive is already in the Closed state");
        }
    }

    fn gen_batch_size(&self) -> u32 {
        // Each entry in tarball is guaranteed to be 1024 bytes
        // if each MIDIFile is <= 512 bytes in length
        (self.batch_archive.get_ref().len() / 1024) as u32
    }

    fn flush(&mut self) -> std::io::Result<()> {
        // Finish batch archive
        self.batch_archive.finish()?;
        // Compress batch archive and finish encoding
        self.batch_encoder.write_all(self.batch_archive.get_ref())?;
        self.batch_encoder.try_finish()?;

        // Create tar header for entry in target archive
        let mut header = tar::Header::new_old();
        header.set_size(self.batch_encoder.get_ref().len() as u64);

        // Write header and compressed batch archive
        // to target archive
        // NOTE: Current batch number is calculated as
        // ((FILE_COUNT % MAX_FILES) / BATCH_SIZE) + 1
        // because:
        //  1) FILE_COUNT must be modded MAX_FILES as FILE_COUNT is
        //     strictly increasing
        //  2) (FILE_COUNT % MAX_FILES / BATCH_SIZE) yields the batch
        //     number of the previous batch.  To see this, assume FILE_COUNT is 4086,
        //     MAX_FILES is 4096, and BATCH_SIZE is 18.  4086 % 4096 is 4086, and
        //     4086 / 18 is 227.
        //  3) 227 + 1 = __228__.  This is correct because 4096 / 18 = 227.556,
        //     thus requiring 228 batches.
        self.target_archive.append_data(
            &mut header,
            format!(
                "{}/batch{}.tar.gz",
                &self.current_partition,
                (self.file_count.wrapping_rem(self.max_files as u64) / self.batch_size as u64) + 1
            ),
            self.batch_encoder.get_ref().as_slice(),
        )?;

        // Calculate number of files in batch archive
        // and increment file_count
        self.file_count = self.file_count + (self.gen_batch_size() as u64);

        // Reset batch archive/encoder
        self.batch_archive =
            BatchedMIDIArchive::gen_archive_as_vec((self.batch_size * 1024) as usize);
        self.batch_encoder = BatchedMIDIArchive::gen_encoder(512);

        Ok(())
    }

    /// Add a MIDI file to the archive
    ///
    /// If adding the MIDI file breaks a partition or batch boundary,
    /// this function will flush the current batch to the underlying tar
    /// archive.
    pub fn push(&mut self, mfile: libatm::MIDIFile) -> std::io::Result<()> {
        // Check archive state and panic if Closed
        self.assert_open();

        // Generate hash and partition
        let hash = mfile.gen_hash();
        let partition = gen_path(&hash, self.partition_size, self.partition_depth);

        // If partition has not been set (first batch)
        // or reached partition boundary
        if self.current_partition.is_empty() {
            self.current_partition = partition;
        } else if self.current_partition != partition {
            // Flush current batch to target archive
            self.flush()?;
            // Set new partition
            self.current_partition = partition;
        }

        // Add MIDI file to batch archive
        let mut header = tar::Header::new_old();
        header.set_size(mfile.gen_size() as u64);
        self.batch_archive.append_data(
            &mut header,
            format!("{}.mid", &hash),
            mfile.gen_buffer().unwrap().as_slice(),
        )?;

        // If reached batch boundary
        if self.gen_batch_size() == self.batch_size {
            // Flush current batch to target archive
            self.flush()?;
        }

        Ok(())
    }

    /// Flush current batch to the tar archive and set the state to `Closed`
    ///
    /// After this function is called, no more files can be written to the archive and 
    /// the [push](struct.BatchedMIDIArchive.html#method.push) function will `panic`.
    pub fn finish(&mut self) -> std::io::Result<()> {
        // Check archive state and panic if Closed
        self.assert_open();

        // If batch archive isn't empty, write out
        // compressed batch archive to target archive
        if self.gen_batch_size() > 0 {
            self.flush()?;
        }

        // Finish target archive and set state
        self.target_archive.finish()?;
        self.state = BatchedMIDIArchiveState::Closed;

        Ok(())
    }
}

/**********************/
/***** Directives *****/
/**********************/

fn atm_single(args: SingleDirectiveArgs) {
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

fn atm_batch(args: BatchDirectiveArgs) {
    // Initialize progress bar and set refresh rate
    let mut pb = pbr::ProgressBar::new(args.max_count as u64);
    pb.set_max_refresh_rate(Some(std::time::Duration::from_millis(args.update)));
    // Initialize output archive
    let mut archive = BatchedMIDIArchive::new(
        &args.target,
        args.partition_depth,
        args.max_files,
        args.partition_size,
        args.batch_size,
    );
    // For each generated sequence
    for (idx, notes) in gen_sequences(&args.sequence.notes, args.length).enumerate() {
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
    if let BatchedMIDIArchiveState::Open = archive.state {
        archive.finish().unwrap();
    }
}

fn atm_partition(args: PartitionDirectiveArgs) {
    println!("::: INFO: Generating MIDI file from pitch sequence");
    // Create MIDIFile from sequence
    let mfile = libatm::MIDIFile::new(args.sequence, libatm::MIDIFormat::Format0, 1, 1);
    // Generate MIDI sequence hash
    let hash = mfile.gen_hash();
    println!("::: INFO: Generating partition(s)");
    // Generate partitions
    let path = gen_path(&hash, args.partition_size, args.partition_depth);
    // Print full path with partitions
    println!("::: INFO: Path for sequence is {}/{}.mid", &path, &hash);
}

/********************************/
/***** Command Line Parsing *****/
/********************************/

#[derive(Debug)]
struct SingleDirectiveArgs {
    pub sequence: libatm::MIDINoteSequence,
    pub target: String,
}

impl<'a> From<&clap::ArgMatches<'a>> for SingleDirectiveArgs {
    fn from(matches: &clap::ArgMatches<'a>) -> SingleDirectiveArgs {
        // Generate libatm::MIDINoteSequence from notes argument
        let sequence = matches.value_of("NOTES").unwrap();
        let sequence = sequence.parse::<libatm::MIDINoteSequence>().unwrap();

        // Parse target argument
        let target = matches.value_of("TARGET").unwrap().to_string();

        SingleDirectiveArgs { sequence, target }
    }
}

#[derive(Debug)]
struct BatchDirectiveArgs {
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
        let sequence = matches.value_of("NOTES").unwrap();
        let sequence = sequence.parse::<libatm::MIDINoteSequence>().unwrap();

        // Parse length argument as integer
        let length = matches.value_of("LENGTH").unwrap();
        let length = length.parse::<u32>().unwrap();

        // Target sequence length cannot be less than # of notes
        if (length as usize) < sequence.notes.len() {
            panic!(
                "Length must be >= the number of notes in the sequence ({} < {})",
                length,
                sequence.notes.len()
            );
        }

        // Parse target argument
        let target = matches.value_of("TARGET").unwrap().to_string();

        // Parse partition_depth argument as integer
        let partition_depth = matches.value_of("PARTITION_DEPTH").unwrap();
        let partition_depth = partition_depth.parse::<u32>().unwrap();

        // Parse max_files argument and set default if not provided
        let max_files = matches.value_of("MAX_FILES");
        let max_files = match max_files {
            None => 4096.0,
            Some(files) => files.parse::<f32>().unwrap(),
        };

        // Calculate partition size (# of notes) from given arguments (see: gen_partition_size)
        let partition_size = gen_partition_size(
            sequence.notes.len() as f32,
            length as i32,
            max_files,
            partition_depth as i32,
        );

        // Parse max_count argument and set default if not provided
        let max_count = matches.value_of("COUNT");
        let max_count = match max_count {
            None => ((sequence.notes.len() as f32).powi(length as i32) as usize),
            Some(count) => {
                let count = count.parse::<usize>().unwrap();
                if count == 0 {
                    panic!("Count must be greater than 0");
                }
                count
            }
        };

        // Parse batch_size argument
        let batch_size = matches.value_of("BATCH_SIZE").unwrap();
        let batch_size = batch_size.parse::<u32>().unwrap();

        // Parse update argument and set default if not provided
        let update = matches.value_of("PB_UPDATE");
        let update: u64 = match update {
            None => 1000,
            Some(duration) => duration.parse::<u64>().unwrap(),
        };

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

#[derive(Debug)]
struct PartitionDirectiveArgs {
    pub sequence: libatm::MIDINoteSequence,
    pub partition_depth: u32,
    pub max_files: f32,
    pub partition_size: u32,
}

impl<'a> From<&clap::ArgMatches<'a>> for PartitionDirectiveArgs {
    fn from(matches: &clap::ArgMatches<'a>) -> PartitionDirectiveArgs {
        // Generate libatm::MIDINoteSequence from notes argument
        let sequence = matches.value_of("NOTES").unwrap();
        let sequence = sequence.parse::<libatm::MIDINoteSequence>().unwrap();

        // Parse partition_depth argument as integer
        let partition_depth = matches.value_of("PARTITION_DEPTH").unwrap();
        let partition_depth = partition_depth.parse::<u32>().unwrap();

        // Parse max_files argument and set default if not provided
        let max_files = matches.value_of("MAX_FILES");
        let max_files = match max_files {
            None => 4096.0,
            Some(files) => files.parse::<f32>().unwrap(),
        };

        // Calculate partition size (# of notes) from given arguments (see: gen_partition_size)
        let partition_size = gen_partition_size(
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

struct Cli<'a, 'b> {
    pub app: clap::App<'a, 'b>,
}

impl<'a, 'b> Cli<'a, 'b> {
    fn initialize_parser() -> clap::App<'a, 'b> {
        // Note list argument
        let note_sequence_argument = clap::Arg::with_name("NOTES")
            .short("n")
            .long("notes")
            .takes_value(true)
            .required(true)
            .help(
                "Comma-separated list NOTE:OCTAVE pairs (i.e., \
                 'C:4,D:4,E:4,F:4,G:4,A:4,B:4,C:5')",
            );
        // Target (output) file argument
        let target_argument = clap::Arg::with_name("TARGET")
            .short("t")
            .long("target")
            .takes_value(true)
            .required(true)
            .help("File output path (directory must exist)");
        // Directory partition depth argument
        let partition_depth_argument = clap::Arg::with_name("PARTITION_DEPTH")
            .short("p")
            .long("partitions")
            .takes_value(true)
            .required(true)
            .help(
                "Partition depth to use for output directory structure.  For \
                 example, if set to 2 the ouput directory structure would look \
                 like <part1>/<part2>/<hash>.mid.",
            );
        // Maximum files per directory argument
        let max_files_argument = clap::Arg::with_name("MAX_FILES")
            .short("m")
            .long("max-files")
            .takes_value(true)
            .help("Maximum number of files per directory (default: 4096)");
        // Command line app
        clap::App::new("atm")
            .version("0.1.0")
            .author("All The Music, LLC")
            .about(
                "Tools for generating and working with MIDI files.  \
                This app was created as part of an effort to generate \
                by brute-force billions of melodies, and is tailored for that use case."
            )
            .subcommand(clap::SubCommand::with_name("single")
                        .about("Generate single MIDI file from provided MIDI pitch sequence")
                        .arg(&note_sequence_argument)
                        .arg(&target_argument))
            .subcommand(clap::SubCommand::with_name("batch")
                        .about(
                            "Generate by brute-force MIDI files containing permutations \
                             of a sequence of MIDI pitches",
                        )
                        .arg(&note_sequence_argument)
                        .arg(&target_argument)
                        .arg(&partition_depth_argument)
                        .arg(&max_files_argument)
                        .arg(
                            clap::Arg::with_name("LENGTH")
                                .short("L")
                                .long("length")
                                .takes_value(true)
                                .required(true)
                                .help("Length of MIDI pitch sequences to generate"))
                        .arg(
                            clap::Arg::with_name("BATCH_SIZE")
                                .short("b")
                                .long("batch-size")
                                .takes_value(true)
                                .required(true)
                                .help("Number of files to batch (and zip) per archive entry"))
                        .arg(
                            clap::Arg::with_name("COUNT")
                                .short("c")
                                .long("count")
                                .takes_value(true)
                                .help("Number of sequences to iterate through (default: NOTES.len() ^ LENGTH)"))
                        .arg(
                            clap::Arg::with_name("PB_UPDATE")
                                .short("u")
                                .long("update")
                                .takes_value(true)
                                .help("Refresh rate for the progress bar (default: 1000 ms)")))
            .subcommand(clap::SubCommand::with_name("partition")
                        .about("Generate the output path from the 'batch' directive for a given MIDI pitch sequence")
                        .arg(&note_sequence_argument)
                        .arg(&partition_depth_argument)
                        .arg(&max_files_argument))
    }

    pub fn new() -> Cli<'a, 'b> {
        Cli {
            app: Cli::initialize_parser(),
        }
    }

    pub fn run(self) {
        let matches = self.app.get_matches();
        match matches.subcommand_name() {
            Some("single") => atm_single(SingleDirectiveArgs::from(
                matches.subcommand_matches("single").unwrap(),
            )),
            Some("batch") => atm_batch(BatchDirectiveArgs::from(
                matches.subcommand_matches("batch").unwrap(),
            )),
            Some("partition") => atm_partition(PartitionDirectiveArgs::from(
                matches.subcommand_matches("partition").unwrap(),
            )),
            Some(directive) => panic!(format!("Received unsupported directive '{}'", directive)),
            None => panic!(format!("Did not receive directive")),
        }
    }
}

fn main() {
    // Parse command line arguments and run program
    Cli::new().run();
}
