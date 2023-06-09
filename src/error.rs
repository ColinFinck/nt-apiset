// Copyright 2023 Colin Finck <colin@reactos.org>
// SPDX-License-Identifier: MIT OR Apache-2.0

use core::ops::Range;

use displaydoc::Display;

/// Central result type of nt-apiset.
pub type Result<T, E = NtApiSetError> = core::result::Result<T, E>;

/// Central error type of nt-apiset.
#[derive(Clone, Debug, Display, Eq, PartialEq)]
pub enum NtApiSetError {
    /// Did not find the ".apiset" section in the PE file
    ApiSetSectionNotFound,
    /// The ".apiset" section in the PE file references data that is out of bounds
    ApiSetSectionOutOfBounds,
    /// Tried to read the name at byte range {name_range:?} of the entry at byte {entry_offset}, but the ".apiset" section only has a size of {actual} bytes
    EntryNameOutOfBounds {
        /// Range of bytes where the entry name was expected.
        name_range: Range<usize>,
        /// Byte offset of the entry inside the ".apiset" section.
        entry_offset: usize,
        /// Actual size of the ".apiset" section.
        actual: usize,
    },
    /// Tried to read the apiset hash entries from byte range {range:?}, but the ".apiset" section only has a size of {actual} bytes
    HashEntriesOutOfBounds {
        /// Start..end range where the hash entries were expected, as byte offsets relative to the start of the ".apiset" section.
        range: Range<usize>,
        /// Actual size of the ".apiset" section.
        actual: usize,
    },
    /// Tried to read {expected} bytes for the API Set Map header, but only {actual} bytes are left in the slice
    InvalidMapHeaderSize {
        /// Size in bytes of the API Set Map header.
        expected: usize,
        /// Actual size in bytes of the provided slice.
        actual: usize,
    },
    /// Tried to read the apiset namespace entries from byte range {range:?}, but the ".apiset" section only has a size of {actual} bytes
    NamespaceEntriesOutOfBounds {
        /// Start..end range where the namespace entries were expected, as byte offsets relative to the start of the ".apiset" section.
        range: Range<usize>,
        /// Actual size of the ".apiset" section.
        actual: usize,
    },
    /// The apiset map version ({version}) is unsupported
    UnsupportedVersion {
        /// Version number reported by the API Set Map.
        version: u32,
    },
    /// Tried to read the apiset value entries from byte range {range:?}, but the ".apiset" section only has a size of {actual} bytes
    ValueEntriesOutOfBounds {
        /// Start..end range where the value entries were expected, as byte offsets relative to the start of the ".apiset" section.
        range: Range<usize>,
        /// Actual size of the ".apiset" section.
        actual: usize,
    },
}

#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
impl std::error::Error for NtApiSetError {}
