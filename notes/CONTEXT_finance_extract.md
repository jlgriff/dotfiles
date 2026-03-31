# Finance Extract — AI Transaction Entry Guide

Use this guide along with the four JSON files produced by the finance-extract
tools to generate new GnuCash transactions.

## Input Files

1. **`amazon_orders_summary.json`** — Amazon order data (order ID, date, items
   with names/prices, subtotal, tax, shipping, total, refund).
2. **`walmart_orders_summary.json`** — Walmart order data (same schema as Amazon).
3. **`accounts.json`** — All GnuCash account paths grouped by category (assets,
   expenses, income, liabilities, equity). Only use account paths from this file.
4. **`transactions.json`** — Recent GnuCash transactions with date, description,
   and splits (account, amount, memo). Use these as categorization precedent.

## Deduplication

Before generating a transaction for an order, check `transactions.json` for an
existing entry with a matching description (e.g. "Amazon", "Walmart") and a
similar total on a nearby date. Skip any order that appears to already be entered.

## Transaction Output Format

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
  used, use that. Otherwise, check `transactions.json` for the card most
  commonly associated with the merchant.
- **When unsure about which expense account an item belongs to, use
  `Imbalance-USD`.** A wrong guess is harder to catch than an explicit
  Imbalance entry. The user will correct these manually.

### Description

Use the merchant name (e.g. "Amazon", "Walmart", "Amazon Fresh").

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

When presenting multiple transactions, separate each table with a blank line.
Group by date.
