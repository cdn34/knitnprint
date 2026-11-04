# Product options and SKUs

The implementation plan is saved in [VARIANTS_PLAN.md](../../VARIANTS_PLAN.md).

Create a draft product, then open **Options and variants** in its editor. Choose
Color or a size preset, customize its copied values, and generate the matrix.
Map existing variants before generating if their stock and SKU should be kept.
Review combinations, exclude ones that are not sold, and set prices/stock before
saving. Unchecked new combinations are not created; unchecked saved variants
are retired. Refreshing the matrix preserves saved combinations and identities.

Products support up to six option axes and 100 active variants. The preview is
bounded to 1,000 combinations; reduce the chosen values before generating larger
matrices. A product can have one swatch axis. Other axes use buttons or dropdowns.
The template library supports additional choices such as capacity, material,
finish, and pack quantity. Edits to templates affect future copies only.

Product and variant SKUs are allocated by the server and cannot be edited.
Product numbers are permanent; variant suffixes use permanent option codes in
alphabetical axis-code order. Names, display order, swatches, and prices can
change without changing SKUs. Actual option identities cannot change after a
variant is mapped: retire the old combination and generate a new one. Existing
SKUs survive migration and one-time mapping. SKU reservations are never reused.
Legacy create clients may still send a SKU; it is ignored in favor of automatic
allocation. Update clients may echo the unchanged SKU, but changing it returns
409. Explicit SQL imports can supply legacy SKUs, which are permanently reserved
on insertion as well.

**Use color-specific photos** is off by default. When enabled, select the existing
photos for each color. Unmapped colors use the normal shared gallery. Assigned
photos also select the personalization views; the server validates that those
views belong to the chosen variant. Existing measurements stay attached to the
same variant IDs. New combinations need their own calibration where required.
Deleting a photo clears its color assignments; historical order photos remain
protected by the existing retention rules.

Saved variants are retired rather than deleted. Existing carts containing a
retired variant must be corrected before checkout; historical orders remain
usable. Current EUR-only pricing and backorder behavior are preserved. Recorded
stock is independent per SKU, including pack variants; no component stock is
inferred from a pack quantity.

Matrix saves require the product revision and each existing variant's expected
stock quantity. Stale saves return 409 and must be reloaded. Template saves also
check their revision. Successful template and matrix changes create audit entries.

Migration 0059 is additive and preserves all previous migration files. Apply it
before deploying the updated API, then deploy the admin and storefront together.
Use `npm run check:local` against a disposable PostgreSQL database and run both
browser suites before pushing. The new browser coverage is in
`tests/admin-e2e/variants.spec.ts`; database coverage is in
`backend/tests/variant_matrix.rs`.
