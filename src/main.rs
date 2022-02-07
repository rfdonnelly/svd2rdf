use svd_parser;

use svd2rdf::Rdf;

use std::fs::File;
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let svd = &mut String::new();
    File::open("tests/input/example.svd")?.read_to_string(svd)?;
    let device = svd_parser::parse(svd)?;

    let rdf = Rdf::from(device);
    println!("{}", serde_json::to_string(&rdf)?);

    Ok(())
}
