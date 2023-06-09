// Copyright 2023 Colin Finck <colin@reactos.org>
// SPDX-License-Identifier: MIT OR Apache-2.0

use core::iter::FusedIterator;
use core::mem;
use core::ops::Range;

use nt_string::u16strle::U16StrLe;
use zerocopy::{FromBytes, LayoutVerified, LittleEndian, Unaligned, U32};

use crate::error::{NtApiSetError, Result};

#[allow(dead_code)]
#[derive(Debug, FromBytes, Unaligned)]
#[repr(packed)]
pub(crate) struct ApiSetValueEntryHeader {
    flags: U32<LittleEndian>,
    name_offset: U32<LittleEndian>,
    name_length: U32<LittleEndian>,
    value_offset: U32<LittleEndian>,
    value_length: U32<LittleEndian>,
}

/// Iterator over the [`ApiSetValueEntry`]s of an [`ApiSetNamespaceEntry`].
///
/// This iterator is returned by [`ApiSetNamespaceEntry::value_entries`].
///
/// Value Entries are sorted case-insensitively by the name of the importing module.
/// The first entry is always the default entry with the importing module name set to an empty string.
///
/// [`ApiSetNamespaceEntry`]: crate::namespace_entry::ApiSetNamespaceEntry
/// [`ApiSetNamespaceEntry::value_entries`]: crate::namespace_entry::ApiSetNamespaceEntry::value_entries
#[derive(Clone, Debug)]
pub struct ApiSetValueEntries<'a> {
    section_bytes: &'a [u8],
    range: Range<usize>,
}

impl<'a> ApiSetValueEntries<'a> {
    pub(crate) const fn new(section_bytes: &'a [u8], range: Range<usize>) -> Self {
        Self {
            section_bytes,
            range,
        }
    }
}

impl<'a> Iterator for ApiSetValueEntries<'a> {
    type Item = ApiSetValueEntry<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let (entry_header, _) =
            LayoutVerified::<_, ApiSetValueEntryHeader>::new_unaligned_from_prefix(
                self.section_bytes.get(self.range.clone())?,
            )?;
        let entry = ApiSetValueEntry {
            section_bytes: self.section_bytes,
            position: self.range.start,
            header: entry_header,
        };
        self.range.start += mem::size_of::<ApiSetValueEntryHeader>();

        Some(entry)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = self.range.len() / mem::size_of::<ApiSetValueEntryHeader>();
        (size, Some(size))
    }
}

impl<'a> ExactSizeIterator for ApiSetValueEntries<'a> {}
impl<'a> FusedIterator for ApiSetValueEntries<'a> {}

/// A single mapping entry for an [`ApiSetNamespaceEntry`].
///
/// Such entries are returned by the [`ApiSetValueEntries`] iterator.
///
/// [`ApiSetNamespaceEntry`]: crate::namespace_entry::ApiSetNamespaceEntry
#[derive(Debug)]
pub struct ApiSetValueEntry<'a> {
    section_bytes: &'a [u8],
    position: usize,
    header: LayoutVerified<&'a [u8], ApiSetValueEntryHeader>,
}

impl<'a> ApiSetValueEntry<'a> {
    /// Returns flags set for this [`ApiSetValueEntry`].
    ///
    /// These flags are currently unknown, so a plain [`u32`] is returned.
    pub fn flags(&self) -> u32 {
        self.header.flags.get()
    }

    /// Returns the name of the importing module for this mapping.
    ///
    /// This string is always empty for the first [`ApiSetValueEntry`] of an [`ApiSetNamespaceEntry`].
    /// Furthermore, most [`ApiSetNamespaceEntry`]s only have a single [`ApiSetValueEntry`].
    ///
    /// If this string is non-empty, it ends with the file extension of the importing module.
    ///
    /// [`ApiSetNamespaceEntry`]: crate::namespace_entry::ApiSetNamespaceEntry
    pub fn name(&self) -> Result<U16StrLe<'a>> {
        let start = self.header.name_offset.get() as usize;
        let length = self.header.name_length.get() as usize;
        let end = start + length;
        let range = start..end;

        let bytes =
            self.section_bytes
                .get(range.clone())
                .ok_or(NtApiSetError::EntryNameOutOfBounds {
                    name_range: range,
                    entry_offset: self.position,
                    actual: self.section_bytes.len(),
                })?;

        Ok(U16StrLe(bytes))
    }

    /// Returns the name of the host module to which this entry is mapped.
    ///
    /// It ends with the file extension of the host module.
    pub fn value(&self) -> Result<U16StrLe<'a>> {
        let start = self.header.value_offset.get() as usize;
        let length = self.header.value_length.get() as usize;
        let end = start + length;
        let range = start..end;

        let bytes =
            self.section_bytes
                .get(range.clone())
                .ok_or(NtApiSetError::EntryNameOutOfBounds {
                    name_range: range,
                    entry_offset: self.position,
                    actual: self.section_bytes.len(),
                })?;

        Ok(U16StrLe(bytes))
    }
}
