# Finance Extract — AI Transaction Entry Guide

Use this guide along with the four JSON files produced by the finance-extract
tools to generate new GnuCash transactions.

## Re-Extracting Old Orders

The order extractors track ingested files in a `.processed` file inside the
input directory. Never delete `.processed` — the extractor's existing summary
JSON already contains previously-processed orders (new runs merge into it),
and if you do need a full rebuild, pass `--all` to the extractor (it
re-processes every file and rewrites `.processed` automatically).

## Input Files

1. **`amazon_orders_summary.json`** — Amazon order data (order ID, date, items
   with names/prices/quantities, subtotal, tax, shipping, total, refund).
2. **`walmart_orders_summary.json`** — Walmart order data (same schema as Amazon).
3. **`accounts.json`** — All GnuCash account paths grouped by category (assets,
   expenses, income, liabilities, equity). Only use account paths from this file.
4. **`transactions.json`** — Recent GnuCash transactions with date, description,
   and splits (account, amount, memo). Use these as categorization precedent.

Amazon Order Details PDFs can be converted to the first file's schema with
`skills/parse-amazon-invoice-pdfs/`. Its bundled helper extracts every PDF in a
batch with Poppler, and its `SKILL.md` defines parsing and validation rules. Do
not pass PDFs to `amazon-order-extract`; that CLI accepts saved HTML pages.

## Item Quantities and Prices

Each item has a `quantity` field. Interpret `price` as follows:

- **Walmart PDF invoices**, **Amazon PDFs parsed with the repository skill**,
  and **single-item Amazon HTML orders** —
  `price` is the **line total** for that quantity. Item prices sum to the
  subtotal directly; do not multiply by `quantity` (it is informational).
- **Multi-item Amazon orders** — saved Amazon "HTML Only" pages do not contain
  per-line quantities (the page fills them in with JavaScript). For these,
  `quantity` is always `1` and `price` is the **per-unit** price. If the items
  do not sum to the subtotal, one or more lines had a hidden quantity > 1 and
  the extractor cannot tell which. The extractor prints a `WARN` line in this
  case. When you cannot reconcile the items to the subtotal, categorize the
  items you can and place the unexplained remainder in `Imbalance-USD` (the user
  reconciles in GnuCash) — do not guess quantities.

## Source of Truth

The bank/credit card statement CSV is the definitive source of transactions.
Each CSV line becomes one GnuCash transaction. Do not combine separate
credit card charges even if they are related to the same vendor or event
(e.g. a service fee and the main charge should be two transactions). The order
summaries and past
transactions are used to decorate those bank charges with Descriptions, Memos,
and Account categorizations — they do not create transactions on their own.

## Deduplication

Before generating a transaction for a CSV line, check `transactions.json` for
an existing entry with a matching amount on the same date. Be careful with
amount formatting — the CSV may omit trailing zeros (e.g. `$35.3` vs `$35.30`).
Skip any charge that appears to already be entered.

## Canonical Transaction Output

Write the result to `~/Downloads/new_transactions_<YYYY-MM-DD>.json`, where the
date is the generation date. Do not print the JSON inline in chat. This neutral
file is the source of truth for both generated formats; neither generated file
is an input to the other.

Use schema version 1:

```json
{
  "version": 1,
  "source_description": "Example statement transactions.",
  "review_notes": [
    "Confirm the example opening balance before entry."
  ],
  "transactions": [
    {
      "date": "2030-01-02",
      "description": "Example Store",
      "splits": [
        {
          "memo": "Widget",
          "account": "Expenses:Supplies",
          "amount": "10.00"
        },
        {
          "account": "Liabilities:Example Card",
          "amount": "-10.00",
          "source": true
        }
      ]
    }
  ]
}
```

Use only these keys. Top-level `source_description` and `review_notes` are
optional and appear only in the Markdown review file. Use `review_notes` for
confirmed actions that must happen before entry, not unresolved questions.
Split-level `memo` and `source` may be omitted when empty or false. Amounts must
be quoted, signed decimal values with a period separator, no currency symbol,
and no digit grouping. Keep normal currency precision unless the renderer will
be called with a different `--decimal-places` value. Mark exactly one source
split per transaction with `"source": true`; this is normally the statement's
bank or credit-card account.

