# Order confirmation email — approved design

Status: approved design, implemented in `backend/src/order_email.rs` and connected
to the transactional notification worker. These preview files remain illustrative.

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

## Production implementation

The garment drawings in the prototype are illustrative. New personalized cart
items save a PNG of each customized face using the same preview component. Uploads
are checked against the owning cart and retained with the immutable order media.
The worker attaches these privately through CID, never public upload URLs or
generic catalog photos. Older orders without saved previews retain their text
details and link to the order; the email does not invent a product preview.

PT/EN/ES subjects, body copy, dates and monetary formatting follow the immutable
checkout language, with English as the fallback. Payment confirmation and outbox
deduplication remain unchanged. Carrier updates replace store fulfillment notifications; pending fulfillment jobs
are suppressed. The confirmation does not show a progress bar.

The standalone document is a browser preview. The production version uses tables,
inline styles, system-font fallbacks, PNG attachments and a plain-text alternative
without scripts. The beige header, plum accents and information order match the
approved design. Shipping is shown as not charged while pricing is unconfigured.
