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
    about = "Extract recent transactions from a GnuCash file.",
    long_about = "Extract recent transactions from a GnuCash file.\n\n\
        Reads a gzip-compressed GnuCash XML file and outputs the most recent\n\
        transactions as JSON, sorted by date descending. Useful as categorization\n\
        precedent for an AI classifying new transactions."
)]
struct Cli {
    /// Path to the GnuCash file (.gnucash, gzip-compressed XML).
    #[arg(short, long, value_name = "FILE")]
    file: PathBuf,

    /// Output file path for the JSON transaction list.
    /// Defaults to 'transactions.json' in the same directory as the GnuCash file.
    #[arg(short, long, value_name = "FILE")]
    output: Option<PathBuf>,

    /// Number of most recent transactions to extract.
    #[arg(short, long, default_value = "100", value_name = "COUNT")]
    num: usize,
}

#[derive(Serialize)]
struct Split {
    account: String,
    amount: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    memo: String,
}

#[derive(Serialize)]
struct Transaction {
    date: String,
    description: String,
    splits: Vec<Split>,
}

struct RawAccount {
    name: String,
    parent: Option<String>,
}

struct RawSplit {
    account_guid: String,
    value_num: i64,
    value_den: i64,
    memo: String,
}

struct RawTransaction {
    date: String,
    description: String,
    splits: Vec<RawSplit>,
}

// ---------------------------------------------------------------------------
// XML helpers
// ---------------------------------------------------------------------------

fn local_name(full: &[u8]) -> String {
    let s = std::str::from_utf8(full).unwrap_or("");
    s.rsplit_once(':')
        .map(|(_, local)| local)
        .unwrap_or(s)
        .to_string()
}

fn full_name(bytes: &[u8]) -> String {
    std::str::from_utf8(bytes).unwrap_or("").to_string()
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
    // Skip "Root Account" prefix
    if parts.first().is_some_and(|p| p == "Root Account") {
        parts.remove(0);
    }
    parts.join(":")
}

fn fmt_dollars(num: i64, den: i64) -> String {
    if den == 0 {
        return "$0.00".to_string();
    }
    let amount = num as f64 / den as f64;
    if amount < 0.0 {
        format!("-${:.2}", -amount)
    } else {
        format!("${:.2}", amount)
    }
}

// ---------------------------------------------------------------------------
// GnuCash XML parsing
// ---------------------------------------------------------------------------

