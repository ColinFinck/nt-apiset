// Copyright 2023 Colin Finck <colin@reactos.org>
// SPDX-License-Identifier: MIT OR Apache-2.0

use core::iter::FusedIterator;
use core::mem;
use core::ops::Range;

use zerocopy::{FromBytes, LayoutVerified, LittleEndian, Unaligned, U32};

#[allow(dead_code)]
#[derive(Debug, FromBytes, Unaligned)]
#[repr(packed)]
pub(crate) struct ApiSetHashEntryHeader {
    hash: U32<LittleEndian>,
    index: U32<LittleEndian>,
}

/// Iterator over the [`ApiSetHashEntry`]s of an [`ApiSetMap`].
///
/// This iterator is returned by [`ApiSetMap::hash_entries`].
/// However, you are recommended to use [`ApiSetMap::find_namespace_entry`] instead of manually iterating through the hash entries.
///
/// Hash Entries are sorted by the hash value.
///
/// [`ApiSetMap`]: crate::map::ApiSetMap
/// [`ApiSetMap::find_namespace_entry`]: crate::map::ApiSetMap::find_namespace_entry
/// [`ApiSetMap::hash_entries`]: crate::map::ApiSetMap::hash_entries
#[derive(Clone, Debug)]
pub struct ApiSetHashEntries<'a> {
    section_bytes: &'a [u8],
    range: Range<usize>,
}

impl<'a> ApiSetHashEntries<'a> {
    pub(crate) const fn new(section_bytes: &'a [u8], range: Range<usize>) -> Self {
        Self {
            section_bytes,
            range,
        }
    }
}

impl<'a> Iterator for ApiSetHashEntries<'a> {
    type Item = ApiSetHashEntry<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let (header, _) = LayoutVerified::<_, ApiSetHashEntryHeader>::new_unaligned_from_prefix(
            self.section_bytes.get(self.range.clone())?,
        )?;
        let entry = ApiSetHashEntry { header };
        self.range.start += mem::size_of::<ApiSetHashEntryHeader>();

        Some(entry)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = self.range.len() / mem::size_of::<ApiSetHashEntryHeader>();
        (size, Some(size))
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        // `n` is arbitrary and usize, so we may hit boundaries here. Check that!
        let bytes_to_skip = n.checked_mul(mem::size_of::<ApiSetHashEntryHeader>())?;
        self.range.start = self.range.start.checked_add(bytes_to_skip)?;
        self.next()
    }
}

impl<'a> ExactSizeIterator for ApiSetHashEntries<'a> {}
impl<'a> FusedIterator for ApiSetHashEntries<'a> {}

/// A single Hash Entry in an [`ApiSetMap`].
///
/// These entries implement the hash table for faster lookup of [`ApiSetNamespaceEntry`]s.
/// While they are returned by the [`ApiSetHashEntries`] iterator, you are recommended to use [`ApiSetMap::find_namespace_entry`] instead of manually iterating through the hash entries.
///
/// [`ApiSetMap`]: crate::map::ApiSetMap
/// [`ApiSetMap::find_namespace_entry`]: crate::map::ApiSetMap::find_namespace_entry
/// [`ApiSetNamespaceEntry`]: crate::namespace_entry::ApiSetNamespaceEntry
#[derive(Debug)]
pub struct ApiSetHashEntry<'a> {
    header: LayoutVerified<&'a [u8], ApiSetHashEntryHeader>,
}

impl<'a> ApiSetHashEntry<'a> {
    /// Returns the hash value of this [`ApiSetHashEntry`].
    pub fn hash(&self) -> u32 {
        self.header.hash.get()
    }

    /// Returns the index of the mapped [`ApiSetNamespaceEntry`].
    ///
    /// This index corresponds to the N-th element returned by the [`ApiSetNamespaceEntries`] iterator.
    ///
    /// [`ApiSetNamespaceEntries`]: crate::namespace_entry::ApiSetNamespaceEntries
    /// [`ApiSetNamespaceEntry`]: crate::namespace_entry::ApiSetNamespaceEntry
    pub fn index(&self) -> u32 {
        self.header.index.get()
    }
}
