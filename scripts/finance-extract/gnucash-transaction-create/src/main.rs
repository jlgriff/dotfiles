use clap::{Parser, ValueEnum};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process;

const SUPPORTED_SCHEMA_VERSION: u32 = 1;

#[derive(Parser, Debug)]
#[command(
    about = "Validate transaction JSON and independently create Markdown and GnuCash QIF.",
    long_about = "Validate a neutral JSON transaction file, then independently render both a\n\
        Markdown review file and a multi-account, multi-split QIF for GnuCash import.\n\
        Neither generated file is parsed to produce the other.",
    after_help = "\
Canonical amount values in the JSON must be strings with a period decimal separator,
no currency symbol, and no digit grouping (for example, \"-1234.50\"). Each
transaction must mark exactly one split with \"source\": true.

Optional \"source_description\" and \"review_notes\" values appear only in the
Markdown review file.

For GnuCash, use File > Import > Import QIF, choose MM/DD/YYYY if prompted,
then review every account/category mapping and possible duplicate before importing."
)]
struct Cli {
    /// Canonical JSON transaction file.
    #[arg(short, long, value_name = "FILE")]
    input: PathBuf,

    /// Markdown output. Defaults to '<input-stem>.md'.
    #[arg(short = 'm', long, value_name = "FILE")]
    markdown_output: Option<PathBuf>,

    /// GnuCash QIF output. Defaults to '<input-stem>_gnucash.qif'.
    #[arg(short = 'q', long, value_name = "FILE")]
    qif_output: Option<PathBuf>,

    /// Currency symbol shown only in Markdown. May be empty.
    #[arg(long, default_value = "$", value_name = "SYMBOL")]
    currency_symbol: String,

    /// Currency-symbol position in Markdown.
    #[arg(long, value_enum, default_value = "prefix")]
    currency_symbol_position: SymbolPosition,

    /// Decimal separator shown in Markdown.
    #[arg(long, default_value = ".", value_name = "CHARACTER")]
    markdown_decimal_separator: char,

