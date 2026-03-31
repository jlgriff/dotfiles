use clap::Parser;
use flate2::read::GzDecoder;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use serde::Serialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
#[command(
    about = "Extract account paths from a GnuCash file.",
    long_about = "Extract account paths from a GnuCash file.\n\n\
        Reads a gzip-compressed GnuCash XML file, builds the account hierarchy,\n\
        and outputs all account paths grouped by type (Assets, Expenses, Income,\n\
        Liabilities, Equity) as JSON.",
    after_help = "\
The GnuCash file must be in the default gzip-compressed XML format.\n\
Account paths use colon-separated notation (e.g. Expenses:Food:Groceries)."
)]
struct Cli {
    /// Path to the GnuCash file (.gnucash, gzip-compressed XML).
    #[arg(short, long, value_name = "FILE")]
    file: PathBuf,

    /// Output file path for the JSON account list.
    /// Defaults to 'accounts.json' in the same directory as the GnuCash file.
    #[arg(short, long, value_name = "FILE")]
    output: Option<PathBuf>,

    /// Filter to a specific top-level category (e.g. "Expenses", "Assets").
    /// Can be specified multiple times. If omitted, all categories are included.
    #[arg(short, long, value_name = "CATEGORY")]
    category: Vec<String>,
}

#[derive(Serialize)]
struct AccountOutput {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    assets: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    expenses: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    income: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    liabilities: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    equity: Vec<String>,
}

struct RawAccount {
    name: String,
    parent: Option<String>,
}

// ---------------------------------------------------------------------------
// GnuCash XML parsing
// ---------------------------------------------------------------------------

fn parse_accounts(reader: impl Read) -> HashMap<String, RawAccount> {
    let mut xml_reader = Reader::from_reader(BufReader::new(reader));
    let mut buf = Vec::new();
    let mut accounts: HashMap<String, RawAccount> = HashMap::new();

    // State for parsing within a <gnc:account> element
    let mut in_account = false;
    let mut current_tag = String::new();
    let mut name = String::new();
    let mut guid = String::new();
    let mut parent = String::new();

    loop {
        match xml_reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let name_bytes = e.name().as_ref().to_vec();
                let local = local_name(&name_bytes);
                if local == "account"
                    && std::str::from_utf8(&name_bytes)
                        .unwrap_or("")
                        .starts_with("gnc:")
                {
                    in_account = true;
                    name.clear();
                    guid.clear();
                    parent.clear();
                } else if in_account {
                    current_tag = local.to_string();
                }
            }
            Ok(Event::Text(ref e)) => {
                if in_account {
                    let text = e.unescape().unwrap_or_default().to_string();
                    match current_tag.as_str() {
                        "name" => name = text,
                        "id" => {
                            if guid.is_empty() {
                                guid = text;
                            }
                        }
                        "parent" => parent = text,
                        _ => {}
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                let name_bytes = e.name().as_ref().to_vec();
                let local = local_name(&name_bytes);
                if local == "account" && in_account {
                    if !guid.is_empty() && name != "Root Account" {
                        accounts.insert(
                            guid.clone(),
                            RawAccount {
                                name: name.clone(),
                                parent: if parent.is_empty() {
                                    None
                                } else {
                                    Some(parent.clone())
                                },
                            },
                        );
                    }
                    in_account = false;
                }
                current_tag.clear();
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                eprintln!("XML parse error: {e}");
                break;
            }
            _ => {}
        }
        buf.clear();
    }

    accounts
}

fn local_name(full: &[u8]) -> &str {
    let s = std::str::from_utf8(full).unwrap_or("");
    s.rsplit_once(':').map(|(_, local)| local).unwrap_or(s)
}

fn build_path(guid: &str, accounts: &HashMap<String, RawAccount>) -> String {
    let mut parts = Vec::new();
    let mut current = Some(guid.to_string());
    while let Some(id) = current {
        if let Some(acct) = accounts.get(&id) {
            parts.push(acct.name.clone());
            current = acct.parent.clone();
        } else {
            break;
        }
    }
    parts.reverse();
    parts.join(":")
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    let cli = Cli::parse();

    if !cli.file.exists() {
        eprintln!("error: File not found: {}", cli.file.display());
        process::exit(2);
    }

    let output_path = cli.output.unwrap_or_else(|| {
        cli.file
            .parent()
            .unwrap_or(&cli.file)
            .join("accounts.json")
    });

    // Read and decompress
    let file = File::open(&cli.file).unwrap_or_else(|e| {
        eprintln!("error: Cannot open file: {e}");
        process::exit(2);
    });
    let decoder = GzDecoder::new(BufReader::new(file));

    // Parse accounts
    let accounts = parse_accounts(decoder);

    // Build full paths and group by top-level category
    let mut categorized: HashMap<String, Vec<String>> = HashMap::new();
    for guid in accounts.keys() {
        let path = build_path(guid, &accounts);
        if path.is_empty() {
            continue;
        }
        let top = path.split(':').next().unwrap_or("").to_string();
        categorized.entry(top).or_default().push(path);
    }

    // Sort each category
    for paths in categorized.values_mut() {
        paths.sort();
    }

    // Filter by category if specified
    let include_all = cli.category.is_empty();
    let filter: Vec<String> = cli.category.iter().map(|s| s.to_lowercase()).collect();

    let mut get = |key: &str| -> Vec<String> {
        if include_all || filter.iter().any(|f| f == &key.to_lowercase()) {
            categorized.remove(key).unwrap_or_default()
        } else {
            Vec::new()
        }
    };

    let output = AccountOutput {
        assets: get("Assets"),
        expenses: get("Expenses"),
        income: get("Income"),
        liabilities: get("Liabilities"),
        equity: get("Equity"),
    };

    let json = serde_json::to_string_pretty(&output).expect("Failed to serialize");
    std::fs::write(&output_path, json + "\n").expect("Failed to write output file");

    // Print summary
    let total = output.assets.len()
        + output.expenses.len()
        + output.income.len()
        + output.liabilities.len()
        + output.equity.len();

    println!("Extracted {total} accounts from {}", cli.file.display());
    if !output.assets.is_empty() {
        println!("  Assets:      {}", output.assets.len());
    }
    if !output.expenses.is_empty() {
        println!("  Expenses:    {}", output.expenses.len());
    }
    if !output.income.is_empty() {
        println!("  Income:      {}", output.income.len());
    }
    if !output.liabilities.is_empty() {
        println!("  Liabilities: {}", output.liabilities.len());
    }
    if !output.equity.is_empty() {
        println!("  Equity:      {}", output.equity.len());
    }
    println!("Output: {}", output_path.display());
}
