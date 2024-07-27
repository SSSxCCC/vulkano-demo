use spirv_builder::{SpirvBuilder, SpirvMetadata};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    SpirvBuilder::new("minimal-shader", "spirv-unknown-vulkan1.2")
        .spirv_metadata(SpirvMetadata::NameVariables)
        .build()?;

    Ok(())
}