    /// Amount precision in both generated files.
    #[arg(
        long,
        default_value_t = 2,
        value_parser = clap::value_parser!(u32).range(0..=9)
    )]
    decimal_places: u32,

    /// Fail unless the JSON contains this many transactions.
    #[arg(long, value_name = "COUNT")]
    expected_transactions: Option<usize>,

    /// Replace existing Markdown and QIF outputs.
    #[arg(long)]
    force: bool,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum SymbolPosition {
    Prefix,
    Suffix,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct InputFile {
    version: u32,
    #[serde(default)]
    source_description: Option<String>,
    #[serde(default)]
    review_notes: Vec<String>,
    transactions: Vec<InputTransaction>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct InputTransaction {
    date: String,
    description: String,
    splits: Vec<InputSplit>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct InputSplit {
    #[serde(default)]
    memo: String,
    account: String,
    amount: String,
    #[serde(default)]
    source: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Transaction {
    date: String,
    description: String,
    splits: Vec<Split>,
    source_index: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Split {
    memo: String,
    account: String,
    amount: i128,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Document {
    source_description: Option<String>,
    review_notes: Vec<String>,
    transactions: Vec<Transaction>,
}

#[derive(Clone, Debug)]
struct RenderConfig {
    currency_symbol: String,
    currency_symbol_position: SymbolPosition,
    markdown_decimal_separator: char,
    decimal_places: u32,
}

fn main() {
    if let Err(error) = run(Cli::parse()) {
        eprintln!("error: {error}");
        process::exit(2);
    }
}

fn run(cli: Cli) -> Result<(), String> {
    validate_cli(&cli)?;
    if !cli.input.is_file() {
        return Err(format!("input file not found: {}", cli.input.display()));
    }

    let raw = fs::read_to_string(&cli.input)
        .map_err(|error| format!("cannot read {}: {error}", cli.input.display()))?;
    let input: InputFile = serde_json::from_str(&raw)
        .map_err(|error| format!("invalid JSON in {}: {error}", cli.input.display()))?;
    let document = validate_input(input, cli.decimal_places)?;

    if let Some(expected) = cli.expected_transactions
        && document.transactions.len() != expected
    {
        return Err(format!(
            "expected {expected} transactions, found {}",
            document.transactions.len()
        ));
    }

    let config = RenderConfig {
        currency_symbol: cli.currency_symbol,
        currency_symbol_position: cli.currency_symbol_position,
        markdown_decimal_separator: cli.markdown_decimal_separator,
        decimal_places: cli.decimal_places,
    };
    let markdown = render_markdown(&document, &config);
    let qif = render_qif(&document.transactions, &config)?;

    let markdown_output = cli
        .markdown_output
        .unwrap_or_else(|| default_markdown_path(&cli.input));
    let qif_output = cli
        .qif_output
        .unwrap_or_else(|| default_qif_path(&cli.input));
    validate_output_paths(&cli.input, &markdown_output, &qif_output)?;
    write_output_pair(
        &markdown_output,
        markdown.as_bytes(),
        &qif_output,
        qif.as_bytes(),
        cli.force,
    )?;

    let split_count: usize = document.transactions.iter().map(|tx| tx.splits.len()).sum();
    println!(
        "Wrote {} transactions ({split_count} splits) to:",
        document.transactions.len()
    );
    println!("  Markdown: {}", markdown_output.display());
    println!("  GnuCash QIF: {}", qif_output.display());
    println!("Source split totals:");
    for (account, total) in source_totals(&document.transactions)? {
        println!(
            "  {account}: {}",
            format_number(total, '.', config.decimal_places)
        );
    }
    Ok(())
}

fn validate_cli(cli: &Cli) -> Result<(), String> {
    validate_numeric_separator("Markdown decimal separator", cli.markdown_decimal_separator)?;
    if cli.currency_symbol.contains(['\n', '\r']) {
        return Err("currency symbol cannot contain a line break".to_string());
    }
    Ok(())
}

fn validate_numeric_separator(name: &str, separator: char) -> Result<(), String> {
    if separator.is_ascii_digit() || matches!(separator, '+' | '-' | '(' | ')' | '"' | '\n' | '\r')
    {
        return Err(format!("{name} is not a valid numeric separator"));
    }
    Ok(())
}

fn validate_input(input: InputFile, decimal_places: u32) -> Result<Document, String> {
    if input.version != SUPPORTED_SCHEMA_VERSION {
        return Err(format!(
            "unsupported schema version {}; expected {}",
            input.version, SUPPORTED_SCHEMA_VERSION
        ));
    }
    if input.transactions.is_empty() {
        return Err("input contains no transactions".to_string());
    }

    if let Some(description) = &input.source_description {
        validate_document_text(description, "source description")?;
    }
    for (index, note) in input.review_notes.iter().enumerate() {
        validate_document_text(note, &format!("review note {}", index + 1))?;
    }

    let transactions = input
        .transactions
        .into_iter()
        .enumerate()
        .map(|(index, transaction)| validate_transaction(transaction, index + 1, decimal_places))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Document {
        source_description: input.source_description,
        review_notes: input.review_notes,
        transactions,
    })
}

fn validate_document_text(value: &str, field: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{field} is empty"));
    }
    if value.contains(['\n', '\r']) {
        return Err(format!("{field} cannot contain a line break"));
    }
    Ok(())
}

fn validate_transaction(
    transaction: InputTransaction,
    number: usize,
    decimal_places: u32,
) -> Result<Transaction, String> {
    validate_text(&transaction.date, "date", number)?;
    validate_text(&transaction.description, "description", number)?;
    if transaction.splits.len() < 2 {
        return Err(format!(
            "transaction {number}: at least two splits are required"
        ));
    }

    let source_indexes: Vec<usize> = transaction
        .splits
        .iter()
        .enumerate()
        .filter_map(|(index, split)| split.source.then_some(index))
        .collect();
    let source_index = match source_indexes.as_slice() {
        [index] => *index,
        [] => {
            return Err(format!(
                "transaction {number}: exactly one split must have source=true; found none"
            ));
        }
        _ => {
            return Err(format!(
                "transaction {number}: exactly one split must have source=true; found {}",
                source_indexes.len()
            ));
        }
    };

    let mut balance = 0_i128;
    let mut splits = Vec::with_capacity(transaction.splits.len());
    for (split_index, split) in transaction.splits.into_iter().enumerate() {
        validate_text(&split.account, "account", number)?;
        validate_optional_text(&split.memo, "memo", number)?;
        let amount = parse_amount(&split.amount, decimal_places)
            .map_err(|error| format!("transaction {number}, split {}: {error}", split_index + 1))?;
        balance = balance
            .checked_add(amount)
            .ok_or_else(|| format!("transaction {number}: balance overflow"))?;
        splits.push(Split {
            memo: split.memo,
            account: split.account,
            amount,
        });
    }
    if balance != 0 {
        return Err(format!(
            "transaction {number}: splits do not balance (sum {})",
            format_number(balance, '.', decimal_places)
        ));
    }

    Ok(Transaction {
        date: transaction.date,
        description: transaction.description,
        splits,
        source_index,
    })
}

fn validate_text(value: &str, field: &str, transaction: usize) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("transaction {transaction}: {field} is empty"));
    }
    validate_optional_text(value, field, transaction)
}

fn validate_optional_text(value: &str, field: &str, transaction: usize) -> Result<(), String> {
    if value.contains(['\n', '\r']) {
        return Err(format!(
            "transaction {transaction}: {field} cannot contain a line break"
        ));
    }
    Ok(())
}

fn parse_amount(raw: &str, decimal_places: u32) -> Result<i128, String> {
    let value = raw.trim();
    if value.is_empty() {
        return Err("amount is empty".to_string());
    }
    let (negative, unsigned) = if let Some(rest) = value.strip_prefix('-') {
        (true, rest)
    } else if let Some(rest) = value.strip_prefix('+') {
        (false, rest)
    } else {
        (false, value)
    };
    if unsigned.is_empty() {
        return Err(format!("invalid canonical amount '{raw}'"));
    }

    let mut parts = unsigned.split('.');
    let whole = parts.next().unwrap_or_default();
    let fraction = parts.next().unwrap_or_default();
    if parts.next().is_some()
        || (whole.is_empty() && fraction.is_empty())
        || !whole.chars().all(|character| character.is_ascii_digit())
        || !fraction.chars().all(|character| character.is_ascii_digit())
    {
        return Err(format!("invalid canonical amount '{raw}'"));
    }
    if fraction.len() > decimal_places as usize {
        return Err(format!(
            "amount '{raw}' has more than {decimal_places} fractional digits"
        ));
    }

    let scale = 10_i128.pow(decimal_places);
    let whole_value: i128 = if whole.is_empty() {
        0
    } else {
        whole
            .parse()
            .map_err(|_| format!("amount '{raw}' is too large"))?
    };
    let mut minor = whole_value
        .checked_mul(scale)
        .ok_or_else(|| format!("amount '{raw}' is too large"))?;
    if !fraction.is_empty() {
        let fraction_value: i128 = fraction
            .parse()
            .map_err(|_| format!("invalid canonical amount '{raw}'"))?;
        let padding = decimal_places - fraction.len() as u32;
        let scaled_fraction = fraction_value
            .checked_mul(10_i128.pow(padding))
            .ok_or_else(|| format!("amount '{raw}' is too large"))?;
        minor = minor
            .checked_add(scaled_fraction)
            .ok_or_else(|| format!("amount '{raw}' is too large"))?;
    }
    if negative {
        minor
            .checked_neg()
            .ok_or_else(|| format!("amount '{raw}' is too large"))
    } else {
        Ok(minor)
    }
}

fn render_markdown(document: &Document, config: &RenderConfig) -> String {
    let mut markdown = String::new();
    if !document.review_notes.is_empty() {
        markdown.push_str("> **Before entry**\n");
        for note in &document.review_notes {
            markdown.push_str("> - ");
            markdown.push_str(note);
            markdown.push('\n');
        }
        markdown.push('\n');
    }
    if let Some(description) = &document.source_description {
        markdown.push_str(description);
        markdown.push_str("\n\n");
    }

    for (transaction_index, transaction) in document.transactions.iter().enumerate() {
        if transaction_index > 0 {
            markdown.push('\n');
        }
        markdown.push_str("| Date | Description | Memo | Account | Amount |\n");
        markdown.push_str("|------|-------------|------|---------|--------|\n");

        let ordered_splits = transaction
            .splits
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != transaction.source_index)
            .chain(std::iter::once((
                transaction.source_index,
                &transaction.splits[transaction.source_index],
            )));
        for (row, (_, split)) in ordered_splits.enumerate() {
            let date = if row == 0 { &transaction.date } else { "" };
            let description = if row == 0 {
                &transaction.description
            } else {
                ""
            };
            let amount = format_markdown_amount(split.amount, config);
            markdown.push_str(&format!(
                "| {} | {} | {} | {} | {} |\n",
                escape_markdown_cell(date),
                escape_markdown_cell(description),
                escape_markdown_cell(&split.memo),
                escape_markdown_cell(&split.account),
                escape_markdown_cell(&amount),
            ));
        }
    }
    markdown
}

