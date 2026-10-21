# Order confirmation email — approved design

Status: implemented in the live confirmation renderer and notification worker.
The HTML files below remain the approved visual reference.

Open [the standalone preview](order-confirmation-email.html) to review the design.
The [editable fragment](order-confirmation-email.fragment.html) is the source of
the preview and includes Portuguese, English and Spanish copy. Customer details,
products, prices and personalization shown in the prototype are illustrative.

## Agreed appearance and content

- Use the KnitnPrint wordmark, plum text and accents, and the existing beige
  background behind both the logo and the order confirmation heading.
- Do not display an order progress bar.
- Include the order number and date, product titles, selected variants and sizes,
  quantities, personalization details, line prices, subtotal, discount, delivery
  cost and total paid.
- Show the final personalized product preview for each order line, including
  each customized face when applicable. The production email must use previews
  saved with that order, matching the customer's submitted design, positioning
  and sizing. A generic catalog photo is not a substitute for that confirmation.
- Under Payment, display only the confirmed status; omit the payment method.
- Include the delivery address, a link to the customer's order and the support
  address `support@knitnprint.com`.
- Explain that shipping and delivery updates will come directly from the carrier.
  Do not promise additional KnitnPrint shipping emails in this confirmation.

## Live implementation

`backend/src/order_email.rs` renders email-client-compatible tables and inline
styles from immutable order snapshots. `admin:deliver-notifications` sends that
HTML and a plain-text alternative through SES. The logo and personalized faces
are embedded as inline PNG attachments, so no private originals become public
and image URLs cannot expire. The worker needs the same S3 configuration/read
access as the API when an order has saved previews.

Checkout saves the selected storefront language (`en`, `pt`, or `es`), variant
options, and references to flattened preview images captured from each customized
face. The capture uses the same markup, geometry, photo cropping, and text as the
final preview. Order retention protects these images independently of later
catalog changes and cart cleanup.

Apply migrations with `npm run db:migrate` before running the updated API and
worker. Historical orders default to English. If an earlier personalized order
has no saved flattened preview, its email includes the personalization details
and an explicit unavailable-preview note; it never substitutes a catalog photo.
Already sent notifications are not automatically resent.

Store fulfillment emails are no longer enqueued. The migration marks outstanding
fulfillment jobs as `suppressed`, without claiming they were delivered. Carrier
shipping and delivery updates remain the carrier's responsibility.

The standalone design document uses browser-only layout and illustrative garment
drawings. Those drawings are never used as customer order assets.
