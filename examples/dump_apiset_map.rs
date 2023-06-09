use std::fs;

use anyhow::{bail, Result};
use nt_apiset::ApiSetMap;
use pelite::pe64::PeFile;

fn main() -> Result<()> {
    let args = std::env::args().collect::<Vec<_>>();

    if args.len() != 2 {
        println!("Usage: dump_apiset_map <FILENAME>");
        println!("Example: dump_apiset_map C:\\Windows\\system32\\apisetschema.dll");
        bail!("Aborted");
    }

    let filename = &args[1];

    let dll = fs::read(filename)?;
    let pe_file = PeFile::from_bytes(&dll)?;
    let map = ApiSetMap::try_from_pe64(pe_file)?;

    for namespace_entry in map.namespace_entries()? {
        println!("● Namespace Entry: \"{}\"", namespace_entry.name()?);

        for value_entry in namespace_entry.value_entries()? {
            println!(
                "  ○ Value Entry: \"{}\" -> \"{}\"",
                value_entry.name()?,
                value_entry.value()?
            );
        }
    }

    Ok(())
}
