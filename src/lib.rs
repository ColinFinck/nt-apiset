// Copyright 2023 Colin Finck <colin@reactos.org>
// SPDX-License-Identifier: MIT OR Apache-2.0
//
//! A parser for API Set Map files of Windows 10 and later.
//!
//! API Sets are dependencies of PE executables whose names start with "api-" or "ext-", e.g. `api-ms-win-core-sysinfo-l1-1-0`.
//! They don't exist as real DLL files.
//! Instead, when that PE executable is loaded, an API Set Map file of the operating system is checked to figure out the real library
//! file belonging to the dependency (in this case: `kernelbase.dll`).
//!
//! The most prominent API Set Map file is `apisetschema.dll`.
//!
//! # Examples
//!
//! To get the real library file behind the aforementioned `api-ms-win-core-sysinfo-l1-1-0`, you can use this crate like:
//!
//! ```no_run
//! # use nt_apiset::ApiSetMap;
//! # use pelite::pe64::PeFile;
//! let dll = std::fs::read("apisetschema.dll").unwrap();
//! let pe_file = PeFile::from_bytes(&dll).unwrap();
//! let map = ApiSetMap::try_from_pe64(pe_file).unwrap();
//!
//! let namespace_entry = map
//!     .find_namespace_entry("api-ms-win-core-sysinfo-l1-1-0")
//!     .unwrap()
//!     .unwrap();
//! let value_entry = namespace_entry.value_entries().unwrap().next().unwrap();
//!
//! let name = namespace_entry.name().unwrap();
//! let default_value = value_entry.value().unwrap();
//! println!("{name} -> {default_value}");
//! ```

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

#[macro_use]
mod helpers;

mod error;
mod hash_entry;
mod map;
mod namespace_entry;
mod value_entry;

pub use error::*;
pub use hash_entry::*;
pub use map::*;
pub use namespace_entry::*;
pub use value_entry::*;