fn format_markdown_amount(value: i128, config: &RenderConfig) -> String {
    let sign = if value < 0 { "-" } else { "" };
    let number = format_number(
        value.abs(),
        config.markdown_decimal_separator,
        config.decimal_places,
    );
    match config.currency_symbol_position {
        SymbolPosition::Prefix => format!("{sign}{}{number}", config.currency_symbol),
        SymbolPosition::Suffix => format!("{sign}{number}{}", config.currency_symbol),
    }
}

fn escape_markdown_cell(value: &str) -> String {
    value.replace('\\', "\\\\").replace('|', "\\|")
}

/// Renders account-grouped QIF transactions so one file can import multiple source accounts.
fn render_qif(transactions: &[Transaction], config: &RenderConfig) -> Result<String, String> {
    let mut qif = String::new();
    let mut source_accounts = Vec::new();
    for transaction in transactions {
        let account = &transaction.splits[transaction.source_index].account;
        if !source_accounts.contains(account) {
            source_accounts.push(account.clone());
        }
    }

    for account in source_accounts {
        let account_type = qif_account_type(&account);
        qif.push_str("!Account\nN");
        qif.push_str(&account);
        qif.push_str("\nT");
        qif.push_str(account_type);
        qif.push_str("\n^\n!Type:");
        qif.push_str(account_type);
        qif.push('\n');

        for transaction in transactions {
            let source = &transaction.splits[transaction.source_index];
            if source.account != account {
                continue;
            }
            qif.push('D');
            qif.push_str(&format_qif_date(&transaction.date)?);
            qif.push_str("\nT");
            qif.push_str(&format_number(source.amount, '.', config.decimal_places));
            qif.push_str("\nP");
            qif.push_str(&transaction.description);
            qif.push('\n');

            for (index, split) in transaction.splits.iter().enumerate() {
                if index == transaction.source_index {
                    continue;
                }
                qif.push('S');
                qif.push_str(&qif_split_account(&split.account));
                qif.push('\n');
                if !split.memo.is_empty() {
                    qif.push('E');
                    qif.push_str(&split.memo);
                    qif.push('\n');
                }
                qif.push('$');
                qif.push_str(&format_number(-split.amount, '.', config.decimal_places));
                qif.push('\n');
            }
            qif.push_str("^\n");
        }
    }
    Ok(qif)
}

