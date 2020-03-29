// storage.rs
//
// Copyright (c) 2020 All The Music, LLC
//
// This work is licensed under the Creative Commons Attribution 4.0 International License.
// To view a copy of this license, visit http://creativecommons.org/licenses/by/4.0/ or send
// a letter to Creative Commons, PO Box 1866, Mountain View, CA 94042, USA.

pub(crate) mod tar_archive;
pub mod batch_tar_file;
pub mod tar_file;
pub mod tar_gz_file;

pub use tar_archive::*;
pub use batch_tar_file::BatchTarFile;
pub use tar_file::TarFile;
pub use tar_gz_file::TarGzFile;

/*****************
***** Traits *****
*****************/

/// Trait to implement storage backends for MIDI files generated from
/// [libatm::MIDIFile](../../libatm/midi_file/struct.MIDIFile.html).
pub trait StorageBackend : Sized {
    /// Error type for storage operations
    type Error: std::fmt::Debug;

    /// Append MIDI file to storage backend
    fn append_file(&mut self, mfile: libatm::MIDIFile, mode: Option<u32>) -> Result<(), Self::Error>;

    /// Convert melody to MIDI file and append to storage backend
    fn append_melody(&mut self, melody: libatm::MIDINoteVec, mode: Option<u32>) -> Result<(), Self::Error> {
        // Create libatm::MIDIFile instance from melody
        let mfile = libatm::MIDIFile::new(melody, libatm::MIDIFormat::Format0, 1, 1);
        self.append_file(mfile, mode)
    }

    /// Conduct cleanup of storage backend and close for writing
    ///
    /// NOTE: For some backends this method may be a NOOP, but should always be called
    /// after the last MIDI file has been written to disk.
    fn finish(&mut self) -> Result<(), Self::Error>;
}

/// Trait to implement functionality for storage backends to expose the underlying
/// (inner) storage object.
pub trait IntoInner : StorageBackend {
    /// Type of inner object
    type Inner;

    /// Finish writing storage backends and return the inner object
    fn into_inner(self) -> Result<Self::Inner, <Self as StorageBackend>::Error>;
}

/// Error type for PathGenerator
#[derive(Debug, thiserror::Error)]
pub enum PathGeneratorError {
    #[error(transparent)]
    PartitionPathGenerator(#[from] PartitionPathGeneratorError),
}

/// Trait to generate storage path for MIDI files in storage backends
pub trait PathGenerator {
    /// Generate storage path for MIDI file
    fn gen_path_for_file(&self, mfile: &libatm::MIDIFile) -> Result<String, PathGeneratorError>;
}

/********************************
***** MIDIHashPathGenerator *****
********************************/

/// Path generator that produces the hash of a MIDI file as the filename
/// without any parent directories (see:
/// [MIDIFile::gen_hash](../../libatm/midi_file/struct.MIDIFile.html#method.gen_hash)).
/// This path generator is useful for smaller datasets.
pub struct MIDIHashPathGenerator;

impl PathGenerator for MIDIHashPathGenerator {
    fn gen_path_for_file(&self, mfile: &libatm::MIDIFile) -> Result<String, PathGeneratorError> {
        Ok(format!("{}.mid", mfile.gen_hash()))
    }
}

/*********************************
***** PartitionPathGenerator *****
*********************************/

#[derive(Debug, thiserror::Error)]
pub enum PartitionPathGeneratorError {
    #[error("Expected melody of length {expected}, found length {observed}")]
    MelodyLengthMismatch { expected: u32, observed: u32, },
    #[error("Partition depth must be less than the length of generated melodies \
            ({partition_depth} > {melody_length})")]
    PartitionDepthLongerThanMelody { partition_depth: u32, melody_length: i32, },
    #[error("Melodies of length {melody_length} cannot be partitioned with depth \
            {partition_depth} and length {partition_length}")]
    PartitionsLongerThanMelody { melody_length: u32, partition_depth: u32, partition_length: u32, },
}

/// Path generator for storage backends that support partitioned output schemes
///
/// Partitioning files by path in the output storage backend can be useful if not all files 
/// can be written to the same directory/file. For example, most modern filesystem don't perform as
/// well with more than 4K files per directory. Partitioning files into subdirectories with a
/// depth (number of partition branches) and partition length (number of notes per partition) can
/// ensure no more than some threshold files are written to a directory
/// (see: [gen_partition_length](struct.PartitionPathGenerator.html#method.gen_partition_length)).
pub struct PartitionPathGenerator {
    /// Length of melodies to generate partitions for
    melody_length: u32,
    /// Partition depth (i.e., number of partitions to generate)
    partition_depth: u32,
    /// Number of MIDI notes per partition
    partition_length: u32,
}

impl PartitionPathGenerator {
    /// Generate partition length (number of MIDI notes per partition) 
    fn gen_partition_length(
        num_notes: f32,
        num_melodies: f32,
        melody_length: i32,
        max_files: f32,
        partition_depth: u32
    ) -> Result<u32, PartitionPathGeneratorError> {
        // Generate maximum number of partition branches (directories)
        // as quotient of number of generated melodies and
        // maximum number of files per directory
        let max_partitions = num_melodies / max_files;

        let partition_length = max_partitions.log(num_notes.powi(partition_depth as i32)).ceil() as u32;
        // Ensure melody_length is at least as long as depth * length
        if (melody_length as u32) < partition_depth * partition_length {
            return Err(PartitionPathGeneratorError::PartitionsLongerThanMelody {
                melody_length: melody_length as u32,
                partition_depth: partition_depth,
                partition_length,
            });
        }
        Ok(partition_length)
    }
    
