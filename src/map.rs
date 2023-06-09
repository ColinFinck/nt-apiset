// Copyright 2023 Colin Finck <colin@reactos.org>
// SPDX-License-Identifier: MIT OR Apache-2.0

use core::cmp::Ordering;
use core::mem;

use bitflags::bitflags;
use zerocopy::{FromBytes, LayoutVerified, LittleEndian, Unaligned, U32};

use crate::error::{NtApiSetError, Result};
use crate::hash_entry::{ApiSetHashEntries, ApiSetHashEntryHeader};
use crate::namespace_entry::{
    ApiSetNamespaceEntries, ApiSetNamespaceEntry, ApiSetNamespaceEntryHeader,
};

#[allow(dead_code)]
#[derive(Debug, FromBytes, Unaligned)]
#[repr(packed)]
struct ApiSetMapHeader {
    version: U32<LittleEndian>,
    size: U32<LittleEndian>,
    /// See [`ApiSetMapFlags`]
    flags: U32<LittleEndian>,
    count: U32<LittleEndian>,
    namespace_entry_offset: U32<LittleEndian>,
    hash_entry_offset: U32<LittleEndian>,
    hash_factor: U32<LittleEndian>,
}

const APISET_VERSION_WINDOWS_10: u32 = 6;

bitflags! {
    /// Flags returned by [`ApiSetMap::flags`].
    pub struct ApiSetMapFlags: u32 {
        /// This API Set Map is sealed, meaning the loader shall not look for schema extensions.
        const SEALED = 1 << 0;
        /// This API Set Map is a schema extension.
        const IS_EXTENSION = 1 << 1;
    }
}

/// Root structure describing an API Set Map.
#[derive(Debug)]
pub struct ApiSetMap<'a> {
    section_bytes: &'a [u8],
    header: LayoutVerified<&'a [u8], ApiSetMapHeader>,
}

impl<'a> ApiSetMap<'a> {
    /// Returns flags set for this [`ApiSetMap`] as specified by [`ApiSetMapFlags`].
    pub fn flags(&self) -> ApiSetMapFlags {
        ApiSetMapFlags::from_bits_truncate(self.header.flags.get())
    }

    /// Finds a namespace entry efficiently in the hash table of the API Set Map.
    ///
    /// `namespace_entry_name` must be non-empty and only consist of lowercase characters, digits, and hyphens.
    /// This is asserted in debug builds.
    /// If you fail to adhere to these requirements in release builds, the lookup will be performed anyway and return `None`.
    pub fn find_namespace_entry(
        &self,
        namespace_entry_name: &str,
    ) -> Option<Result<ApiSetNamespaceEntry<'a>>> {
        debug_assert!(!namespace_entry_name.is_empty());
        debug_assert!(namespace_entry_name
            .chars()
            .all(|x| x.is_ascii_lowercase() || x.is_ascii_digit() || x == '-'));

        // "NTDLL first hashes the supposed name up to but not including the last hyphen"
        let (name_to_hash, _) = namespace_entry_name.rsplit_once('-')?;

        let hash_factor = self.header.hash_factor.get();
        let hash = name_to_hash.chars().fold(0u32, |acc, x| {
            acc.wrapping_mul(hash_factor).wrapping_add(x as u32)
        });

        let hash_entries = iter_try!(self.hash_entries());
        let mut namespace_entries = iter_try!(self.namespace_entries());

        // Perform binary search in the sorted array of hash entries.
        let mut left = 0i64;
        let mut right = hash_entries.len() as i64 - 1;

        while left <= right {
            let mid = (left + right) / 2;
            let hash_entry = hash_entries.clone().nth(mid as usize).unwrap();

            match hash_entry.hash().cmp(&hash) {
                Ordering::Equal => {
                    // This must be the entry we are looking for.
                    // Check the name to make absolutely sure.
                    let index = hash_entry.index();
                    let namespace_entry = namespace_entries.nth(index as usize)?;
                    let name = iter_try!(namespace_entry.name());

                    if name == namespace_entry_name {
                        return Some(Ok(namespace_entry));
                    } else {
                        return None;
                    }
                }
                Ordering::Less => left = mid + 1,
                Ordering::Greater => right = mid - 1,
            }
        }

