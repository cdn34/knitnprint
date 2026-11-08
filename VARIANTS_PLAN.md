# Product variant templates and permanent SKUs

## Summary

Build on the existing variant-based pricing and inventory system. Administrators choose reusable option templates for each product, customize their values, and generate sellable combinations. Each product gets a permanent parent SKU; each combination gets a permanent variant SKU. Color-specific photos are opt-in, with shared photos as fallback.

## Templates and product editing

- Add a separate Variant templates section in the admin navigation using existing catalog permissions; product editing only applies and customizes template copies.
- Seed Color, Adult size (XS–XXL), Baby size in months (0–6, 6–12, 12–18, 18–24), and Children's size in years (1–2, 2–4, 5–6). All defaults are configurable.
- Provide color swatches with text labels, choice buttons, and dropdowns. Administrators can create templates such as capacity, material, finish, and pack quantity without code changes.
- Applying a template copies it into a product. Product values, labels, swatches, and ordering can change independently; template edits affect future copies only.
- Preview generated combinations before saving; support exclusions, bulk price/stock changes, and individual adjustments.
- Multi-variant products edit price and stock per combination rather than silently modifying the first variant.
- Limit active combinations to 100 per product with visible validation. New variants start with zero recorded stock; ordering regardless of stock remains supported.

## SKU and variant identity

- Generate parent codes on the server, e.g. KP-000123; generate variant codes from permanent option codes, e.g. KP-000123-COL-RED-SIZ-M. A product without options has a default variant ending in STD.
- Allocate parent numbers through a database sequence. Names, slugs, prices, labels, and display order never regenerate SKUs.
- Reserve parent and variant codes in a shared registry; enforce uniqueness during concurrent creation and never reuse retired codes. Resolve allocation collisions, including existing legacy codes.
- Enforce SKU immutability through API and database guards. Display SKUs read-only in the admin.
- Include the permanent parent SKU in canonical product URLs, preserve historical URLs for the original product, and never reassign deleted-product URLs.
- Matrix refreshes preserve existing variant IDs, SKUs, stock history, and personalization measurements.
- Real option identity changes require a new combination/SKU. Retire rather than delete old combinations; reactivation retains their identity. Display labels and swatches remain editable.
- Backfill parent codes for existing products and preserve all existing variant SKUs. Offer explicit one-time mapping of legacy variants to structured options without replacing IDs or histories; mapped identities become immutable.

## Storefront and optional photos

- Render one selector per option and resolve choices to an existing sellable variant. Excluded or retired combinations cannot be bought; stock alone does not disable purchasing.
- Share selection behavior between product and personalization pages and preserve selection during navigation.
- Add a product-level Use color-specific photos toggle, off by default. Assign existing product photos to colors optionally.
- Mapped colors use assigned photos and corresponding personalization views; unmapped colors use shared photos/views. Do not recolor images automatically.
- Size changes preserve artwork, revalidate measurements, and regenerate previews. Ask before clearing affected artwork when a color change replaces photographed views.
- Validate photos and personalization views on the server and snapshot the resolved variant context into orders.
- Preserve EUR-only pricing and checkout review protections.

## Data, API, and compatibility

- Append migrations after 0058 without rewriting existing migrations.
- Persist templates, copied product options/values, immutable selections and combination identity, retirement status, and optional photo assignments.
- Extend product responses with parent SKU, structured options/selections, and photo configuration. Keep Variant.sku and readable option_values for order snapshots/emails.
- Add catalog APIs for templates, matrix preview/save, legacy mapping, pricing, retirement/reactivation, and photo assignments. Save matrices transactionally and reject stale edits.
- Remove writable SKU controls/contracts, reject explicit overrides, and regenerate OpenAPI/TypeScript clients.
- Existing products keep their picker until explicitly mapped. Retirement blocks existing-cart checkout without impairing historical orders or fulfillment.

## Validation and defaults

- Test copied defaults, overrides, custom values, exclusions, duplicates, and repeat saves.
- Verify concurrent allocation, database immutability, rename/reorder stability, and legacy stock/calibration preservation.
- Test opt-in photos, partial mappings/fallback, invalid references, artwork changes, and order/email snapshots.
- Cover desktop/mobile selection, keyboard access, labeled swatches, unavailable combinations, backorders, and retirement in carts.
- Run local checks and both browser suites, including existing currency, checkout, notification, and personalization regressions.

Defaults: copied templates; opt-in photos; preserved legacy SKUs; personalization text is not a stock variant. Pack quantities are independently stocked sellable packs, without component-inventory calculations.
