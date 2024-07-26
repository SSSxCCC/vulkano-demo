use spirv_builder::{MetadataPrintout, SpirvBuilder};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    SpirvBuilder::new("minimal-shader", "spirv-unknown-vulkan1.2")
        .print_metadata(MetadataPrintout::Full)
        .build()?;

    Ok(())
}
