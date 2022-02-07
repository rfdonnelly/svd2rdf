use svd2rdf::Rdf;

use clap::Parser;
use svd_parser;

#[derive(Parser)]
#[clap(about, version, author)]
struct Args {
    /// The SVD file to convert.
    file: String
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let svd = std::fs::read_to_string(&args.file)?;
    let device = svd_parser::parse(&svd)?;

    let rdf = Rdf::from(device);
    println!("{}", serde_json::to_string(&rdf)?);

    Ok(())
}
