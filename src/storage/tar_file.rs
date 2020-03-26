// tar_file.rs
//
// Copyright (c) 2020 All The Music, LLC
//
// This work is licensed under the Creative Commons Attribution 4.0 International License.
// To view a copy of this license, visit http://creativecommons.org/licenses/by/4.0/ or send
// a letter to Creative Commons, PO Box 1866, Mountain View, CA 94042, USA.

use crate::storage::{
    IntoInner,
    PathGenerator,
    StorageBackend,
    TarArchive,
    TarArchiveError,
};

/// Type alias for `TarFile` inner object
type InnerObject = std::io::BufWriter<std::fs::File>;

/// [Tar archive](https://en.wikipedia.org/wiki/Tar_(computing)) storage backend.
/// Use for datasets where compression isn't necessary. For reference, a tar archive
/// with 4096 entries will take up ~4.19MB on disk, as each entry requires a header of 512
/// bytes and at least 512 bytes of data.
pub struct TarFile<G: PathGenerator> {
    archive: TarArchive<InnerObject, G>,
}

impl<G: PathGenerator> TarFile<G> {
    /// Create new `TarFile` instance
    pub fn new<P: AsRef<std::path::Path>>(target_path: P, path_generator: G) -> Result<Self, TarArchiveError> {
        // Open filepath
        let archive = std::fs::File::open(target_path)?;
        // Wrap in BufWriter, optimized for many small writes
        // (see: https://doc.rust-lang.org/std/io/struct.BufWriter.html)
        let archive = std::io::BufWriter::new(archive);
        Ok(Self {
            archive: TarArchive::new(archive, path_generator),
        })
    }
}

impl<G: PathGenerator> StorageBackend for TarFile<G> {
    type Error = <TarArchive<InnerObject, G> as StorageBackend>::Error;

    fn append_file(&mut self, mfile: libatm::MIDIFile, mode: Option<u32>) -> Result<(), Self::Error> {
        self.archive.append_file(mfile, mode)
    }

    fn append_melody(&mut self, melody: Vec<libatm::MIDINote>, mode: Option<u32>) -> Result<(), Self::Error> {
        self.archive.append_melody(melody, mode)
    }
    
    fn finish(&mut self) -> Result<(), Self::Error> {
        self.archive.finish()
    }
}

impl<G: PathGenerator> IntoInner for TarFile<G> {
    type Inner = InnerObject;

    fn into_inner(self) -> Result<Self::Inner, <Self as StorageBackend>::Error> {
        self.archive.into_inner()
    }
}