**Rules:**

- **Date** — use the order date from the JSON. If the user provides a credit
  card statement date that differs, prefer the statement date.
- Every item gets its own row with the correct expense account and a Memo.
- Sales tax gets its own row (`Expenses:Taxes:Sales Tax`) only when a
  transaction has items going to different expense accounts. If all items go
  to the same account, include tax in that single row's amount.
- The source split has the negative total for a purchase and the positive total
  for a refund.
- All amounts must sum to zero.
- Refunds are always separate transactions from the original purchase, even
  for partial refunds. Reverse all signs (credit card row is positive, expense
  rows are negative). Add "Refund: " prefix to the memo.

### Memos

- Keep memos short but specific — use the product name, not a generic category.
  - Good: "Brand Name Size 4 diapers", "Brand Name laundry detergent"
  - Bad: "diapers", "soap"
- When multiple items go to the same account, merge them into a single row
  with a combined memo (e.g. "Brand A diapers, Brand B wipes").
- The credit card row does not need a memo.

### Categorization

- Use `transactions.json` as precedent. If a similar item was categorized in a
  past transaction, use the same account.
- Only use account paths that exist in `accounts.json`.
- Determine the payment account (credit card or bank account) from
  `accounts.json` liabilities/assets. If the user specifies which card was
  used, use that. Otherwise, match a few charges against `transactions.json`
  early to identify which card the CSV export belongs to, then use that card
  consistently for all entries.
- **When unsure about which expense account an item belongs to, batch
  ambiguous charges into a single question to the user before generating
  the output** rather than defaulting everything to `Imbalance-USD`. A wrong
  guess is harder to catch than an explicit Imbalance entry. The user will
  correct these manually.

### Description

Use a clean merchant name, not the raw bank description. Derive it from the CSV
description (e.g. "AMAZON MKTPL*..." → "Amazon", "WALMART.COM..." → "Walmart",
"COLUMBIA GAS OF PENNSYLVA..." → "Columbia Gas"). Check `transactions.json` for
how the same merchant was named previously.

## Matching Orders to Bank Charges

### Amazon

Amazon charges typically post 1–3 days after the order date. Match by total
amount. One order = one charge. The `card_last4` field in the order summary
indicates which card was used — skip orders that don't match the statement's
card.

Amazon refunds appear as negative amounts with descriptions like
"AMAZON MKTPLACE PMTS". Match refund amounts to orders with a `refund` field,
or to individual item prices + tax when the refund is for a single returned item.

### Walmart

Walmart is unpredictable. A single order often results in multiple bank charges
(split shipments), and charges may post days after the order date. Strategies:

- **Exact match**: If a charge equals an order total, it's a direct match.
- **Sum match**: If two or more charges on the same date sum to an order total,
  they are split shipments from one order.
- **Partial match**: Individual charges may correspond to subsets of items + tax
  from an order. This is hard to verify without shipment data.

When charges are split shipments and you cannot determine which items are in
which shipment, use `Imbalance-USD` for those charges rather than guessing.

Walmart refunds appear as negative charges with descriptions like
"WALMART.COM WALMART.COM". Check the order's `refund` field.

### Discounts and Promotions

Order summaries include a `discount` field when promotions or savings were
applied. This means item prices may sum to more than the order total. The bank
charge is always correct — use it as the transaction total and adjust the tax
line to absorb the difference. Never adjust item prices.

## Recurring Charges

Some merchants have consistent memo patterns across months. Check
`transactions.json` for the exact memo text used previously and replicate it.

## Flagging Issues

If the CSV contains charges that look anomalous, flag them in chat before
creating the canonical JSON rather than embedding notes in memo fields.
Examples:
- Duplicate charges (same merchant, same amount, same date appearing twice)
- Unexpected double billing from a merchant

Flags are temporary. Once the user resolves an item, fold the answer into its
Memo and Account. The delivered JSON must account for every statement row and
must not contain a resolved-questions section.

