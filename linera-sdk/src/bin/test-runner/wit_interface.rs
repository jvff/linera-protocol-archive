use anyhow::Result;
use wasmtime::Linker;

pub fn configure_linker(linker: &mut Linker<()>) -> Result<()> {
    linker.allow_shadowing(true);

    Ok(())
}
