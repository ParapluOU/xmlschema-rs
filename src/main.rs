//! Command-line interface for xmlschema-rs

#[cfg(feature = "cli")]
use clap::{Parser, Subcommand};

#[cfg(feature = "cli")]
use std::fs;
#[cfg(feature = "cli")]
use std::path::PathBuf;

#[cfg(feature = "cli")]
use xmlschema::converters::{create_converter, ConverterType, ElementData};
#[cfg(feature = "cli")]
use xmlschema::documents::{Document, Element};
#[cfg(feature = "cli")]
use xmlschema::validators::XsdSchema;

#[cfg(feature = "cli")]
#[derive(Parser, Debug)]
#[command(name = "xmlschema")]
#[command(author, version, about = "XML Schema validation and conversion tool", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[cfg(feature = "cli")]
#[derive(Subcommand, Debug)]
enum Commands {
    /// Inspect an XSD schema and display its structure
    Inspect {
        /// Path to the XSD schema file
        #[arg(value_name = "SCHEMA")]
        schema: PathBuf,

        /// Show detailed information about a specific element
        #[arg(short, long)]
        element: Option<String>,

        /// Show detailed information about a specific type
        #[arg(short = 't', long)]
        type_name: Option<String>,

        /// Show all elements
        #[arg(long)]
        elements: bool,

        /// Show all types
        #[arg(long)]
        types: bool,

        /// Show all attributes
        #[arg(long)]
        attributes: bool,

        /// Show all groups
        #[arg(long)]
        groups: bool,

        /// Output as JSON
        #[arg(short, long)]
        json: bool,
    },

