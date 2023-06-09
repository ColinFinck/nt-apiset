<img align="right" src="img/nt-apiset.svg">

# nt-apiset

[![crates.io](https://img.shields.io/crates/v/nt-apiset)](https://crates.io/crates/nt-apiset)
[![docs.rs](https://img.shields.io/docsrs/nt-apiset)](https://docs.rs/nt-apiset)
![license: MIT OR Apache-2.0](https://img.shields.io/crates/l/nt-apiset)

*by Colin Finck <<colin@reactos.org>>*

A parser for API Set Map files of Windows 10 and later.

API Sets are dependencies of PE executables whose names start with "api-" or "ext-", e.g. `api-ms-win-core-sysinfo-l1-1-0`.
They don't exist as real DLL files.
Instead, when that PE executable is loaded, an API Set Map file of the operating system is checked to figure out the real library file belonging to the dependency (in this case: `kernelbase.dll`).

The most prominent API Set Map file is `apisetschema.dll`.

## Examples
To get the real library file behind the aforementioned `api-ms-win-core-sysinfo-l1-1-0`, you can use this crate like:

```rust,no_run
let dll = std::fs::read("apisetschema.dll")?;
let pe_file = PeFile::from_bytes(&dll)?;
let map = ApiSetMap::try_from_pe64(pe_file)?;

let namespace_entry = map.find_namespace_entry("api-ms-win-core-sysinfo-l1-1-0")??;
let value_entry = namespace_entry.value_entries()?.next()?;

let name = namespace_entry.name()?;
let default_value = value_entry.value()?;
println!("{name} -> {default_value}");
```

## Further Resources
This parser is based on research by numerous people, who should be named here:

* https://www.geoffchappell.com/studies/windows/win32/apisetschema/index.htm  
  Original research by Geoff Chappell on the API Set Map file format, covering all versions down to Windows 7.

* https://lucasg.github.io/2017/10/15/Api-set-resolution/  
  All you need to know about how Windows 10 uses API Set Maps, distilled into a detailed blog post by Lucas Georges.

* https://apisets.info  
  Mark Jansen's online browser to get information about the `apisetschema.dll` files of various Windows versions.

## Acknowledgments
This crate is dedicated to the RE1 RRX train, which gave me much time (and often unplanned extra time) to work on it.
