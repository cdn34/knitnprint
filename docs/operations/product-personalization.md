# Personalization permissions per print area

In the product editor, select a photograph and then a print-area tab. Below the
area name, **Personalização permitida nesta área** offers **Só texto**, **Só imagem**,
and **Texto e imagem**. Save the product to publish the selection. Each area's
name, position, dimensions, and content permissions are independent.

For a backpack with three print areas, the main area can accept text and images
while the top and bottom areas accept only text. The public personalizer shows
only the permitted tools for the selected area. Its canvas, final preview,
uploaded-media references, and cart payload respect each area's permissions.
Text and images are both optional when both are permitted.

Existing areas without an explicit `mode` inherit the photograph's mode, or the
product mode for legacy configurations without photographs. Changing one area's
mode does not change the others. Saving an enabled photograph also records the
effective permissions of its inherited areas. **Sem personalização** on a photograph still
disables all of its areas, including areas with explicit permissions. The
photograph's enabled mode remains the initial value for areas without overrides.

Permissions are stored as the optional `mode` field inside each `print_areas`
entry (`text`, `photo`, or `photo_text`). The existing JSON columns hold the new
field, so no database migration is needed. Product-wide availability is derived
from the enabled areas. The cart validates the saved permissions server-side,
rejects disallowed content, and prevents older payload formats from bypassing
area or photograph restrictions. Existing order snapshots remain unchanged.

## Integration with favorites and pricing

The favorites migration is `0062_customer_favorites.sql`. It was renumbered from
`0059_customer_favorites.sql` when integrating the favorites branch, because
`0059_variant_templates_and_skus.sql` already exists. Existing migrations and their
checksums are unchanged. Apply pending migrations normally; do not reset the database.

Each filled area adds EUR 1 per product unit, regardless of whether it contains
text, a photo, or both. Area permissions continue to be enforced independently
when calculating and submitting the design.
