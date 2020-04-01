// batch_tar_file.rs
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

use crate::storage::{
    IntoInner,
    MIDIHashPathGenerator,
    PartitionPathGenerator,
    PathGeneratorError,
    StorageBackend,
    StorageState,
    TarArchive,
    TarArchiveError,
};

/// Type alias for `BatchTarFile` archive inner object
type ArchiveInnerObject = std::io::BufWriter<std::fs::File>;

/// Type alias for `BatchTarFile` batch inner object
type BatchInnerObject = GzEncoder<std::io::BufWriter<Vec<u8>>>;

/// Nested [tar archive](https://en.wikipedia.org/wiki/Tar_(computing)) storage backend,
/// where each entry in the archive is a 
/// [gzip](https://en.wikipedia.org/wiki/Gzip)-compressed tar archive containing MIDI files.
/// Use for the largest datasets where compression, or output file size, is of the utmost
/// importance. Choosing a batch size (and compression level) such that each compressed tar
/// archive aligns with 512 bytes will ensure that no space is wasted in the top-level archive.
/// For example, if a batch size of 25 compresses to 515 bytes, then each entry will take `1,536`
/// bytes (512 for header plus 1024 for data). However, if a batch compresses to 510 bytes,
/// then each entry will take 1024 bytes, with only 2 bytes extra. Keep in mind that higher
/// compression levels will reduce throughput of the program.
pub struct BatchTarFile {
    /// Top-level archive file
    archive: tar::Builder<ArchiveInnerObject>,
    /// Batch archive buffer
    batch_archive: TarArchive<BatchInnerObject, MIDIHashPathGenerator>,
    /// Compression level to use for batch archive
    batch_compression: Compression,
    /// Number of files in current batch
    batch_count: u32,
    /// Maximum number of files per batch
    batch_size: u32,
    /// Permissions to use for entries in top-level archive file
    batch_mode: Option<u32>,
    /// Current batch number within partition
    batch_number: u32,
    /// Current partition path
    partition: String,
    /// Partition path generator
    path_generator: PartitionPathGenerator,
    /// Top-level archive file state
    state: StorageState,
}

impl BatchTarFile {
    /// Generate new batch archive
    fn gen_batch_archive(compression_level: Compression) -> TarArchive<BatchInnerObject, MIDIHashPathGenerator> {
        TarArchive::new(
            GzEncoder::new(
                std::io::BufWriter::new(Vec::with_capacity(512)),
                compression_level,
            ),
            MIDIHashPathGenerator,
        )
    }

    /// Create new `BatchTarFile` instance
    pub fn new<P: AsRef<std::path::Path>>(
        target_path: P,
        batch_size: u32,
        num_notes: u32,
        melody_length: u32,
        max_files: u32,
        partition_depth: u32,
        batch_compression: Option<Compression>,
        batch_mode: Option<u32>,
    ) -> Result<Self, TarArchiveError> {
        // Validate batch entries mode (must be integer <= 777)
        if let Some(mode) = batch_mode {
            if mode > 777 {
                return Err(TarArchiveError::IOError(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Invalid file mode {}", mode),
                )));
            }
        }

        // Open target file and initialize tar builder
        let archive = tar::Builder::new(std::io::BufWriter::new(
            std::fs::File::open(target_path)?
        ));

        // Generate partition manager
        let path_generator = PartitionPathGenerator::new(
            num_notes,
            melody_length,
            max_files,
            partition_depth
        ).map_err(|e| TarArchiveError::PathGenerator(
            PathGeneratorError::PartitionPathGenerator(e)
        ))?;

        // Resolve batch compression
        let batch_compression = match batch_compression {
            Some(compression) => compression,
            None => Compression::default(),
        };

