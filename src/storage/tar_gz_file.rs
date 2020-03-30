// tar_gz_file.rs
//
// Copyright (c) 2020 All The Music, LLC
//
// This work is licensed under the Creative Commons Attribution 4.0 International License.
// To view a copy of this license, visit http://creativecommons.org/licenses/by/4.0/ or send
// a letter to Creative Commons, PO Box 1866, Mountain View, CA 94042, USA.

use flate2::write::GzEncoder;

use crate::storage::{
    IntoInner,
    PathGenerator,
    StorageBackend,
    TarArchive,
};

/// Type alias for `TarGzFile` inner object
type InnerObject = GzEncoder<std::io::BufWriter<std::fs::File>>;

/// [Gzip](https://en.wikipedia.org/wiki/Gzip)-compressed
/// [tar archive](https://en.wikipedia.org/wiki/Tar_(computing)) storage backend.
/// Use for larger datasets where smaller output file is desired (i.e., for sharing via email or
/// messaging apps). Realized compression ratio will depend on the `compression_level` used,
/// as well as the compressibility of the input data.
pub struct TarGzFile<G: PathGenerator> {
    archive: TarArchive<InnerObject, G>,
}

impl<G: PathGenerator> TarGzFile<G> {
    /// Create new `TarGzFile` instance. If no compression level specified,
    /// uses default compression level as implemented in
    /// [flate2::Compression](../../../flate2/struct.Compression.html#method.default).
    pub fn new<P: AsRef<std::path::Path>>(
        target_path: P,
        path_generator: G,
        compression_level: Option<flate2::Compression>,
    ) -> std::io::Result<Self> {
        // Open filepath
        let archive = std::fs::File::open(target_path)?;
        // Wrap in BufWriter, optimized for many small writes
        // (see: https://doc.rust-lang.org/std/io/struct.BufWriter.html)
        let archive = std::io::BufWriter::new(archive);
        // Create Gzip encoder with file as underlying buffer
        // If no compression level provided, use default compression level
        // as implemented in flate2::Compression::default
        let archive = flate2::write::GzEncoder::new(
            archive,
            match compression_level {
                Some(level) => level,
                None => flate2::Compression::default(),
            },
        );
        Ok(Self {
            archive: TarArchive::new(archive, path_generator),
        })
    }
}

impl<G: PathGenerator> StorageBackend for TarGzFile<G> {
    type Error = <TarArchive<InnerObject, G> as StorageBackend>::Error;

    fn append_file(&mut self, mfile: libatm::MIDIFile, mode: Option<u32>) -> Result<(), Self::Error> {
        self.archive.append_file(mfile, mode)
    }

    fn finish(&mut self) -> Result<(), Self::Error> {
        // NOTE: The underlying flate2::write::GzEncoder implements std::ops::Drop,
        // and thus will finish itself when it goes out of scope
        self.archive.finish()
    }
}

impl<G: PathGenerator> IntoInner for TarGzFile<G> {
    type Inner = InnerObject;

    fn into_inner(self) -> Result<Self::Inner, <Self as StorageBackend>::Error> {
        self.archive.into_inner()
    }
}
