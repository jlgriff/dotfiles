use clap::Parser;
use regex::Regex;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
#[command(
    about = "Extract order details from saved Amazon order HTML files.",
    long_about = "Extract order details from saved Amazon order HTML files.\n\n\
        Parses both standard Amazon orders and Amazon Fresh/Whole Foods\n\
        orders, producing a JSON summary. Tracks which files have already\n\
        been processed in a .processed file so subsequent runs only handle\n\
        new files.",
    after_help = "\
Saving order HTML files:
  1. Go to https://www.amazon.com/gp/your-account/order-history
  2. Click an order to open its details page
  3. Save the page as HTML (Ctrl+S or Cmd+S), selecting \"Webpage, HTML Only\"
  4. Save/move the file into your input directory (e.g. ~/Downloads/Orders/Amazon)
  5. Repeat for each order, then run this tool with -i <input-dir>"
)]
struct Cli {
    /// Directory containing Amazon order HTML files to process.
    /// Each file should be a saved Amazon order details page.
    /// Files starting with '.' and 'amazon_orders_summary.json' are ignored.
    #[arg(short, long, value_name = "DIR")]
    input_dir: PathBuf,

    /// Directory to write 'amazon_orders_summary.json' into.
    /// Defaults to the input directory if not specified.
    /// The '.processed' tracking file always stays in the input directory.
    #[arg(short, long, value_name = "DIR")]
    output_dir: Option<PathBuf>,

    /// Process all files, ignoring the '.processed' tracking file.
    /// Useful for re-extracting everything from scratch.
    /// The '.processed' file is rebuilt afterward to reflect all files.
    #[arg(short, long)]
    all: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Item {
    name: String,
    price: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Order {
    order_id: String,
    date: String,
    items: Vec<Item>,
    subtotal: String,
    shipping: String,
    tax: String,
    total: String,
    refund: String,
    source: String,
}

// ---------------------------------------------------------------------------
// HTML helpers
// ---------------------------------------------------------------------------

fn text_of(element: &scraper::ElementRef) -> String {
    let raw: String = element.text().collect::<Vec<_>>().join(" ");
    raw.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn select_first<'a>(doc: &'a Html, selector_str: &str) -> Option<scraper::ElementRef<'a>> {
    let sel = Selector::parse(selector_str).ok()?;
    doc.select(&sel).next()
}

fn select_all<'a>(doc: &'a Html, selector_str: &str) -> Vec<scraper::ElementRef<'a>> {
    match Selector::parse(selector_str) {
        Ok(sel) => doc.select(&sel).collect(),
        Err(_) => vec![],
    }
}

fn extract_dollar(text: &str) -> String {
    let re = Regex::new(r"\$([\d,]+\.\d{2})").unwrap();
    match re.find(text) {
        Some(m) => m.as_str().to_string(),
        None => String::new(),
    }
}

// ---------------------------------------------------------------------------
// Standard Amazon order parsing
// ---------------------------------------------------------------------------

