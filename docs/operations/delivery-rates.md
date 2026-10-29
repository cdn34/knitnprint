# Destination shipping rates

The admin **Shipping rates** workspace requires `settings.manage`. It supports
editable countries, region labels, comma-separated postal prefixes, one flat
charge per order, availability, adding/removing destinations, and testing saved
rules without purchasing. Prices use integer minor units; blank is unconfigured,
while zero explicitly means free shipping. Saving requires an audit reason and
the current revision, preventing one administrator from overwriting another.

The main table shows Country, optional Region, Price and Active. New destinations
default to **All regions** and cover their whole country: choose the country,
enter a price and activate the row. A blank region label is saved as All regions.
Postal prefixes live under each row's collapsed **Advanced configuration**, only
needed for regional exceptions. Opening or closing those controls never changes
the saved prefixes. Existing Portuguese regional rules remain intact. The region
label by itself does not restrict a country-wide rule's coverage.

## Initial setup

Migration 0054 creates four inactive, unpriced destinations and leaves the table
**off**, preserving the existing zero-charge checkout until staff opt in:

| Country | Region | Postal prefixes |
| --- | --- | --- |
| PT | Mainland Portugal | 1–8 |
| PT | Madeira, including Porto Santo | 90–94 |
| PT | Azores | 95–99 |
| ES | All regions | Any valid Spanish postcode |

Enter prices, activate the destinations served, enable **Use these rates at
checkout**, and save. Enabling needs at least one priced active destination.
Spain initially includes all Spanish regions, not just the peninsula. Add more
specific prefix rules if islands or other territories need separate charges.

## Matching and checkout

- The server derives the destination from the delivery country and postcode,
  never from a customer-selected shipping region or the free-text address region.
- Customer account and checkout addresses use a country dropdown with localized
  names (English, Portuguese or Spanish); submissions retain ISO two-letter codes.
- Postcodes are normalized for matching. PT requires seven digits; ES requires
  five digits and a province prefix from 01 through 52.
- The longest matching prefix wins over broader prefixes and country-wide rules.
  Duplicate country/prefix assignments are rejected. An inactive specific rule
  blocks that destination instead of falling back to a cheaper general rule.
- With the table enabled, missing/inactive/unpriced destinations, invalid
  postcodes and currency mismatches block checkout. They never become free.
- Cart totals include shipping and any configured exclusive tax, including tax
  on shipping. Checkout re-evaluates everything in its database transaction.
  Clients send the cart's `checkout_quote` and the shipping quote's
  `shipping_rate_revision`. The checkout quote identifies the saved address and
  calculated prices. A change in another tab, delivery rates or totals returns
  `cart_changed`; the storefront reloads the address and total for review before
  another attempt. Successful order retries remain idempotent.
- EUR is the only supported currency. Store settings, product variants, discounts
  and delivery rates cannot be configured in another currency. Existing records
  are not converted or relabelled; non-EUR products cannot enter checkout.
- Order shipping snapshots keep the chosen destination, rule ID, currency and
  amount. Editing or deleting a rule cannot change an earlier order or email.

The destination tester uses saved rules even while the table is off, clearly
labelled as a simulation. It cannot activate pricing or create orders.

## Deployment compatibility

Migration 0053 retains the retired shipping tables and product/cart columns so
the previous API can continue serving traffic during a migration-first rollout.
The new API no longer uses them. Their removal requires a later release after
all previous API tasks have been replaced. Migration 0055 enforces EUR on new
configuration and catalog writes without changing existing stored amounts.

## Postal references

Seeded Portuguese island prefixes follow [CTT delivery coverage](https://www.ctt.pt/application/themes/pdfs/prazos_de_entrega_expresso_entre_empresas_envios_para_amanha_10h_13h_19h_22h.pdf).
Postcode formats follow the Universal Postal Union's addressing specifications
for [Portugal](https://www.upu.int/UPU/media/upu/PostalEntitiesFiles/addressingUnit/prtEn.pdf)
and [Spain](https://www.upu.int/UPU/media/upu/PostalEntitiesFiles/addressingUnit/espEn.pdf).
These are region/format checks, not validation of an individual street address.