        Ok(Self {
            archive,
            batch_archive: Self::gen_batch_archive(batch_compression),
            batch_compression,
            batch_mode,
            batch_count: 0,
            batch_size,
            batch_number: 0,
            partition: String::new(),
            path_generator,
            state: StorageState::Open,
        })
    }
    
    /// Flush current batch archive to disk (if exists)
    fn flush_batch(&mut self) -> Result<(), TarArchiveError> {
        // If batch archive is open
        if self.batch_archive.state == StorageState::Open {
            // Finish batch archive
            self.batch_archive.finish()?;
            // Get Gzip encoder and finish writing data
            let encoder = self.batch_archive.get_mut();
            encoder.try_finish()?;
            // Get underlying BufWriter
            let buf_writer = encoder.get_mut();
            // Get underlying buffer (Vec<u8>)
            let raw_buffer = buf_writer.get_mut();

            // Construct path: `<partition>/batch<batch_number>.tar.gz`
            let path = format!(
                "{partition}{separator}batch{batch_number}.tar.gz",
                partition=self.partition,
                separator=&std::path::MAIN_SEPARATOR.to_string(),
                batch_number=self.batch_number,
            );

            // Construct tar header and write raw buffer data to top-level archive
            let mut header = tar::Header::new_old();
            header.set_size(raw_buffer.len() as u64);
            match self.batch_mode {
                Some(mode) => header.set_mode(mode),
                None => header.set_mode(644),
            }
            self
                .archive
                .append_data(&mut header, &path, raw_buffer.as_slice())
                .map_err(|e| TarArchiveError::IOError(e))?;
        }
        Ok(())
    }

    /// Flush current batch archive to disk (if exists), initialize new batch archive,
    /// and set batch counters appropriately.
    fn flush_and_init_batch(&mut self, is_partition_boundary: bool) -> Result<(), TarArchiveError> {
        // Flush current batch archive to disk (if exists)
        self.flush_batch()?;

        // Initialize new batch archive
        self.batch_archive = Self::gen_batch_archive(self.batch_compression);

        // Reset batch count and:
        // If partition boundary, reset batch_number
        // else increment batch_number
        self.batch_count = 0;
        if is_partition_boundary {
            self.batch_number = 0;
        } else {
            self.batch_number = self.batch_number + 1;
        }
        Ok(())
    }
}

impl StorageBackend for BatchTarFile {
    type Error = TarArchiveError;

    fn append_file(&mut self, mfile: libatm::MIDIFile, mode: Option<u32>) -> Result<(), Self::Error> {
        // Ensure archive is still open
        if self.state == StorageState::Closed {
            return Err(TarArchiveError::IOError(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Archive is closed for writing, cannot append file",
            )));
        }

        // Generate partition for MIDI file
        let partition = self.path_generator.gen_basename_for_file(&mfile)?;

        // If first MIDI file or reached partition_boundary
        if self.partition != partition {
            // Flush current batch and reset counters
            self.flush_and_init_batch(true)?;
            // Update partition
            self.partition = partition;
        // Else if just batch boundary
        } else if self.batch_count == self.batch_size {
            // FLush current batch, reset batch_count and
            // increment batch_number
            self.flush_and_init_batch(false)?;
        }

        // Add file to batch archive and increment batch_count
        self.batch_archive.append_file(mfile, mode)?;
        self.batch_count = self.batch_count + 1;
        Ok(())
    }

    fn finish(&mut self) -> Result<(), Self::Error> {
        match self.state {
            // If archive is still "open"
            StorageState::Open => {
                // Flush remaining batch to disk
                self.flush_batch()?;
                // Write footer sections to top-level archive and
                // close for writing
                self.archive.finish().map_err(|e| TarArchiveError::IOError(e))
            },
            _ => Ok(()),
        }
    }
}

impl IntoInner for BatchTarFile {
    type Inner = ArchiveInnerObject;

    fn into_inner(mut self) -> Result<Self::Inner, <Self as StorageBackend>::Error> {
        self.finish()?;
        self.archive.into_inner().map_err(|e| TarArchiveError::IOError(e))
    }
}
