//! Command-line interface for xmlschema-rs

#[cfg(feature = "cli")]
use clap::Parser;

#[cfg(feature = "cli")]
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Schema file path
    #[arg(short, long)]
    schema: String,

    /// XML document to validate
    #[arg(short, long)]
    document: Option<String>,

    /// Validate the document
    #[arg(short, long)]
    validate: bool,

    /// Convert to JSON
    #[arg(short, long)]
    json: bool,
}

#[cfg(feature = "cli")]
fn main() {
    let args = Args::parse();

    println!("xmlschema-rs v{}", xmlschema::VERSION);
    println!("Schema: {}", args.schema);

    if let Some(doc) = args.document {
        println!("Document: {}", doc);

        if args.validate {
            println!("Validation: TODO - Not yet implemented");
        }

        if args.json {
            println!("JSON conversion: TODO - Not yet implemented");
        }
    }

    println!("\nNote: Full implementation is in progress.");
    println!("See TODO.md for current status.");
}

#[cfg(not(feature = "cli"))]
fn main() {
    eprintln!("CLI feature not enabled. Rebuild with --features cli");
    std::process::exit(1);
}
