use clap::Parser;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
#[command(
    about = "Extract order details from saved Walmart order PDF or HTML files.",
    long_about = "Extract order details from saved Walmart order PDF or HTML files.\n\n\
        Produces a JSON summary with the same structure as amazon-order-extract.\n\
        Tracks which files have already been processed in a .processed file so\n\
        subsequent runs only handle new files.\n\n\
        PDF invoices (printed from the order details page) are the preferred\n\
        format — they include per-line quantities. Parsing PDFs requires the\n\
        'pdftotext' tool from poppler-utils.",
    after_help = "\
Saving order invoices (recommended — PDF):
  1. Go to https://www.walmart.com/orders
  2. Click an order, then open its invoice/receipt view
  3. Print to PDF (Ctrl+P -> Save as PDF) into your input directory
     (e.g. ~/Downloads/Orders/Walmart)
  4. Repeat for each order, then run this tool with -i <input-dir>

HTML saves (legacy) are also accepted, but lack quantities.
Requires poppler-utils (pdftotext) for PDF parsing."
)]
struct Cli {
    /// Directory containing Walmart order PDF or HTML files to process.
    /// Each file should be a saved Walmart order invoice (PDF) or details page (HTML).
    /// Files starting with '.', directories, and 'walmart_orders_summary.json' are ignored.
    #[arg(short, long, value_name = "DIR")]
    input_dir: PathBuf,

    /// Directory to write 'walmart_orders_summary.json' into.
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
    /// Line-item total (the amount that contributes to the subtotal), not the
    /// per-unit price. For a line with quantity > 1 this is the combined price.
    price: String,
    /// Number of units on this line ("1" when unknown).
    #[serde(default = "one")]
    quantity: String,
}