    /// Convert an XML document to JSON
    #[command(name = "xml2json")]
    XmlToJson {
        /// Path to the XML file to convert
        #[arg(value_name = "FILE")]
        file: PathBuf,

        /// Conversion format: default, parker, badgerfish, unordered
        #[arg(short, long, default_value = "default")]
        format: String,

        /// Pretty print the output
        #[arg(short, long)]
        pretty: bool,

        /// Output file (defaults to stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Validate an XML document against an XSD schema
    Validate {
        /// Path to the XSD schema file
        #[arg(short, long, value_name = "SCHEMA")]
        schema: PathBuf,

        /// Path to the XML file to validate
        #[arg(value_name = "FILE")]
        file: PathBuf,

        /// Validation mode: strict or lax
        #[arg(short, long, default_value = "strict")]
        mode: String,
    },
}

#[cfg(feature = "cli")]
fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Inspect {
            schema,
            element,
            type_name,
            elements,
            types,
            attributes,
            groups,
            json,
        } => cmd_inspect(schema, element, type_name, elements, types, attributes, groups, json),
        Commands::XmlToJson {
            file,
            format,
            pretty,
            output,
        } => cmd_xml2json(file, format, pretty, output),
        Commands::Validate { schema, file, mode } => cmd_validate(schema, file, mode),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

#[cfg(feature = "cli")]
fn cmd_inspect(
    schema_path: PathBuf,
    element: Option<String>,
    type_name: Option<String>,
    show_elements: bool,
    show_types: bool,
    show_attributes: bool,
    show_groups: bool,
    json_output: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let schema = XsdSchema::from_file(&schema_path)?;

    // If specific element or type requested
    if let Some(elem_name) = element {
        print_element_details(&schema, &elem_name, json_output)?;
        return Ok(());
    }

    if let Some(type_name) = type_name {
        print_type_details(&schema, &type_name, json_output)?;
        return Ok(());
    }

    // If no specific flags, show summary
    let show_all = !show_elements && !show_types && !show_attributes && !show_groups;

    if json_output {
        print_schema_json(&schema, show_all || show_elements, show_all || show_types)?;
    } else {
        print_schema_summary(&schema);

        if show_all || show_elements {
            println!("\n=== Global Elements ===");
            for (qname, elem) in schema.elements() {
                let type_str = elem.type_name.as_ref().map(|t| t.to_string()).unwrap_or_else(|| "anonymous".to_string());
                println!("  {} : {}", qname.to_string(), type_str);
            }
        }

        if show_all || show_types {
            println!("\n=== Global Types ===");
            for (qname, global_type) in schema.types() {
                let type_kind = match global_type {
                    xmlschema::validators::GlobalType::Simple(_) => "simple",
                    xmlschema::validators::GlobalType::Complex(_) => "complex",
                };
                println!("  {} ({})", qname.to_string(), type_kind);
            }
        }

        if show_attributes {
            println!("\n=== Global Attributes ===");
            for (qname, _attr) in schema.attributes() {
                println!("  {}", qname.to_string());
            }
        }

        if show_groups {
            println!("\n=== Model Groups ===");
            for (qname, _group) in schema.groups() {
                println!("  {}", qname.to_string());
            }

            println!("\n=== Attribute Groups ===");
            for (qname, _group) in schema.attribute_groups() {
                println!("  {}", qname.to_string());
            }
        }
    }

    Ok(())
}

#[cfg(feature = "cli")]
fn print_schema_summary(schema: &XsdSchema) {
    println!("xmlschema-rs v{}", xmlschema::VERSION);
    println!();
    println!("Schema Information:");
    println!("  Version: XSD {}", schema.xsd_version());
    if let Some(ns) = &schema.target_namespace {
        println!("  Target Namespace: {}", ns);
    } else {
        println!("  Target Namespace: (none)");
    }
    println!("  Element Form Default: {}", schema.element_form_default);
    println!("  Attribute Form Default: {}", schema.attribute_form_default);
    println!();
    println!("Statistics:");
    println!("  Global Elements: {}", schema.element_count());
    println!("  Global Types: {}", schema.type_count());
    println!("  Global Attributes: {}", schema.attributes().count());
    println!("  Model Groups: {}", schema.groups().count());
    println!("  Attribute Groups: {}", schema.attribute_groups().count());
}

#[cfg(feature = "cli")]
fn print_schema_json(schema: &XsdSchema, include_elements: bool, include_types: bool) -> Result<(), Box<dyn std::error::Error>> {
    use serde_json::{json, Map, Value};

    let mut output = Map::new();

    output.insert("version".to_string(), json!(schema.xsd_version()));
    output.insert("targetNamespace".to_string(), json!(schema.target_namespace));
    output.insert("elementFormDefault".to_string(), json!(schema.element_form_default.to_string()));
    output.insert("attributeFormDefault".to_string(), json!(schema.attribute_form_default.to_string()));

    let mut stats = Map::new();
    stats.insert("globalElements".to_string(), json!(schema.element_count()));
    stats.insert("globalTypes".to_string(), json!(schema.type_count()));
    stats.insert("globalAttributes".to_string(), json!(schema.attributes().count()));
    stats.insert("modelGroups".to_string(), json!(schema.groups().count()));
    stats.insert("attributeGroups".to_string(), json!(schema.attribute_groups().count()));
    output.insert("statistics".to_string(), Value::Object(stats));

    if include_elements {
        let elements: Vec<Value> = schema.elements()
            .map(|(qname, elem)| {
                json!({
                    "name": qname.to_string(),
                    "type": elem.type_name.as_ref().map(|t| t.to_string()),
                    "nillable": elem.nillable,
                })
            })
            .collect();
        output.insert("elements".to_string(), Value::Array(elements));
    }

    if include_types {
        let types: Vec<Value> = schema.types()
            .map(|(qname, global_type)| {
                let kind = match global_type {
                    xmlschema::validators::GlobalType::Simple(_) => "simple",
                    xmlschema::validators::GlobalType::Complex(_) => "complex",
                };
                json!({
                    "name": qname.to_string(),
                    "kind": kind,
                })
            })
            .collect();
        output.insert("types".to_string(), Value::Array(types));
    }

    let json_str = serde_json::to_string_pretty(&Value::Object(output))?;
    println!("{}", json_str);
    Ok(())
}

#[cfg(feature = "cli")]
fn print_element_details(schema: &XsdSchema, name: &str, json_output: bool) -> Result<(), Box<dyn std::error::Error>> {
    // Try to find the element
    let found = schema.elements()
        .find(|(qname, _)| qname.local_name == name || qname.to_string().contains(name));

    if let Some((qname, elem)) = found {
        if json_output {
            let json = serde_json::json!({
                "name": qname.to_string(),
                "localName": qname.local_name.clone(),
                "namespace": qname.namespace.clone(),
                "type": elem.type_name.as_ref().map(|t| t.to_string()),
                "nillable": elem.nillable,
                "abstract": elem.abstract_element,
            });
            println!("{}", serde_json::to_string_pretty(&json)?);
        } else {
            println!("Element: {}", qname.to_string());
            println!("  Local Name: {}", qname.local_name);
            if let Some(ns) = &qname.namespace {
                println!("  Namespace: {}", ns);
            }
            if let Some(type_name) = &elem.type_name {
                println!("  Type: {}", type_name.to_string());
            }
            println!("  Nillable: {}", elem.nillable);
            println!("  Abstract: {}", elem.abstract_element);
        }
    } else {
        return Err(format!("Element '{}' not found in schema", name).into());
    }

    Ok(())
}

#[cfg(feature = "cli")]
fn print_type_details(schema: &XsdSchema, name: &str, json_output: bool) -> Result<(), Box<dyn std::error::Error>> {
    let found = schema.types()
        .find(|(qname, _)| qname.local_name == name || qname.to_string().contains(name));

    if let Some((qname, global_type)) = found {
        let kind = match global_type {
            xmlschema::validators::GlobalType::Simple(_) => "simple",
            xmlschema::validators::GlobalType::Complex(_) => "complex",
        };

        if json_output {
            let json = serde_json::json!({
                "name": qname.to_string(),
                "localName": qname.local_name.clone(),
                "namespace": qname.namespace.clone(),
                "kind": kind,
            });
            println!("{}", serde_json::to_string_pretty(&json)?);
        } else {
            println!("Type: {}", qname.to_string());
            println!("  Local Name: {}", qname.local_name);
            if let Some(ns) = &qname.namespace {
                println!("  Namespace: {}", ns);
            }
            println!("  Kind: {}", kind);
        }
    } else {
        return Err(format!("Type '{}' not found in schema", name).into());
    }

    Ok(())
}

#[cfg(feature = "cli")]
fn cmd_xml2json(
    file: PathBuf,
    format: String,
    pretty: bool,
    output: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Read the XML file
    let xml_content = fs::read_to_string(&file)?;

    // Parse the XML document
    let doc = Document::from_string(&xml_content)?;

    // Get the root element
    let root = doc.root.as_ref()
        .ok_or("XML document has no root element")?;

    // Select converter based on format
    let conv_type = match format.to_lowercase().as_str() {
        "default" => ConverterType::Default,
        "parker" => ConverterType::Parker,
        "badgerfish" => ConverterType::BadgerFish,
        "unordered" => ConverterType::Unordered,
        _ => return Err(format!("Unknown format: {}. Use: default, parker, badgerfish, unordered", format).into()),
    };

    let converter = create_converter(conv_type);

    // Convert Element to ElementData
    let element_data = element_to_element_data(root);

    // Convert to JSON
    let json_value = converter.decode(&element_data, 0);

    // Wrap in root element if needed (preserve root element name)
    let output_json = serde_json::json!({
        root.local_name(): json_value
    });

    // Format output
    let json_str = if pretty {
        serde_json::to_string_pretty(&output_json)?
    } else {
        serde_json::to_string(&output_json)?
    };

    // Write output
    if let Some(output_path) = output {
        fs::write(output_path, &json_str)?;
    } else {
        println!("{}", json_str);
    }

    Ok(())
}

#[cfg(feature = "cli")]
fn element_to_element_data(elem: &Element) -> ElementData {
    let mut data = ElementData::new(elem.local_name());

    // Add text content
    if let Some(text) = &elem.text {
        data = data.with_text(text.clone());
    }

    // Add attributes
    for (qname, value) in &elem.attributes {
        data = data.with_attribute(qname.local_name.clone(), value.clone());
    }

    // Add xmlns declarations
    for (prefix, uri) in elem.namespaces.iter() {
        data = data.with_xmlns(prefix.clone(), uri.clone());
    }

    // Add child elements recursively
    for child in &elem.children {
        let child_data = element_to_element_data(child);
        let child_json = create_converter(ConverterType::Default).decode(&child_data, 1);
        data = data.with_child(child.local_name(), child_json);
    }

    data
}

#[cfg(feature = "cli")]
fn cmd_validate(
    schema_path: PathBuf,
    file: PathBuf,
    mode: String,
) -> Result<(), Box<dyn std::error::Error>> {
    use xmlschema::validators::ValidationMode;

    // Load the schema
    let schema = XsdSchema::from_file(&schema_path)?;

    // Read the XML file
    let xml_content = fs::read_to_string(&file)?;

    // Parse the XML document
    let doc = Document::from_string(&xml_content)?;

    // Determine validation mode
    let validation_mode = match mode.to_lowercase().as_str() {
        "strict" => ValidationMode::Strict,
        "lax" => ValidationMode::Lax,
        _ => return Err(format!("Unknown validation mode: {}. Use: strict, lax", mode).into()),
    };

    // Validate
    let result = schema.validate_with_mode(&doc, validation_mode);

    if result.valid {
        println!("✓ Document is valid");
        Ok(())
    } else {
        println!("✗ Document is invalid");
        println!();
        println!("Errors:");
        for error in &result.errors {
            println!("  - {}", error);
        }
        std::process::exit(1);
    }
}

#[cfg(not(feature = "cli"))]
fn main() {
    eprintln!("CLI feature not enabled. Rebuild with --features cli");
    std::process::exit(1);
}