fn parse_standard(doc: &Html, html: &str, _filename: &str) -> Option<Order> {
    // Order ID
    let order_id = select_first(doc, r#"div[data-component="orderId"] span"#)
        .map(|el| text_of(&el))
        .filter(|s| !s.is_empty())?;

    // Order date
    let date = select_first(doc, r#"div[data-component="orderDate"] span"#)
        .map(|el| text_of(&el))
        .unwrap_or_default();

    // Items
    let title_elements = select_all(doc, r#"div[data-component="itemTitle"]"#);
    let price_elements = select_all(doc, r#"div[data-component="unitPrice"]"#);

    let mut items = Vec::new();
    for (idx, title_el) in title_elements.iter().enumerate() {
        let name = match Selector::parse("a").ok() {
            Some(sel) => title_el
                .select(&sel)
                .next()
                .map(|a| text_of(&a))
                .unwrap_or_else(|| text_of(title_el)),
            None => text_of(title_el),
        };

        let price = if idx < price_elements.len() {
            match Selector::parse(".a-offscreen").ok() {
                Some(sel) => price_elements[idx]
                    .select(&sel)
                    .next()
                    .map(|el| text_of(&el))
                    .unwrap_or_default(),
                None => String::new(),
            }
        } else {
            String::new()
        };

        items.push(Item { name, price });
    }

    // Grand Total
    let total = extract_total(html);

    // Charge line items
    let subtotal = extract_charge_line(html, r"Item\(s\) Subtotal");
    let shipping = extract_charge_line(html, r"Shipping &(?:amp;)? Handling");
    let tax = extract_charge_line(html, r"Estimated tax to be collected");

    // Refund
    let refund = extract_refund(html);

    Some(Order {
        order_id,
        date,
        items,
        subtotal,
        shipping,
        tax,
        total,
        refund,
        source: "amazon".to_string(),
    })
}

fn extract_total(html: &str) -> String {
    let re = Regex::new(
        r#"(?s)Grand Total:.*?</span>.*?<span[^>]*a-text-bold[^>]*>\s*(\$[\d,]+\.\d{2})"#,
    )
    .unwrap();
    re.captures(html)
        .map(|c| c[1].trim().to_string())
        .unwrap_or_default()
}

fn extract_charge_line(html: &str, label_pattern: &str) -> String {
    let pattern = format!(
        r#"(?s){label_pattern}.*?</div>.*?<div[^>]*od-line-item-row-content[^>]*>.*?\$([\d,]+\.\d{{2}})"#
    );
    Regex::new(&pattern)
        .ok()
        .and_then(|re| re.captures(html))
        .map(|c| format!("${}", &c[1]))
        .unwrap_or_default()
}

fn extract_refund(html: &str) -> String {
    let re = Regex::new(
        r#"(?s)Refund Total.*?</span>.*?<span[^>]*a-text-bold[^>]*>\s*(\$[\d,]+\.\d{2})"#,
    )
    .unwrap();
    re.captures(html)
        .map(|c| c[1].trim().to_string())
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Amazon Fresh / Whole Foods order parsing
// ---------------------------------------------------------------------------

fn parse_fresh(doc: &Html, html: &str, _filename: &str) -> Option<Order> {
    // Order ID — "Order #: 111-1234567-1234567"
    let order_id = Regex::new(r"Order\s*#:\s*([\d-]+)")
        .unwrap()
        .captures(html)
        .map(|c| c[1].trim().to_string())?;

    // Order date — "Ordered March 18, 2026 8:47AM"
    let date = Regex::new(r"Ordered\s+(\w+ \d{1,2},\s*\d{4})\s*\d{1,2}:\d{2}\s*[AP]M")
        .unwrap()
        .captures(html)
        .map(|c| c[1].trim().to_string())
        .unwrap_or_default();

    // Items from detail grid rows (id="<ASIN>-item-grid-row")
    let row_re = Regex::new(r#"id="([^"]+)-item-grid-row""#).unwrap();
    let a_sel = Selector::parse("a").unwrap();
    let mut items = Vec::new();

    for cap in row_re.captures_iter(html) {
        let asin = &cap[1];
        let row_selector_str = format!(r#"div[id="{asin}-item-grid-row"]"#);
        if let Some(row_el) = select_first(doc, &row_selector_str) {
            // Product name from <a> tag
            let name = row_el
                .select(&a_sel)
                .next()
                .map(|a| text_of(&a))
                .unwrap_or_default();

            // Price from <span id="ASIN-item-total-price">
            let price_selector_str = format!(r#"span[id="{asin}-item-total-price"]"#);
            let price = select_first(doc, &price_selector_str)
                .map(|el| extract_dollar(&text_of(&el)))
                .unwrap_or_default();

            if !name.is_empty() {
                items.push(Item { name, price });
            }
        }
    }

    // Grand total
    let total = select_first(doc, r#"span[id="ufpo-grand-total-amount"]"#)
        .map(|el| extract_dollar(&text_of(&el)))
        .unwrap_or_default();

    // Subtotal
    let subtotal = select_first(doc, r#"span[id="ufpo-itemsSubtotal-amount"]"#)
        .map(|el| extract_dollar(&text_of(&el)))
        .unwrap_or_default();

    // Tax
    let tax = select_first(doc, r#"span[id="ufpo-totalTax-amount"]"#)
        .map(|el| extract_dollar(&text_of(&el)))
        .unwrap_or_default();

    Some(Order {
        order_id,
        date,
        items,
        subtotal,
        shipping: String::new(),
        tax,
        total,
        refund: String::new(),
        source: "amazon".to_string(),
    })
}

// ---------------------------------------------------------------------------
// Format detection
// ---------------------------------------------------------------------------

fn detect_and_parse(html: &str, filename: &str) -> Option<Order> {
    let doc = Html::parse_document(html);
    if html.contains("ufpo-grand-total-amount") || html.contains("ufpo-order-summary") {
        parse_fresh(&doc, html, filename)
    } else if html.contains(r#"data-component="orderDate""#) {
        parse_standard(&doc, html, filename)
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Processed-file tracking
// ---------------------------------------------------------------------------

fn load_processed(path: &PathBuf) -> BTreeSet<String> {
    match fs::read_to_string(path) {
        Ok(contents) => contents
            .lines()
            .filter(|l| !l.is_empty())
            .map(String::from)
            .collect(),
        Err(_) => BTreeSet::new(),
    }
}

fn save_processed(path: &PathBuf, processed: &BTreeSet<String>) {
    let contents: String = processed.iter().cloned().collect::<Vec<_>>().join("\n") + "\n";
    fs::write(path, contents).expect("Failed to write .processed file");
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    let cli = Cli::parse();

    let input_dir = cli.input_dir.canonicalize().unwrap_or_else(|_| {
        eprintln!(
            "error: Input directory does not exist: {}",
            cli.input_dir.display()
        );
        process::exit(2);
    });

    if !input_dir.is_dir() {
        eprintln!(
            "error: Input path is not a directory: {}",
            input_dir.display()
        );
        process::exit(2);
    }

    let output_dir = cli.output_dir.unwrap_or_else(|| input_dir.clone());
    fs::create_dir_all(&output_dir).expect("Failed to create output directory");

    let processed_file = input_dir.join(".processed");
    let output_file = output_dir.join("amazon_orders_summary.json");

    let mut processed = if cli.all {
        BTreeSet::new()
    } else {
        load_processed(&processed_file)
    };

    let mut entries: Vec<_> = fs::read_dir(&input_dir)
        .expect("Failed to read input directory")
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            e.file_type().map(|ft| ft.is_file()).unwrap_or(false)
                && !name.starts_with('.')
                && name != "amazon_orders_summary.json"
        })
        .collect();
    entries.sort_by_key(|e| e.file_name());

    let mut new_orders: Vec<Order> = Vec::new();
    let mut seen_order_ids: BTreeSet<String> = BTreeSet::new();
    let mut skipped = 0usize;
    let mut errors: Vec<String> = Vec::new();

    for entry in &entries {
        let fname = entry.file_name().to_string_lossy().to_string();

        if processed.contains(&fname) {
            skipped += 1;
            continue;
        }

        let html = match fs::read_to_string(entry.path()) {
            Ok(h) => h,
            Err(e) => {
                eprintln!("  ERR   {fname} ({e})");
                errors.push(fname);
                continue;
            }
        };

        match detect_and_parse(&html, &fname) {
            Some(order) => {
                if seen_order_ids.contains(&order.order_id) {
                    println!("  DUP   {fname}  (duplicate of {})", order.order_id);
                } else {
                    let item_count = order.items.len();
                    println!(
                        "  OK    {}  {}  {}  ({} items)",
                        fname, order.date, order.total, item_count
                    );
                    seen_order_ids.insert(order.order_id.clone());
                    new_orders.push(order);
                }
                processed.insert(fname);
            }
            None => {
                println!("  SKIP  {fname} (unrecognised format)");
                errors.push(fname);
            }
        }
    }

    // Merge with existing summary (--all replaces entirely)
    let new_count = new_orders.len();
    let mut all_orders = if cli.all {
        new_orders
    } else {
        let mut existing: Vec<Order> = if output_file.exists() {
            fs::read_to_string(&output_file)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        let existing_ids: BTreeSet<String> =
            existing.iter().map(|o| o.order_id.clone()).collect();
        for o in new_orders {
            if !existing_ids.contains(&o.order_id) {
                existing.push(o);
            }
        }
        existing
    };

    all_orders.sort_by(|a, b| a.date.cmp(&b.date));
    let json = serde_json::to_string_pretty(&all_orders).expect("Failed to serialize orders");
    fs::write(&output_file, json + "\n").expect("Failed to write order summary");
    save_processed(&processed_file, &processed);

    println!();
    println!("Processed: {new_count} new orders");
    println!("Skipped:   {skipped} already processed");
    if !errors.is_empty() {
        println!(
            "Errors:    {} unrecognised ({})",
            errors.len(),
            errors.join(", ")
        );
    }
    println!("Summary:   {}", output_file.display());
}
