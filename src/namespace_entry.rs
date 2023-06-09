// Copyright 2023 Colin Finck <colin@reactos.org>
// SPDX-License-Identifier: MIT OR Apache-2.0

use core::iter::FusedIterator;
use core::mem;
use core::ops::Range;

use bitflags::bitflags;
use nt_string::u16strle::U16StrLe;
use zerocopy::{FromBytes, LayoutVerified, LittleEndian, Unaligned, U32};

use crate::error::{NtApiSetError, Result};
use crate::value_entry::{ApiSetValueEntries, ApiSetValueEntryHeader};

#[allow(dead_code)]
#[derive(Debug, FromBytes, Unaligned)]
#[repr(packed)]
pub(crate) struct ApiSetNamespaceEntryHeader {
    /// See [`ApiSetNamespaceEntryFlags`]
    flags: U32<LittleEndian>,
    name_offset: U32<LittleEndian>,
    name_length: U32<LittleEndian>,
    hashed_length: U32<LittleEndian>,
    array_offset: U32<LittleEndian>,
    array_count: U32<LittleEndian>,
}

bitflags! {
    /// Flags returned by [`ApiSetNamespaceEntry::flags`].
    pub struct ApiSetNamespaceEntryFlags: u32 {
        /// This API Set Namespace Entry is sealed, meaning the loader shall not look for a schema extension.
        const SEALED = 1 << 0;
        /// This API Set Namespace Entry is an extension (begins with "ext-" and not with "api-").
        const IS_EXTENSION = 1 << 1;
    }
}

/// Iterator over the [`ApiSetNamespaceEntry`]s of an [`ApiSetMap`].
///
/// This iterator is returned by [`ApiSetMap::namespace_entries`].
///
/// Namespace Entries are sorted case-insensitively by the name of the API Set.
///
/// [`ApiSetMap`]: crate::map::ApiSetMap
/// [`ApiSetMap::namespace_entries`]: crate::map::ApiSetMap::namespace_entries
#[derive(Clone, Debug)]
pub struct ApiSetNamespaceEntries<'a> {
    section_bytes: &'a [u8],
    range: Range<usize>,
}

impl<'a> ApiSetNamespaceEntries<'a> {
    pub(crate) const fn new(section_bytes: &'a [u8], range: Range<usize>) -> Self {
        Self {
            section_bytes,
            range,
        }
    }
}

impl<'a> Iterator for ApiSetNamespaceEntries<'a> {
    type Item = ApiSetNamespaceEntry<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let (header, _) =
            LayoutVerified::<_, ApiSetNamespaceEntryHeader>::new_unaligned_from_prefix(
                self.section_bytes.get(self.range.clone())?,
            )?;
        let entry = ApiSetNamespaceEntry {
            section_bytes: self.section_bytes,
            position: self.range.start,
            header,
        };
        self.range.start += mem::size_of::<ApiSetNamespaceEntryHeader>();

        Some(entry)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = self.range.len() / mem::size_of::<ApiSetNamespaceEntryHeader>();
        (size, Some(size))
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        // `n` is arbitrary and usize, so we may hit boundaries here. Check that!
        let bytes_to_skip = n.checked_mul(mem::size_of::<ApiSetNamespaceEntryHeader>())?;
        self.range.start = self.range.start.checked_add(bytes_to_skip)?;
        self.next()
    }
}

impl<'a> ExactSizeIterator for ApiSetNamespaceEntries<'a> {}
impl<'a> FusedIterator for ApiSetNamespaceEntries<'a> {}

/// A single Namespace Entry in an [`ApiSetMap`].
///
/// Such entries are returned by the [`ApiSetNamespaceEntries`] iterator as well as the [`ApiSetMap::find_namespace_entry`] function.
///
/// [`ApiSetMap`]: crate::map::ApiSetMap
/// [`ApiSetMap::find_namespace_entry`]: crate::map::ApiSetMap::find_namespace_entry
#[derive(Debug)]
pub struct ApiSetNamespaceEntry<'a> {
    section_bytes: &'a [u8],
    position: usize,
    header: LayoutVerified<&'a [u8], ApiSetNamespaceEntryHeader>,
}

impl<'a> ApiSetNamespaceEntry<'a> {
    /// Returns flags set for this [`ApiSetNamespaceEntry`] as specified by [`ApiSetNamespaceEntryFlags`].
    pub fn flags(&self) -> ApiSetNamespaceEntryFlags {
        ApiSetNamespaceEntryFlags::from_bits_truncate(self.header.flags.get())
    }

    /// Returns the name of this API Set Namespace Entry.
    ///
    /// This name should begin with either "api-" or "ext-".
    /// It does not end with a file extension.
    pub fn name(&self) -> Result<U16StrLe<'a>> {
        let start = self.header.name_offset.get() as usize;
        let length = self.header.name_length.get() as usize;
        let end = start + length;
        let range = start..end;

        let name_bytes =
            self.section_bytes
                .get(range.clone())
                .ok_or(NtApiSetError::EntryNameOutOfBounds {
                    name_range: range,
                    entry_offset: self.position,
                    actual: self.section_bytes.len(),
                })?;

        Ok(U16StrLe(name_bytes))
    }

    /// Returns an iterator over the [`ApiSetValueEntry`]s of this [`ApiSetNamespaceEntry`].
    ///
    /// These entries describe the mapping destination of an API Set Namespace Entry.
    ///
    /// [`ApiSetValueEntry`]: crate::value_entry::ApiSetValueEntry
    pub fn value_entries(&self) -> Result<ApiSetValueEntries<'a>> {
        let start = self.header.array_offset.get() as usize;
        let count = self.header.array_count.get() as usize;
        let end = start + mem::size_of::<ApiSetValueEntryHeader>() * count;
        let range = start..end;

        self.section_bytes
            .get(range.clone())
            .ok_or(NtApiSetError::ValueEntriesOutOfBounds {
                range: start..end,
                actual: self.section_bytes.len(),
            })?;

        Ok(ApiSetValueEntries::new(self.section_bytes, range))
    }
}