    /// Create new `PartitionPathGenerator` instance
    pub fn new(
        num_notes: f32,
        melody_length: i32,
        max_files: f32,
        partition_depth: u32
    ) -> Result<Self, PartitionPathGeneratorError> {
        // Ensure partition depth is less than length of generated melodies
        if partition_depth > melody_length as u32 {
            return Err(PartitionPathGeneratorError::PartitionDepthLongerThanMelody {
                partition_depth,
                melody_length,
            });
        }

        // Generate total number of generated melodies
        let num_melodies = num_notes.powi(melody_length);
        // If number of notes is 1, or total number of generated melodies is
        // less than max files per directory, then partition depth should be 1
        // and partition length should be 0
        let mut calc_partition_depth = 1;
        let mut calc_partition_length = 0;
        if !(num_notes == 1.0 || num_melodies <= max_files) {
            calc_partition_depth = partition_depth;
            // Generate partition length
            calc_partition_length = Self::gen_partition_length(
                num_notes,
                num_melodies,
                melody_length,
                max_files,
                partition_depth,
            )?;
        }

        Ok(Self {
            melody_length: melody_length as u32,
            partition_depth: calc_partition_depth,
            partition_length: calc_partition_length,
        })
    }

    /// Generate basename (parent directory/directories) for filepath
    fn gen_basename_for_file(&self, mfile: &libatm::MIDIFile) -> Result<String, PathGeneratorError> {
        // Ensure melody is expected length
        let melody_length = mfile.sequence.len() as u32;
        if melody_length != self.melody_length {
            return Err(PathGeneratorError::PartitionPathGenerator(
                PartitionPathGeneratorError::MelodyLengthMismatch {
                    expected: self.melody_length,
                    observed: melody_length
                }
            ));
        }
        
        match self.partition_depth {
            // if partition_depth is zero, return empty basename
            0 => Ok(String::new()),
            // Otherwise, generate partitioned path by
            //  1) Generating self.partition_depth slices of size self.partition_length over the input
            //     melody by using a sliding window method
            //  2) Converting each slice into a string of integer representations of each note in the
            //     slice
            //  3) Joining the slices together using the OS path separator
            _ => Ok((0..self.partition_depth)
                .map(|p| {
                    &mfile.sequence[
                        ( (self.partition_length * p) as usize )..( (self.partition_length * (p + 1)) as usize )
                    ]
                })
                .map(|p| p.iter().map(|n| n.convert().to_string()).collect::<Vec<String>>().join(""))
                .collect::<Vec<String>>()
                .join(&std::path::MAIN_SEPARATOR.to_string()))
        }
    }
}

impl PathGenerator for PartitionPathGenerator {
    fn gen_path_for_file(&self, mfile: &libatm::MIDIFile) -> Result<String, PathGeneratorError> {
        // Generate basename (could be "")
        let basename = self.gen_basename_for_file(mfile)?;
        // Generate filename from MIDI file hash
        let filename = format!("{}.mid", mfile.gen_hash());
        Ok(format!(
            "{}",
            std::path::Path::new(&basename)
                .join(&filename)
                .as_path()
                .to_string_lossy(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /*********************************
    ***** PartitionPathGenerator *****
    *********************************/

    #[test]
    #[should_panic]
    fn test_partition_depth_melody_length() {
        // Fails because partition depth must be less
        // less than length of melodies. Each partition branch
        // must contian at least one note, so if depth > # of notes,
        // cannnot generate enough branches from the input melody.        
        let path_generator = PartitionPathGenerator::new(3f32, 3, 4096f32, 4).unwrap();
    }

    #[test]
    #[should_panic]
    fn test_melody_length_match() {
        let path_generator = PartitionPathGenerator::new(4f32, 12, 4096f32, 2).unwrap();
        let mfile = libatm::MIDIFile::new(
            vec!["C:4", "D:5", "G:7"].iter().map(|n| n.parse::<libatm::MIDINote>().unwrap()).collect::<Vec<libatm::MIDINote>>(),
            libatm::MIDIFormat::Format0,
            1,
            1,
        );
        // Fails because melody isn't 4 notes
        path_generator.gen_path_for_file(&mfile).unwrap();
    }

    macro_rules! check_num_files_partition {
        ($test_name:ident, $note_set:expr, $melody_length:expr, $max_files:expr, $partition_depth:expr) => {
            #[test]
            fn $test_name() { 
                let notes = $note_set.parse::<libatm::MIDINoteSet>().unwrap();
                let num_notes = notes.len() as f32;
                let mut partition = String::new();
                let mut num_files_in_partition = 0;
                let path_generator = PartitionPathGenerator::new(
                    num_notes,
                    $melody_length,
                    $max_files,
                    $partition_depth,
                ).unwrap();

                for melody in crate::utils::gen_sequences(
                    &Vec::from(&notes),
                    $melody_length,
                ) { 
                    // Generate partition for melody
                    let melody_partition = path_generator.gen_basename_for_file(&libatm::MIDIFile::new(
                        melody.iter().map(|n| *n.clone()).collect::<Vec<libatm::MIDINote>>(),
                        libatm::MIDIFormat::Format0,
                        1,
                        1,
                    )).unwrap();
                    // If partition boundary check number of files in partition
                    if melody_partition != partition {
                        assert!(
                            num_files_in_partition as f32 <= $max_files,
                            "{} files in partition, maximum specified was {}",
                            num_files_in_partition,
                            $max_files,
                        );
                        num_files_in_partition = 0;
                        partition = melody_partition;
                    }
                }
            }
        }
    }
}