## Existing Imbalance Corrections

When `transactions.json` contains transactions with `Imbalance-USD` splits,
check whether order summary data can now categorize them. Report these
separately in chat so the user can update them in GnuCash.

## Rendered Markdown Examples

`gnucash-transaction-create` renders the neutral JSON into this review format.
It puts the source split last and shows Date and Description only on the first
row. All account paths below are illustrative. Use only paths from
`accounts.json`.

Simple single-item order:

| Date | Description | Memo | Account | Amount |
|------|-------------|------|---------|--------|
| 2026-03-23 | Amazon | Book title | Expenses:Books | $15.99 |
|  |  |  | Expenses:Taxes:Sales Tax | $1.28 |
|  |  |  | Liabilities:Credit Card | -$17.27 |

Multi-item order with items in different expense accounts:

| Date | Description | Memo | Account | Amount |
|------|-------------|------|---------|--------|
| 2026-02-18 | Walmart | Pest traps, shower caddy | Expenses:Household Supplies | $41.41 |
|  |  | Winter boots | Expenses:Clothing | $34.99 |
|  |  |  | Expenses:Taxes:Sales Tax | $6.11 |
|  |  |  | Liabilities:Credit Card | -$82.51 |

Multi-item order where all items go to the same account (tax merged in):

| Date | Description | Memo | Account | Amount |
|------|-------------|------|---------|--------|
| 2026-03-18 | Amazon Fresh | Eggs, milk, bread, bananas, cereal, yogurt | Expenses:Food:Groceries Delivered | $158.57 |
|  |  |  | Liabilities:Credit Card | -$158.57 |

Refund (always a separate transaction, signs reversed):

| Date | Description | Memo | Account | Amount |
|------|-------------|------|---------|--------|
| 2026-03-14 | Amazon |  | Liabilities:Credit Card | $57.22 |
|  |  | Refund: Jeans | Expenses:Clothing | -$52.98 |
|  |  |  | Expenses:Taxes:Sales Tax | -$4.24 |

Walmart split shipment (charge cannot be matched to specific items):

| Date | Description | Memo | Account | Amount |
|------|-------------|------|---------|--------|
| 2026-03-24 | Walmart | Order XXXXXXXX partial shipment | Imbalance-USD | $109.33 |
|  |  |  | Liabilities:Credit Card | -$109.33 |

The renderer separates transaction tables with a blank line and preserves input
transaction order. Group the canonical JSON transactions by date.

## Creating Review Markdown and GnuCash CSV

After all flags are resolved, validate the canonical JSON and create both files:

```bash
gnucash-transaction-create \
  --input <transactions.json> \
  --markdown-output <transactions.md> \
  --csv-output <transactions.csv> \
  --expected-transactions <count>
```

The renderer has no built-in paths, account names, dates, totals, or expected
transaction counts. It validates the neutral data once, then independently
creates each output. It:

- rejects unknown JSON fields and malformed amounts;
- requires exactly one source split per transaction;
- requires every transaction to sum exactly to zero;
- puts the source split last in Markdown and first in CSV;
- supports configurable currency symbols, decimal separators, CSV delimiters,
  decimal precision, transaction-ID prefix, and expected transaction count;
- writes GnuCash's current 18-column transaction-export layout, leaving ignored
  fields blank and using numeric Amount and Value columns for single-currency
  transactions;
- adds import-only grouping keys so adjacent transactions with the same date and
  description remain separate; these keys are not stored as transaction GUIDs;
- refuses to overwrite either output unless `--force` is supplied.

Without explicit output paths, it creates `<input-stem>.md` and
`<input-stem>_gnucash.csv` beside the JSON file.

Review the Markdown, apply any corrections to the canonical JSON, then rerun
the renderer so both generated files stay synchronized. Do not use either
generated file as the source for the other.

For GnuCash, select **File → Import → Import Transactions from CSV**, choose the
built-in **GnuCash Export Format** settings, and leave the global Account blank.
This preset enables multi-split mode, skips the one-line header, and maps the 18
columns by position. Select date and currency formats matching the file, review
every account mapping and possible duplicate, then apply the import.