        None
    }

    /// Returns an iterator over the [`ApiSetHashEntry`]s of this [`ApiSetMap`].
    ///
    /// You usually don't need to iterate through the hash entries manually.
    /// Use [`find_namespace_entry`](Self::find_namespace_entry) instead.
    ///
    /// [`ApiSetHashEntry`]: crate::hash_entry::ApiSetHashEntry
    /// [`ApiSetMap`]: crate::map::ApiSetMap
    pub fn hash_entries(&self) -> Result<ApiSetHashEntries<'a>> {
        let start = self.header.hash_entry_offset.get() as usize;
        let count = self.header.count.get() as usize;
        let end = start + mem::size_of::<ApiSetHashEntryHeader>() * count;
        let range = start..end;

        self.section_bytes
            .get(range.clone())
            .ok_or(NtApiSetError::HashEntriesOutOfBounds {
                range: start..end,
                actual: self.section_bytes.len(),
            })?;

        Ok(ApiSetHashEntries::new(self.section_bytes, range))
    }

    /// Returns an iterator over the [`ApiSetNamespaceEntry`] elements of this [`ApiSetMap`].
    ///
    /// Alternatively, you can lookup a specific namespace entry via the [`find_namespace_entry`](Self::find_namespace_entry) method.
    pub fn namespace_entries(&self) -> Result<ApiSetNamespaceEntries<'a>> {
        let start = self.header.namespace_entry_offset.get() as usize;
        let count = self.header.count.get() as usize;
        let end = start + mem::size_of::<ApiSetNamespaceEntryHeader>() * count;
        let range = start..end;

        self.section_bytes.get(range.clone()).ok_or(
            NtApiSetError::NamespaceEntriesOutOfBounds {
                range: start..end,
                actual: self.section_bytes.len(),
            },
        )?;

        Ok(ApiSetNamespaceEntries::new(self.section_bytes, range))
    }

    /// Creates an [`ApiSetMap`] from an API Set Map file opened via the `pelite` crate.
    ///
    /// If you already have the raw bytes of the `.apiset` section of that file, consider using [`try_from_apiset_section_bytes`](Self::try_from_apiset_section_bytes).
    #[cfg(feature = "pelite")]
    #[cfg_attr(docsrs, doc(cfg(feature = "pelite")))]
    pub fn try_from_pe64<T>(pe64: T) -> Result<Self>
    where
        T: pelite::pe64::Pe<'a>,
    {
        let apiset_section_header = pe64
            .section_headers()
            .by_name(".apiset")
            .ok_or(NtApiSetError::ApiSetSectionNotFound)?;
        let section_bytes = pe64
            .get_section_bytes(apiset_section_header)
            .map_err(|_| NtApiSetError::ApiSetSectionOutOfBounds)?;
        Self::try_from_apiset_section_bytes(section_bytes)
    }

    /// Creates an [`ApiSetMap`] from the raw bytes of the `.apiset` section of an API Set Map file.
    ///
    /// If you only have the DLL file and not the `.apiset` section bytes, consider using [`try_from_pe64`](Self::try_from_pe64).
    pub fn try_from_apiset_section_bytes(section_bytes: &'a [u8]) -> Result<Self> {
        let length = section_bytes.len();
        let (header, _) = LayoutVerified::<_, ApiSetMapHeader>::new_unaligned_from_prefix(
            section_bytes,
        )
        .ok_or(NtApiSetError::InvalidMapHeaderSize {
            expected: mem::size_of::<ApiSetMapHeader>(),
            actual: length,
        })?;

        // The internal structures are slightly different for older Windows versions.
        // See https://www.geoffchappell.com/studies/windows/win32/apisetschema/index.htm
        let version = header.version.get();
        if version != APISET_VERSION_WINDOWS_10 {
            return Err(NtApiSetError::UnsupportedVersion { version });
        }

        Ok(Self {
            section_bytes,
            header,
        })
    }
}