/// Maps source account paths to QIF account types used by GnuCash's importer.
fn qif_account_type(account: &str) -> &'static str {
    if account.starts_with("Liabilities:") {
        "CCard"
    } else {
        "Bank"
    }
}

/// Marks balance-sheet destinations as transfers while leaving income and expense categories plain.
fn qif_split_account(account: &str) -> String {
    if account.starts_with("Expenses:") || account.starts_with("Income:") {
        account.to_string()
    } else {
        format!("[{account}]")
    }
}

/// Converts canonical ISO dates to the unambiguous US QIF date written by this renderer.
fn format_qif_date(date: &str) -> Result<String, String> {
    let bytes = date.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return Err(format!("invalid ISO date '{date}'"));
    }
    let year = &date[0..4];
    let month = &date[5..7];
    let day = &date[8..10];
    if !year.chars().all(|value| value.is_ascii_digit())
        || !month.chars().all(|value| value.is_ascii_digit())
        || !day.chars().all(|value| value.is_ascii_digit())
    {
        return Err(format!("invalid ISO date '{date}'"));
    }
    let month_number: u8 = month
        .parse()
        .map_err(|_| format!("invalid ISO date '{date}'"))?;
    let day_number: u8 = day
        .parse()
        .map_err(|_| format!("invalid ISO date '{date}'"))?;
    if !(1..=12).contains(&month_number) || !(1..=31).contains(&day_number) {
        return Err(format!("invalid ISO date '{date}'"));
    }
    Ok(format!("{month}/{day}/{year}"))
}

