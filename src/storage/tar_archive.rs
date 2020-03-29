// tar_archive.rs
//
// Copyright (c) 2020 All The Music, LLC
//
// This work is licensed under the Creative Commons Attribution 4.0 International License.
// To view a copy of this license, visit http://creativecommons.org/licenses/by/4.0/ or send
// a letter to Creative Commons, PO Box 1866, Mountain View, CA 94042, USA.

use crate::storage::{
    IntoInner,
    PathGenerator,
    PathGeneratorError,
    StorageBackend,
};

/***********************
***** StorageState *****
***********************/

/// Whether storage backend is open, or has been closed
/// for writing
#[derive(Debug, PartialEq)]
pub enum StorageState {
    /// Backend is open (can be written to)
    Open,
    /// Backend is closed (cannot be written to)
    Closed,
}

/*********************
***** TarArchive *****
*********************/

/// Error type for TarArchive (wrapping around `std::io::Error` and `PathGeneratorError`)
#[derive(Debug, thiserror::Error)]
pub enum TarArchiveError {
    /// IO error
    #[error(transparent)]
    IOError(#[from] std::io::Error),
    /// PathGenerator error
    #[error(transparent)]
    PathGenerator(#[from] PathGeneratorError),
}

/// [Tar archive](https://en.wikipedia.org/wiki/Tar_(computing)) storage backend. Used by other
/// storage backends as the underlying storage object.
pub struct TarArchive<W: std::io::Write, G: PathGenerator> {
    pub state: StorageState,
    archive: tar::Builder<W>,
    path_generator: G,
}

impl<W, G> TarArchive<W, G>
where
    W: std::io::Write,
    G: PathGenerator,
{
    /// Create new `TarArchive` instance
    pub fn new(buffer: W, path_generator: G) -> Self {
        Self {
            archive: tar::Builder::new(buffer),
            state: StorageState::Open,
            path_generator,
        }
    }

    /// Acquires a mutable reference to the underlying writer
    pub fn get_mut(&mut self) -> &mut W {
        self.archive.get_mut()
    }

    /// Acquires a reference to the underlying writer
    pub fn get_ref(&self) -> &W {
        self.archive.get_ref()
    }
}

impl<W, G> StorageBackend for TarArchive<W, G>
where
    W: std::io::Write,
    G: PathGenerator,
{
    type Error = TarArchiveError;

    fn append_file(&mut self, mfile: libatm::MIDIFile, mode: Option<u32>) -> Result<(), Self::Error> {
        // Ensure archive is stil open
        if self.state == StorageState::Closed {
            return Err(TarArchiveError::IOError(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Archive is closed for writing, cannot append file",
            )));
        }

        // Generate path from melody hash
        let path = self.path_generator.gen_path_for_file(&mfile)?;
        // Generate header for entry
        let mut header = tar::Header::new_old();
        // Set size field in header
        header.set_size(mfile.gen_size() as u64);
        // Set file permissions to provided value,
        // or 644 (rw-r-r) by default
        match mode {
            Some(mode) => header.set_mode(mode),
            None => header.set_mode(644),
        }
        // Generate buffer containing MIDI file data
        let data = mfile.gen_file()?;
        self
            .archive
            .append_data(&mut header, &path, data.as_slice())
            .map_err(|e| TarArchiveError::IOError(e))
    }

    fn finish(&mut self) -> Result<(), Self::Error> {
        // If archive is still "open" call tar.finish() and set state
        match self.state {
            StorageState::Open => {
                self.state = StorageState::Closed;
                // Write footer sections to archive and close for writing
                self.archive.finish().map_err(|e| TarArchiveError::IOError(e))
            },
            _ => Ok(())
        }
    }
}

impl<W, G> IntoInner for TarArchive<W, G>
where
    W: std::io::Write,
    G: PathGenerator,
{
    type Inner = W;

    fn into_inner(mut self) -> Result<Self::Inner, <Self as StorageBackend>::Error> {
        self.finish()?;
        self.archive.into_inner().map_err(|e| TarArchiveError::IOError(e))
    }
}
