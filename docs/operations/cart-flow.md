# Add-to-cart destination and side panel

Both the product page and the personalization page use
`redirectToProductsWithCart` after a successful add. This navigates to the
product catalog and opens the cart on the right. The personalization flow saves
its design and preview before adding, including when the customer confirms a
partially completed design or adds from the final preview.

The drawer shows the current cart, personalization summaries, quantities, and
subtotal. **View cart and checkout** opens the existing checkout page.
**Continue shopping**, the close button, or Escape closes the panel and leaves
the catalog available. The header cart icon can reopen it.

The open-on-arrival URL marker is consumed once, so refreshing the catalog does
not reopen a dismissed panel. Adding failures keep the customer on the product
or personalization page. Retrying a failed cart read loads the existing cart
without posting another item. Signed-out customers receive a sign-in link.

Browser coverage lives in `tests/admin-e2e/cart-flow.spec.ts`; the area-permission
flow also verifies the personalized redirect and cart summary.