fn format_number(value: i128, decimal_separator: char, decimal_places: u32) -> String {
    let negative = value < 0;
    let absolute = value.unsigned_abs();
    let scale = 10_u128.pow(decimal_places);
    let sign = if negative { "-" } else { "" };
    if decimal_places == 0 {
        return format!("{sign}{absolute}");
    }
    format!(
        "{sign}{}{}{:0width$}",
        absolute / scale,
        decimal_separator,
        absolute % scale,
        width = decimal_places as usize
    )
}

fn source_totals(transactions: &[Transaction]) -> Result<BTreeMap<String, i128>, String> {
    let mut totals = BTreeMap::new();
    for transaction in transactions {
        let source = &transaction.splits[transaction.source_index];
        let total = totals.entry(source.account.clone()).or_insert(0_i128);
        *total = total
            .checked_add(source.amount)
            .ok_or_else(|| "source account total overflow".to_string())?;
    }
    Ok(totals)
}

fn default_markdown_path(input: &Path) -> PathBuf {
    let stem = input
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("transactions");
    input.with_file_name(format!("{stem}.md"))
}

fn default_qif_path(input: &Path) -> PathBuf {
    let stem = input
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("transactions");
    input.with_file_name(format!("{stem}_gnucash.qif"))
}

fn validate_output_paths(input: &Path, markdown: &Path, qif: &Path) -> Result<(), String> {
    for output in [markdown, qif] {
        let parent = output.parent().unwrap_or_else(|| Path::new("."));
        if !parent.as_os_str().is_empty() && !parent.is_dir() {
            return Err(format!(
                "output directory does not exist: {}",
                parent.display()
            ));
        }
    }
    if paths_match(input, markdown)? || paths_match(input, qif)? {
        return Err("input and output paths must differ".to_string());
    }
    if paths_match(markdown, qif)? {
        return Err("Markdown and QIF output paths must differ".to_string());
    }
    Ok(())
}

fn paths_match(left: &Path, right: &Path) -> Result<bool, String> {
    Ok(comparable_path(left)? == comparable_path(right)?)
}

fn comparable_path(path: &Path) -> Result<PathBuf, String> {
    if path.exists() {
        return path
            .canonicalize()
            .map_err(|error| format!("cannot resolve {}: {error}", path.display()));
    }
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let parent = if parent.as_os_str().is_empty() {
        Path::new(".")
    } else {
        parent
    };
    let canonical_parent = parent
        .canonicalize()
        .map_err(|error| format!("cannot resolve {}: {error}", parent.display()))?;
    let file_name = path
        .file_name()
        .ok_or_else(|| format!("path has no file name: {}", path.display()))?;
    Ok(canonical_parent.join(file_name))
}

fn write_output_pair(
    markdown_path: &Path,
    markdown: &[u8],
    qif_path: &Path,
    qif: &[u8],
    force: bool,
) -> Result<(), String> {
    if !force {
        let existing: Vec<String> = [markdown_path, qif_path]
            .into_iter()
            .filter(|path| path.exists())
            .map(|path| path.display().to_string())
            .collect();
        if !existing.is_empty() {
            return Err(format!(
                "output file already exists: {} (use --force to replace both outputs)",
                existing.join(", ")
            ));
        }
    }

    let markdown_temp = write_temp_file(markdown_path, "markdown", markdown)?;
    let qif_temp = match write_temp_file(qif_path, "qif", qif) {
        Ok(path) => path,
        Err(error) => {
            let _ = fs::remove_file(&markdown_temp);
            return Err(error);
        }
    };

    let commit = (|| {
        commit_temp_file(&markdown_temp, markdown_path, force)?;
        commit_temp_file(&qif_temp, qif_path, force)
    })();
    if commit.is_err() {
        let _ = fs::remove_file(&markdown_temp);
        let _ = fs::remove_file(&qif_temp);
    }
    commit
}