fn parse_gnucash(reader: impl Read) -> (HashMap<String, RawAccount>, Vec<RawTransaction>) {
    let mut xml_reader = Reader::from_reader(BufReader::new(reader));
    let mut buf = Vec::new();

    let mut accounts: HashMap<String, RawAccount> = HashMap::new();
    let mut transactions: Vec<RawTransaction> = Vec::new();

    // Account parsing state
    let mut in_account = false;
    let mut acct_name = String::new();
    let mut acct_guid = String::new();
    let mut acct_parent = String::new();

    // Transaction parsing state
    let mut in_transaction = false;
    let mut in_splits = false;
    let mut in_split = false;
    let mut in_date_posted = false;
    let mut txn_date = String::new();
    let mut txn_desc = String::new();
    let mut txn_splits: Vec<RawSplit> = Vec::new();
    let mut split_account = String::new();
    let mut split_value = String::new();
    let mut split_memo = String::new();

    // Track current element for text capture
    let mut current_tag = String::new();
    let mut tag_stack: Vec<String> = Vec::new();

    loop {
        match xml_reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name_bytes = e.name().as_ref().to_vec();
                let full = full_name(&name_bytes);
                let local = local_name(&name_bytes);
                tag_stack.push(full.clone());
                current_tag = local.clone();

                if full == "gnc:account" {
                    in_account = true;
                    acct_name.clear();
                    acct_guid.clear();
                    acct_parent.clear();
                } else if full == "gnc:transaction" {
                    in_transaction = true;
                    txn_date.clear();
                    txn_desc.clear();
                    txn_splits.clear();
                } else if in_transaction && full == "trn:date-posted" {
                    in_date_posted = true;
                } else if in_transaction && full == "trn:splits" {
                    in_splits = true;
                } else if in_splits && full == "trn:split" {
                    in_split = true;
                    split_account.clear();
                    split_value.clear();
                    split_memo.clear();
                }
            }
            Ok(Event::Text(ref e)) => {
                let text = e.unescape().unwrap_or_default().to_string();

                if in_account {
                    match current_tag.as_str() {
                        "name" => acct_name = text,
                        "id" if acct_guid.is_empty() => acct_guid = text,
                        "parent" => acct_parent = text,
                        _ => {}
                    }
                } else if in_split {
                    match current_tag.as_str() {
                        "account" => split_account = text,
                        "value" => split_value = text,
                        "memo" => split_memo = text,
                        _ => {}
                    }
                } else if in_transaction {
                    if in_date_posted && current_tag == "date" {
                        // Extract just the date portion (YYYY-MM-DD)
                        txn_date = text.trim().chars().take(10).collect();
                    } else if current_tag == "description" {
                        txn_desc = text;
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                let name_bytes = e.name().as_ref().to_vec();
                let full = full_name(&name_bytes);

                if full == "gnc:account" && in_account {
                    if !acct_guid.is_empty() {
                        accounts.insert(
                            acct_guid.clone(),
                            RawAccount {
                                name: acct_name.clone(),
                                parent: if acct_parent.is_empty() {
                                    None
                                } else {
                                    Some(acct_parent.clone())
                                },
                            },
                        );
                    }
                    in_account = false;
                } else if full == "trn:split" && in_split {
                    let (num, den) = parse_fraction(&split_value);
                    txn_splits.push(RawSplit {
                        account_guid: split_account.clone(),
                        value_num: num,
                        value_den: den,
                        memo: split_memo.clone(),
                    });
                    in_split = false;
                } else if full == "trn:splits" {
                    in_splits = false;
                } else if full == "trn:date-posted" {
                    in_date_posted = false;
                } else if full == "gnc:transaction" && in_transaction {
                    transactions.push(RawTransaction {
                        date: txn_date.clone(),
                        description: txn_desc.clone(),
                        splits: std::mem::take(&mut txn_splits),
                    });
                    in_transaction = false;
                }

                tag_stack.pop();
                current_tag = tag_stack
                    .last()
                    .map(|s| local_name(s.as_bytes()))
                    .unwrap_or_default();
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

    (accounts, transactions)
}

fn parse_fraction(s: &str) -> (i64, i64) {
    if let Some((num_s, den_s)) = s.split_once('/') {
        let num: i64 = num_s.parse().unwrap_or(0);
        let den: i64 = den_s.parse().unwrap_or(1);
        (num, den)
    } else {
        let num: i64 = s.parse().unwrap_or(0);
        (num, 1)
    }
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
            .join("transactions.json")
    });

    let file = File::open(&cli.file).unwrap_or_else(|e| {
        eprintln!("error: Cannot open file: {e}");
        process::exit(2);
    });
    let decoder = GzDecoder::new(BufReader::new(file));
    let (accounts, mut transactions) = parse_gnucash(decoder);

    // Sort by date descending, take the most recent N
    transactions.sort_by(|a, b| b.date.cmp(&a.date));
    transactions.truncate(cli.num);

    // Convert to output format
    let output: Vec<Transaction> = transactions
        .iter()
        .map(|txn| {
            let mut splits: Vec<Split> = txn
                .splits
                .iter()
                .map(|sp| {
                    let path = build_path(&sp.account_guid, &accounts);
                    Split {
                        account: path,
                        amount: fmt_dollars(sp.value_num, sp.value_den),
                        memo: sp.memo.clone(),
                    }
                })
                .collect();
            // Sort splits: positive amounts first (expenses), then negative (payment)
            splits.sort_by(|a, b| {
                let a_neg = a.amount.starts_with('-');
                let b_neg = b.amount.starts_with('-');
                a_neg.cmp(&b_neg)
            });
            Transaction {
                date: txn.date.clone(),
                description: txn.description.clone(),
                splits,
            }
        })
        .collect();

    let json = serde_json::to_string_pretty(&output).expect("Failed to serialize");
    std::fs::write(&output_path, json + "\n").expect("Failed to write output file");

    println!(
        "Extracted {} transactions from {}",
        output.len(),
        cli.file.display()
    );
    if let (Some(first), Some(last)) = (output.last(), output.first()) {
        println!("  Date range: {} to {}", first.date, last.date);
    }
    println!("Output: {}", output_path.display());
}
