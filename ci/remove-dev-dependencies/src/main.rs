use std::{error::Error, fs};

fn main() -> Result<(), Box<dyn Error>> {
    let content = fs::read_to_string("Cargo.toml")?;
    let mut doc: toml_edit::Document = content.parse()?;
    doc.as_table_mut().remove("dev-dependencies");
    fs::write("Cargo.toml", doc.to_string())?;
    Ok(())
}