fn write_temp_file(output: &Path, label: &str, contents: &[u8]) -> Result<PathBuf, String> {
    let file_name = output
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("output path has no UTF-8 file name: {}", output.display()))?;
    let temp = output.with_file_name(format!(".{file_name}.{}.{}.tmp", process::id(), label));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .map_err(|error| format!("cannot create temporary file {}: {error}", temp.display()))?;
    if let Err(error) = file.write_all(contents) {
        let _ = fs::remove_file(&temp);
        return Err(format!(
            "cannot write temporary file {}: {error}",
            temp.display()
        ));
    }
    Ok(temp)
}

fn commit_temp_file(temp: &Path, output: &Path, force: bool) -> Result<(), String> {
    if force {
        fs::rename(temp, output)
            .map_err(|error| format!("cannot replace {}: {error}", output.display()))
    } else {
        fs::hard_link(temp, output)
            .map_err(|error| format!("cannot create {}: {error}", output.display()))?;
        fs::remove_file(temp)
            .map_err(|error| format!("cannot remove temporary file {}: {error}", temp.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_JSON: &str = r#"
{
  "version": 1,
  "source_description": "Example statement transactions.",
  "review_notes": ["Confirm example opening balance."],
  "transactions": [
    {
      "date": "2030-01-02",
      "description": "Example Store",
      "splits": [
        {
          "memo": "Widget, large",
          "account": "Expenses:Supplies",
          "amount": "10.00"
        },
        {
          "account": "Liabilities:Example Card",
          "amount": "-10.00",
          "source": true
        }
      ]
    },
    {
      "date": "2030-01-02",
      "description": "Example Store",
      "splits": [
        {
          "account": "Liabilities:Example Card",
          "amount": "5.00",
          "source": true
        },
        {
          "memo": "Refund: Widget",
          "account": "Expenses:Supplies",
          "amount": "-5.00"
        }
      ]
    }
  ]
}
"#;

    fn config() -> RenderConfig {
        RenderConfig {
            currency_symbol: "$".to_string(),
            currency_symbol_position: SymbolPosition::Prefix,
            markdown_decimal_separator: '.',
            decimal_places: 2,
        }
    }

    fn sample_document() -> Document {
        let input: InputFile = serde_json::from_str(SAMPLE_JSON).unwrap();
        validate_input(input, 2).unwrap()
    }

    #[test]
    fn independently_renders_markdown_and_qif() {
        let document = sample_document();
        let markdown = render_markdown(&document, &config());
        let qif = render_qif(&document.transactions, &config()).unwrap();

        assert!(markdown.starts_with(
            "> **Before entry**\n> - Confirm example opening balance.\n\n\
             Example statement transactions.\n\n"
        ));
        assert!(markdown.contains(
            "| 2030-01-02 | Example Store | Widget, large | Expenses:Supplies | $10.00 |\n\
             |  |  |  | Liabilities:Example Card | -$10.00 |"
        ));
        assert!(markdown.contains(
            "| 2030-01-02 | Example Store | Refund: Widget | Expenses:Supplies | -$5.00 |\n\
             |  |  |  | Liabilities:Example Card | $5.00 |"
        ));
        assert_eq!(
            qif,
            "!Account\n\
             NLiabilities:Example Card\n\
             TCCard\n\
             ^\n\
             !Type:CCard\n\
             D01/02/2030\n\
             T-10.00\n\
             PExample Store\n\
             SExpenses:Supplies\n\
             EWidget, large\n\
             $-10.00\n\
             ^\n\
             D01/02/2030\n\
             T5.00\n\
             PExample Store\n\
             SExpenses:Supplies\n\
             ERefund: Widget\n\
             $5.00\n\
             ^\n"
        );
        assert!(!qif.contains("opening balance"));
    }

    #[test]
    fn renders_qif_transfer_splits_and_multiple_source_accounts() {
        let input: InputFile = serde_json::from_str(
            r#"{
              "version": 1,
              "transactions": [
                {
                  "date": "2030-02-01",
                  "description": "Example Store",
                  "splits": [
                    {"account": "Expenses:Supplies", "amount": "167.75"},
                    {"memo": "Gift-card balance", "account": "Assets:Gift Cards", "amount": "-130.65"},
                    {"account": "Liabilities:Card One", "amount": "-37.10", "source": true}
                  ]
                },
                {
                  "date": "2030-02-02",
                  "description": "Other Store",
                  "splits": [
                    {"account": "Expenses:Supplies", "amount": "20.00"},
                    {"account": "Liabilities:Card Two", "amount": "-20.00", "source": true}
                  ]
                }
              ]
            }"#,
        )
        .unwrap();
        let document = validate_input(input, 2).unwrap();
        let qif = render_qif(&document.transactions, &config()).unwrap();

        assert_eq!(qif.matches("!Account\n").count(), 2);
        assert!(qif.contains("NLiabilities:Card One\nTCCard\n^\n!Type:CCard\n"));
        assert!(qif.contains("SExpenses:Supplies\n$-167.75\n"));
        assert!(qif.contains("S[Assets:Gift Cards]\nEGift-card balance\n$130.65\n"));
        assert!(qif.contains("NLiabilities:Card Two\nTCCard\n^\n!Type:CCard\n"));
    }

    #[test]
    fn rejects_unbalanced_transaction() {
        let input: InputFile = serde_json::from_str(
            r#"{
              "version": 1,
              "transactions": [{
                "date": "2030-02-01",
                "description": "Example",
                "splits": [
                  {"account": "Expenses:Supplies", "amount": "9.00"},
                  {"account": "Liabilities:Example Card", "amount": "-10.00", "source": true}
                ]
              }]
            }"#,
        )
        .unwrap();
        let error = validate_input(input, 2).unwrap_err();
        assert!(error.contains("do not balance"));
    }

    #[test]
    fn requires_exactly_one_source_split() {
        let input: InputFile = serde_json::from_str(
            r#"{
              "version": 1,
              "transactions": [{
                "date": "2030-02-01",
                "description": "Example",
                "splits": [
                  {"account": "Expenses:Supplies", "amount": "10.00"},
                  {"account": "Liabilities:Example Card", "amount": "-10.00"}
                ]
              }]
            }"#,
        )
        .unwrap();
        let error = validate_input(input, 2).unwrap_err();
        assert!(error.contains("found none"));
    }

    #[test]
    fn parses_canonical_amounts_exactly() {
        assert_eq!(parse_amount("1234.50", 2).unwrap(), 123_450);
        assert_eq!(parse_amount("-.50", 2).unwrap(), -50);
        assert_eq!(parse_amount("+12", 2).unwrap(), 1_200);
        for invalid in ["", "-", "$1.00", "1,000.00", "1.2.3", "1e3"] {
            assert!(parse_amount(invalid, 2).is_err(), "accepted {invalid}");
        }
    }

    #[test]
    fn supports_configurable_markdown() {
        let mut config = config();
        config.currency_symbol = " EUR".to_string();
        config.currency_symbol_position = SymbolPosition::Suffix;
        config.markdown_decimal_separator = ',';

        let document = sample_document();
        let markdown = render_markdown(&document, &config);
        assert!(markdown.contains("10,00 EUR"));
    }

    #[test]
    fn escapes_markdown_pipes_and_backslashes() {
        assert_eq!(
            escape_markdown_cell(r"Part A | C:\Temp"),
            r"Part A \| C:\\Temp"
        );
    }

    #[test]
    fn rejects_unknown_json_fields() {
        let error = serde_json::from_str::<InputFile>(
            r#"{"version":1,"transactions":[],"unexpected":true}"#,
        )
        .unwrap_err();
        assert!(error.to_string().contains("unknown field"));
    }
}
