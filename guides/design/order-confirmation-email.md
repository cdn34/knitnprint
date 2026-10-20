# Order confirmation email — approved design

Status: approved visual proposal. This prototype is not connected to the live
transactional email renderer or notification worker.

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

## Implementation boundary

The garment drawings in the prototype illustrate the intended personalized
thumbnails; they are not customer order assets. Connecting saved order previews,
order data and localized copy to the live email is separate implementation work.
The carrier notification preference above also requires a separate change to the
live notification flow; this design commit does not change delivery behavior.

The standalone document is a browser preview. Its CSS layout, scripts and embedded
WebP logo are not a production email template. The live version must use
email-client-compatible markup and image delivery while preserving this design.