fn one() -> String {
    "1".to_string()
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
    discount: String,
    refund: String,
    card_last4: String,
    source: String,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn html_decode(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#x27;", "'")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
}

// ---------------------------------------------------------------------------
// Walmart order parsing
// ---------------------------------------------------------------------------

fn parse_walmart(html: &str) -> Option<Order> {
    // Order ID — "Order# 2000140-94533258"
    let order_id = Regex::new(r"Order#\s+([\d-]+)")
        .unwrap()
        .captures(html)
        .map(|c| c[1].trim().to_string())?;

    // Order date — class="...print-bill-date...">Mar 13, 2026 order</h1>
    let date = Regex::new(r"print-bill-date[^>]*>(\w+ \d+, \d+)")
        .unwrap()
        .captures(html)
        .map(|c| c[1].trim().to_string())
        .unwrap_or_default();

    // Item names — class="...print-item-title...">...<span ...>Item Name</span>
    let title_re =
        Regex::new(r#"print-item-title[^>]*>.*?<span[^>]*>([^<]+)</span>"#).unwrap();
    let names: Vec<String> = title_re
        .captures_iter(html)
        .map(|c| html_decode(c[1].trim()))
        .collect();

    // Item prices — data-testid="line-price" ...>$XX.XX</span>
    let price_re =
        Regex::new(r#"line-price[^>]*>[^>]*>\$?([\d,.]+)</span>"#).unwrap();
    let prices: Vec<String> = price_re
        .captures_iter(html)
        .map(|c| format!("${}", &c[1]))
        .collect();

    let mut items = Vec::new();
    for (idx, name) in names.iter().enumerate() {
        let price = prices.get(idx).cloned().unwrap_or_default();
        items.push(Item {
            name: name.clone(),
            price,
            quantity: "1".to_string(),
        });
    }

    // Subtotal — "Subtotal</span>...$XXX.XX"
    // Use "Subtotal after savings" if present, otherwise plain subtotal.
    let subtotal_val: f64 = Regex::new(r"Subtotal after savings, \$([\d,.]+)")
        .unwrap()
        .captures(html)
        .or_else(|| {
            Regex::new(r"Subtotal</span>.*?\$([\d,.]+)")
                .unwrap()
                .captures(html)
        })
        .and_then(|c| c[1].replace(',', "").parse().ok())
        .unwrap_or(0.0);
    let subtotal = Regex::new(r"Subtotal</span>.*?\$([\d,.]+)")
        .unwrap()
        .captures(html)
        .map(|c| format!("${}", &c[1]))
        .unwrap_or_default();

    // Tax — "Tax $X.XX</span>"
    let tax_val: f64 = Regex::new(r"Tax \$([\d,.]+)</span>")
        .unwrap()
        .captures(html)
        .and_then(|c| c[1].replace(',', "").parse().ok())
        .unwrap_or(0.0);
    let tax = if tax_val > 0.0 {
        format!("${:.2}", tax_val)
    } else {
        String::new()
    };

    // Discount / Savings — aria-label="Savings, $XX.XX"
    let discount = Regex::new(r#"aria-label="Savings, \$([\d,.]+)""#)
        .unwrap()
        .captures(html)
        .map(|c| format!("-${}", &c[1]))
        .unwrap_or_default();

    // Total — class="...bill-order-total-payment..."...$XXX.XX
    let total = Regex::new(r"bill-order-total-payment.*?\$([\d,.]+)")
        .unwrap()
        .captures(html)
        .map(|c| format!("${}", &c[1]))
        .unwrap_or_default();

    // Refund — sum line-prices of items under "Refund issued" group headers,
    // then apply the order's effective tax rate (tax / subtotal) to estimate
    // the total refund including tax.
    let group_re = Regex::new(r"Delivered on \w+ \d+|Refund issued(?:\s+on\s+\w+ \d+)?").unwrap();
    let line_price_re = Regex::new(r#"line-price[^>]*>[^>]*>\$?([\d,.]+)</span>"#).unwrap();

    let group_positions: Vec<(usize, bool)> = group_re
        .find_iter(html)
        .map(|m| (m.start(), m.as_str().starts_with("Refund")))
        .collect();
    let price_positions: Vec<(usize, f64)> = line_price_re
        .captures_iter(html)
        .filter_map(|c| {
            let pos = c.get(0)?.start();
            let val: f64 = c[1].replace(',', "").parse().ok()?;
            Some((pos, val))
        })
        .collect();

    let mut refund_items_sum: f64 = 0.0;
    for (i, &(gpos, is_refund)) in group_positions.iter().enumerate() {
        if !is_refund {
            continue;
        }
        let next_gpos = group_positions.get(i + 1).map(|g| g.0).unwrap_or(usize::MAX);
        for &(ppos, val) in &price_positions {
            if ppos > gpos && ppos < next_gpos {
                refund_items_sum += val;
            }
        }
    }

    let refund = if refund_items_sum > 0.0 {
        let tax_rate = if subtotal_val > 0.0 {
            tax_val / subtotal_val
        } else {
            0.0
        };
        let refund_with_tax = refund_items_sum * (1.0 + tax_rate);
        format!("${:.2}", refund_with_tax)
    } else {
        String::new()
    };

    Some(Order {
        order_id,
        date,
        items,
        subtotal,
        shipping: String::new(),
        tax,
        total,
        discount,
        refund,
        card_last4: Regex::new(r"(?i)ending in (\d{4})")
            .unwrap()
            .captures(html)
            .map(|c| c[1].to_string())
            .unwrap_or_default(),
        source: "walmart".to_string(),
    })
}

// ---------------------------------------------------------------------------
// Walmart invoice PDF parsing
// ---------------------------------------------------------------------------

/// Run `pdftotext` (poppler-utils) and return its stdout. `layout` selects the
/// `-layout` mode, which keeps each footer label on the same line as its value;
/// the default (raw) mode lays items out one field per line.
fn pdftotext(path: &std::path::Path, layout: bool) -> Option<String> {
    let mut cmd = std::process::Command::new("pdftotext");
    if layout {
        cmd.arg("-layout");
    }
    let output = cmd.arg(path).arg("-").output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Parse a saved Walmart order invoice PDF. `raw` is the default pdftotext
/// dump (used for the item list), `layout` is the `-layout` dump (used for the
/// footer totals). The price shown on a PDF line is the line total for that
/// quantity, so `price` is stored as-is and `quantity` from the "Qty N" marker.
fn parse_walmart_pdf(raw: &str, layout: &str) -> Option<Order> {
    let order_id = Regex::new(r"Order#\s+([\d-]+)")
        .unwrap()
        .captures(raw)
        .map(|c| c[1].trim().to_string())?;

    let date = Regex::new(r"(\w+ \d{1,2}, \d{4})\s+order")
        .unwrap()
        .captures(raw)
        .map(|c| c[1].trim().to_string())
        .unwrap_or_default();

    // Items: in raw mode each item is a block of one-or-more consecutive
    // name lines, a blank line, "Qty N", a blank line, then "$price". Scan
    // imperatively so header/footer text (which never sits directly above a
    // "Qty N" line) can't bleed into a product name.
    let lines: Vec<&str> = raw.lines().map(|l| l.trim_end()).collect();
    let qty_re = Regex::new(r"^Qty\s+(\d+)$").unwrap();
    let price_re = Regex::new(r"^\$([\d,]+\.\d{2})$").unwrap();
    let mut items = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let qty = match qty_re.captures(line.trim()) {
            Some(c) => c[1].to_string(),
            None => continue,
        };
        // Price: first "$X.XX"-only line below the marker.
        let price = lines[i + 1..]
            .iter()
            .take(4)
            .find_map(|l| price_re.captures(l.trim()).map(|c| format!("${}", &c[1])));
        let price = match price {
            Some(p) => p,
            None => continue,
        };
        // Name: skip the blank line(s) above, then gather consecutive non-blank
        // lines (the wrapped product title) and join them top-to-bottom.
        let mut name_lines: Vec<String> = Vec::new();
        let mut j = i;
        while j > 0 && lines[j - 1].trim().is_empty() {
            j -= 1;
        }
        while j > 0 && !lines[j - 1].trim().is_empty() {
            name_lines.push(lines[j - 1].trim().to_string());
            j -= 1;
        }
        name_lines.reverse();
        let name = html_decode(&name_lines.join(" "));
        if !name.is_empty() {
            items.push(Item {
                name,
                price,
                quantity: qty,
            });
        }
    }

    // Footer totals come from the -layout dump, where each label shares a line
    // with its value. "^\s*Total" can't match the "Subtotal" line.
    let money = |pat: &str| -> Option<f64> {
        Regex::new(pat)
            .ok()?
            .captures(layout)
            .and_then(|c| c[1].replace(',', "").parse::<f64>().ok())
    };
    let fmt = |v: Option<f64>| v.map(|n| format!("${n:.2}")).unwrap_or_default();

    let subtotal = fmt(money(r"(?m)^\s*Subtotal\s+\$([\d,]+\.\d{2})"));
    let tax = fmt(money(r"(?m)^\s*Tax\s+\$([\d,]+\.\d{2})"));
    let total = fmt(money(r"(?m)^\s*Total\s+\$([\d,]+\.\d{2})"));
    let discount = money(r"(?m)^\s*Savings\s+-?\$([\d,]+\.\d{2})")
        .map(|n| format!("-${n:.2}"))
        .unwrap_or_default();

    Some(Order {
        order_id,
        date,
        items,
        subtotal,
        shipping: String::new(),
        tax,
        total,
        discount,
        refund: String::new(),
        card_last4: String::new(),
        source: "walmart".to_string(),
    })
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
    let output_file = output_dir.join("walmart_orders_summary.json");

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
                && name != "walmart_orders_summary.json"
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

        let path = entry.path();
        let is_pdf = path
            .extension()
            .map(|e| e.eq_ignore_ascii_case("pdf"))
            .unwrap_or(false);

        let parsed = if is_pdf {
            match (pdftotext(&path, false), pdftotext(&path, true)) {
                (Some(raw), Some(layout)) => parse_walmart_pdf(&raw, &layout),
                _ => {
                    eprintln!("  ERR   {fname} (pdftotext failed — is poppler-utils installed?)");
                    errors.push(fname);
                    continue;
                }
            }
        } else {
            let html = match fs::read_to_string(&path) {
                Ok(h) => h,
                Err(e) => {
                    eprintln!("  ERR   {fname} ({e})");
                    errors.push(fname);
                    continue;
                }
            };
            if html.trim().is_empty() {
                println!("  SKIP  {fname} (empty file)");
                continue;
            }
            parse_walmart(&html)
        };

        match parsed {
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
