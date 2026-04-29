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
   with names/prices, subtotal, tax, shipping, total, refund).
2. **`walmart_orders_summary.json`** — Walmart order data (same schema as Amazon).
3. **`accounts.json`** — All GnuCash account paths grouped by category (assets,
   expenses, income, liabilities, equity). Only use account paths from this file.
4. **`transactions.json`** — Recent GnuCash transactions with date, description,
   and splits (account, amount, memo). Use these as categorization precedent.

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

## Transaction Output Format

Write the output to a file at `~/Downloads/new_transactions_<YYYY-MM-DD>.md`,
where the date is today's date (the date the file is generated). Do not print
the tables inline in chat — the file is the deliverable.

Output each transaction as a markdown table. Each transaction is a separate table.

**Structure:**

| Date | Description | Account | Amount | Memo |
|------|-------------|---------|--------|------|
| _post date_ | _vendor_ | _account path_ | _signed $_ | _item description_ |
| | | _account path_ | _signed $_ | _item description_ |
| | | ... | ... | ... |

**Rules:**

- **Date** — use the order date from the JSON. If the user provides a credit
  card statement date that differs, prefer the statement date.
- Date and Description appear only on the first row of each transaction.
- Every item gets its own row with the correct expense account and a Memo.
- Sales tax gets its own row (`Expenses:Taxes:Sales Tax`) only when a
  transaction has items going to different expense accounts. If all items go
  to the same account, include tax in that single row's amount.
- The credit card (or bank account) row goes last with the negative total.
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

If the CSV contains charges that look anomalous, flag them at the top of the
output rather than embedding notes in memo fields. Examples:
- Duplicate charges (same merchant, same amount, same date appearing twice)
- Unexpected double billing from a merchant

## Existing Imbalance Corrections

When `transactions.json` contains transactions with `Imbalance-USD` splits,
check whether order summary data can now categorize them. Present these as a
separate "Corrections" section so the user can update them in GnuCash.

## Examples

All account paths below are illustrative. Use only paths from `accounts.json`.

Simple single-item order:

| Date | Description | Account | Amount | Memo |
|------|-------------|---------|--------|------|
| 2026-03-23 | Amazon | Expenses:Books | $15.99 | Book title |
|  |  | Expenses:Taxes:Sales Tax | $1.28 |  |
|  |  | Liabilities:Credit Card | -$17.27 |  |

Multi-item order with items in different expense accounts:

| Date | Description | Account | Amount | Memo |
|------|-------------|---------|--------|------|
| 2026-02-18 | Walmart | Expenses:Household Supplies | $41.41 | Pest traps, shower caddy |
|  |  | Expenses:Clothing | $34.99 | Winter boots |
|  |  | Expenses:Taxes:Sales Tax | $6.11 |  |
|  |  | Liabilities:Credit Card | -$82.51 |  |

Multi-item order where all items go to the same account (tax merged in):

| Date | Description | Account | Amount | Memo |
|------|-------------|---------|--------|------|
| 2026-03-18 | Amazon Fresh | Expenses:Food:Groceries Delivered | $158.57 | Eggs, milk, bread, bananas, cereal, yogurt |
|  |  | Liabilities:Credit Card | -$158.57 |  |

Refund (always a separate transaction, signs reversed):

| Date | Description | Account | Amount | Memo |
|------|-------------|---------|--------|------|
| 2026-03-14 | Amazon | Liabilities:Credit Card | $57.22 |  |
|  |  | Expenses:Clothing | -$52.98 | Refund: Jeans |
|  |  | Expenses:Taxes:Sales Tax | -$4.24 |  |

Walmart split shipment (charge cannot be matched to specific items):

| Date | Description | Account | Amount | Memo |
|------|-------------|---------|--------|------|
| 2026-03-24 | Walmart | Imbalance-USD | $109.33 | Order XXXXXXXX partial shipment |
|  |  | Liabilities:Credit Card | -$109.33 |  |

When presenting multiple transactions, separate each table with a blank line.
Group by date.
