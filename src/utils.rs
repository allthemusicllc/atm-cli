// utils.rs
//
// Copyright (c) 2019 All The Music, LLC
//
// This work is licensed under the Creative Commons Attribution 4.0 International License.
// To view a copy of this license, visit http://creativecommons.org/licenses/by/4.0/ or send
// a letter to Creative Commons, PO Box 1866, Mountain View, CA 94042, USA.

#![allow(unused_parens)]

extern crate flate2;
extern crate itertools;
extern crate libatm;
extern crate tar;

use std::io::Write;

use itertools::Itertools;

/// Calculate number of MIDI notes per partition.
///
/// # Arguments
///
/// * `num_notes`: number of possible notes (duplicates allowed)
/// * `length`: length of generated MIDI sequences (see: [gen_sequences](fn.gen_sequences.html))
/// * `max_file_count`: maximum number of files per partition (recommended <= 4K)
/// * `partition_depth`: number of partitions
///
/// # Preconditions
///
/// * `num_notes` > 0
/// * `length` > 0
/// * `max_files_count` > 0
/// * `partition_depth` > 0
/// * `partition_depth` <= `num_notes`
/// * if `num_notes`<sup>`length`</sup> <= `max_file_count`, `partition_depth` == 1
///
/// # Examples
///
/// ```rust
/// let partition_size = gen_partition_size(8, 12, 4096, 2);
///
/// // With these parameters, the calculation would be:
/// // https://www.wolframalpha.com/input/?i=ceil%28log64%28max%288%5E12%2F4096%2C+1%29%29%29
/// assert_eq!(4, partition_size)
/// ```
///
/// # Notes:
///
/// Most modern filesystems do not perform well with more than 4K files per folder,
/// and XFS is no exception.  In order to prevent overloading a single folder, we decided
/// to partition sequences by their hash, which is just the integer representation of
/// the MIDI notes in the sequence (see: [libatm::MIDIFile::gen_hash](../../libatm/struct.MIDIFile.html#method.gen_hash)),
/// such that no folder would contain more than `max_file_count` number of files. The formula works as follows:
///
/// let N = num_notes<sup>length</sup> <b>(number of files to be generated)</b> </br>
/// let D = max(N / max_file_count, 1) <b>(maximum number of directories to generate)</b> </br>
/// let B = num_notes<sup>partition_depth</sup> <b>(logarithm base)</b> </br>
/// partition_size = <b>ceil(log<sub>B</sub>(D))</b>
///
/// NOTE: If `num_notes` is 1 or num_notes<sup>length</sup> is less than or equal to
/// `max_file_count` then the function will simply return `length` (see Preconditions above).
pub fn gen_partition_size(
    num_notes: f32,
    length: i32,
    max_file_count: f32,
    partition_depth: i32,
) -> u32 {
    if partition_depth > (num_notes as i32) {
        panic!("Partition depth must be greater than number of notes in MIDI sequence");
    }
    let num_sequences = num_notes.powi(length);
    if num_notes == 1.0 || ((num_sequences as f32) <= max_file_count) {
        if partition_depth > 1 {
            panic!("Total number of sequences is {} and max_files is {}, partition depth must be 1", num_sequences, max_file_count);
        }
        return length as u32;
    }
    let max_directories = num_sequences / max_file_count;
    let base = num_notes.powi(partition_depth);
    max_directories.log(base).ceil() as u32
}

