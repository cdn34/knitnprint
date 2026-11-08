# Product options and SKUs

The implementation plan is saved in [VARIANTS_PLAN.md](../../VARIANTS_PLAN.md).

Use **Variant templates** in the admin navigation to create, edit, or delete
shared defaults independently of products. Deleting a template does not remove
options or variants already copied into products.

Create a draft product, then open **Tamanhos e cores** in its editor.
The workspace follows three numbered sections: product options, photographs,
and generated combinations. Enable **Tem tamanhos** and **Tem cores**, set the
base price, and choose the size and color chips. The quick-add fields accept
custom sizes and colors. **Modelos e outras opções** retains the reusable
templates, custom option types, and name/value editors. Generate combinations
after changing options; for example, two colors and three sizes produce six variants.
Map existing variants before generating if their stock and SKU should be kept.
Review combinations, exclude ones that are not sold, and set prices/stock before
saving. Unchecked new combinations are not created; unchecked saved variants
are retired. Refreshing the matrix preserves saved combinations and identities.
The table groups option labels in one **Variante** column. Stock remains editable
in each row; enable **Definir preços diferentes por variante** to edit individual
prices. Hiding these price fields preserves the recorded prices. Under
**Opções avançadas das combinações**, use **Aplicar preço ou stock em lote** for
bulk fields, **Mostrar referências** for permanent SKUs, and
**Mostrar variantes desativadas** to reactivate saved combinations. The
**Por encomenda** state identifies active variants with no positive recorded
stock; it does not disable purchasing.

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

**Usar fotografias específicas por cor (opcional)** is off by default. Each color
card previews its assigned photographs and identifies shared-gallery fallback
when no assignment is active. Open **Adicionar fotos** or **Alterar fotos** to
select existing product photos for a color; opening a chooser enables the optional
color galleries. **Opções da galeria** also exposes the opt-in switch. Unmapped
colors use the normal shared gallery. Assigned
photos also select the personalization views; the server validates that those
views belong to the chosen variant. Existing measurements stay attached to the
same variant IDs. New combinations need their own calibration where required.
Deleting a photo clears its color assignments; historical order photos remain
protected by the existing retention rules.

**Como fica na loja** previews draft selections, photographs, prices, and recorded
stock before saving. The public product page uses the same photo/selection layout,
with color swatches first, size buttons, and a compact availability badge. The
public API masks stock quantities, so the public badge does not present its
placeholder quantity as real inventory. Personalization links and cart actions
continue to use the selected variant.

Product URLs include the parent SKU in lowercase, for example
`/products/linen-shirt-kp-000123`. The editor changes only the readable name;
the server adds the permanent parent identifier, never a variant SKU. Existing
products receive canonical URLs through migration 0060. Previous URLs redirect
to the current URL of their original product, including personalization links
and their query parameters. Deleting a product permanently reserves its previous
URLs, so a same-name replacement cannot inherit its links.

Saved variants are retired rather than deleted. Existing carts containing a
retired variant must be corrected before checkout; historical orders remain
usable. Current EUR-only pricing and backorder behavior are preserved. Recorded
stock is independent per SKU, including pack variants; no component stock is
inferred from a pack quantity.

Matrix saves require the product revision and each existing variant's expected
stock quantity. Stale saves return 409 and must be reloaded. Template saves also
check their revision. Successful template and matrix changes create audit entries.

Migrations 0059 and 0060 preserve all previous migration files. Apply them
before deploying the updated API, then deploy the admin and storefront together.
Use `npm run check:local` against a disposable PostgreSQL database and run both
browser suites before pushing. The new browser coverage is in
`tests/admin-e2e/variants.spec.ts`; database coverage is in
`backend/tests/variant_matrix.rs`.