/// Generate a path representing the partition for the given MIDI hash separated by
/// the OS path separator.
///
/// # Arguments
///
/// * `hash`: MIDI hash (see: [libatm::MIDIFile](../../libatm/struct.MIDIFile.html))
/// * `partition_size`: number of MIDI notes per partition
/// * `parition_depth`: number of partitions
///
/// # Examples
///
/// ```rust
/// // Hash of length 12 with 8 distinct notes
/// let hash = "606060606467717262616464";
/// // Partition_depth is 2
/// let partition_depth = 2;
/// let partition_size = gen_partition_size(8.0, 12, 4096.0, partition_depth);
/// // Generate path
/// let path = gen_path(&hash, partition_size, partition_depth);
///
/// assert_eq!("60606060/64677172", path);
/// ```
pub fn gen_path(hash: &str, partition_size: u32, partition_depth: u32) -> String {
    if (hash.len() as u32) < (partition_size * 2 * partition_depth) {
        panic!("Hash has insufficient length ({}) for partition size {} and partition_depth {}", hash.len(), partition_size, partition_depth);
    }
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
/// Tar archives as created by the [tar](../../tar/index.html) crate are either `Open`
/// or `Closed` and, once `Closed`, cannot be modified by the program.
/// This enum is a simple way to track the state of the underlying tar archive
/// in a [BatchedMIDIArchive](struct.BatchedMIDIArchive.html).
#[derive(PartialEq)]
pub enum BatchedMIDIArchiveState {
    Open,
    Closed,
}

/// Container for tar archive of MIDI files
///
/// `BatchedMIDIArchive` is a convenience wrapper around functionality
/// to create tar archives of batches of MIDI files. Most hard drives and OS's
/// align files to 512 bytes, and the official tar spec aligns headers and
/// entries the same way.  However, MIDI files generated by
/// [libatm::MIDIFile](../../libatm/struct.MIDIFile.html), depending on sequence length,
/// tend to be much smaller (95 bytes for a 12-note sequence).  Thus, in order to maximize
/// disk space usage, this class bundles batches of MIDI files (compressed tar archives)
/// into each entry in the output tar archive.  During testing, All the Music was able to
/// compress up to 25 MIDI files per batch using the [flate2](../../flate2/index.html) crate
/// with the [default compression level](../../src/flate2/lib.rs.html#223-225).
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

#[cfg(test)]
mod tests {
    use super::*;

    /************************************/
    /***** gen_partition_size tests *****/
    /************************************/

    #[test]
    fn test_gen_partition_size_1_1_4096_1() {
        let partition_size = gen_partition_size(1.0, 1, 4096.0, 1);
        assert_eq!(1, partition_size);
    }

    #[test]
    fn test_gen_partition_size_2_8_4096_1() {
        let partition_size = gen_partition_size(2.0, 8, 4096.0, 1);
        assert_eq!(8, partition_size);
    }

    #[test]
    #[should_panic]
    fn test_gen_partition_size_2_8_4096_2() {
        let _partition_size = gen_partition_size(2.0, 8, 4096.0, 2);
    }

    #[test]
    fn test_gen_partition_size_8_8_4096_1() {
        let partition_size = gen_partition_size(8.0, 8, 4096.0, 1);
        assert_eq!(4, partition_size);
    }

    #[test]
    fn test_gen_partition_size_8_8_4096_2() {
        let partition_size = gen_partition_size(8.0, 8, 4096.0, 2);
        assert_eq!(2, partition_size);
    }

    #[test]
    fn test_gen_partition_size_8_12_4096_1() {
        let partition_size = gen_partition_size(8.0, 12, 4096.0, 1);
        assert_eq!(8, partition_size);
    }

    #[test]
    fn test_gen_partition_size_8_12_4096_2() {
        let partition_size = gen_partition_size(8.0, 12, 4096.0, 2);
        assert_eq!(4, partition_size);
    }

    #[test]
    fn test_gen_partition_size_9_13_4096_2() {
        let partition_size = gen_partition_size(9.0, 13, 4096.0, 2);
        assert_eq!(5, partition_size);
    }

    #[test]
    fn test_gen_partition_size_9_13_256_2() {
        let partition_size = gen_partition_size(9.0, 13, 256.0, 2);
        assert_eq!(6, partition_size);
    }

    #[test]
    fn test_gen_partition_size_9_13_4096_3() {
        let partition_size = gen_partition_size(9.0, 13, 4096.0, 3);
        assert_eq!(4, partition_size);
    }

    /**************************/
    /***** gen_path tests *****/
    /**************************/

    fn gen_os_path(components: Vec<&str>) -> String {
        components.join(&std::path::MAIN_SEPARATOR.to_string())
    }

    #[test]
    fn test_gen_path_8_12_2() {
        let partition_depth: u32 = 2;
        let partition_size = gen_partition_size(8.0, 12, 4096.0, partition_depth as i32);
        let hash = "606060606467717262616464";
        let path = gen_path(hash, partition_size, partition_depth);
        assert_eq!(gen_os_path(vec!["60606060", "64677172"]), path);
    }

    #[test]
    fn test_gen_path_8_12_1() {
        let partition_depth: u32 = 1;
        let partition_size = gen_partition_size(8.0, 12, 4096.0, partition_depth as i32);
        let hash = "606060606467717262616464";
        let path = gen_path(hash, partition_size, partition_depth);
        assert_eq!("6060606064677172", path);
    }

    #[test]
    fn test_gen_path_9_9_3() {
        let partition_depth: u32 = 3;
        let partition_size = gen_partition_size(9.0, 9, 4096.0, partition_depth as i32);
        let hash = "606060606467717262";
        let path = gen_path(hash, partition_size, partition_depth);
        assert_eq!(gen_os_path(vec!["6060", "6060", "6467"]), path);
    }

    #[test]
    fn test_gen_path_9_6_3() {
        let partition_depth: u32 = 3;
        let partition_size = gen_partition_size(9.0, 6, 4096.0, partition_depth as i32);
        let hash = "606060606467";
        let path = gen_path(hash, partition_size, partition_depth);
        assert_eq!(gen_os_path(vec!["60", "60", "60"]), path);
    }

    #[test]
    #[should_panic]
    fn test_gen_path_9_6_3_4() {
        let partition_depth: u32 = 3;
        let partition_size: u32 = 4;
        let hash = "606060606467";
        let _path = gen_path(hash, partition_size, partition_depth);
    }

    #[test]
    fn test_gen_path_9_13_4() {
        let partition_depth: u32 = 4;
        let partition_size = gen_partition_size(9.0, 13, 4096.0, partition_depth as i32);
        let hash = "60606060646760606060646760";
        let path = gen_path(hash, partition_size, partition_depth);
        assert_eq!(gen_os_path(vec!["606060", "606467", "606060", "606467"]), path);
    }
}
