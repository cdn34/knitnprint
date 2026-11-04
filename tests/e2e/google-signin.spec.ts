import { expect, test, type Page } from "@playwright/test";
const profile = {
  id: "00000000-0000-0000-0000-000000000001",
  email: "marta@gmail.com",
  first_name: "Marta",
  last_name: "Silva",
  phone: "",
  email_verified: true,
  addresses: [],
  auth_methods: { password: false, google_email: "marta@gmail.com" },
};
async function googleScript(page: Page) {
  await page.route("https://accounts.google.com/gsi/client", (route) =>
    route.fulfill({
      contentType: "application/javascript",
      body: `let options; window.google={accounts:{id:{initialize(value){options=value},renderButton(element){const button=document.createElement('button');button.textContent='Continue with Google';button.onclick=()=>options.callback({credential:'test-only-credential'});element.append(button)},disableAutoSelect(){},revoke(email,callback){callback()}}}};`,
    }),
  );
}
test("Google sign-in confirms ownership and preserves the customer profile", async ({
  page,
}) => {
  await googleScript(page);
  await page.route("**/api/account/**", async (route) => {
    const path = new URL(route.request().url()).pathname;
    if (path.endsWith("/google/config"))
      return route.fulfill({
        json: { client_id: "test.apps.googleusercontent.com" },
      });
    if (path.endsWith("/google/challenge"))
      return route.fulfill({ json: { id: "challenge", nonce: "nonce" } });
    if (path.endsWith("/google"))
      return route.fulfill({
        json: { status: "confirmation_required", profile: null },
      });
    if (path.endsWith("/google/confirm")) {
      expect(route.request().postDataJSON().password).toBe(
        "existing-account-password",
      );
      return route.fulfill({ json: { status: "authenticated", profile } });
    }
    if (path.endsWith("/orders"))
      return route.fulfill({ json: { orders: [], total: 0, page: 1 } });
    return route.fulfill({
      status: 401,
      json: {
        error: { code: "customer_authentication_required", message: "Sign in" },
      },
    });
  });
  await page.goto("/account");
  await page.getByRole("button", { name: "Continue with Google" }).click();
  await expect(
    page.getByText("Check your email for a confirmation link", {
      exact: false,
    }),
  ).toBeVisible();
  await page
    .getByLabel("Current password", { exact: true })
    .fill("existing-account-password");
  await page.getByRole("button", { name: "Confirm", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "Welcome, Marta." }),
  ).toBeVisible();
  await expect(
    page.getByRole("heading", { name: "Sign-in methods" }),
  ).toBeVisible();
  await expect(page.getByLabel("New email address")).toHaveCount(0);
});
test("Google-only customer proves identity before adding a password", async ({
  page,
}) => {
  await googleScript(page);
  let changed = false;
  await page.route("**/api/account/**", async (route) => {
    const path = new URL(route.request().url()).pathname;
    if (path.endsWith("/me")) return route.fulfill({ json: profile });
    if (path.endsWith("/orders"))
      return route.fulfill({ json: { orders: [], total: 0, page: 1 } });
    if (path.endsWith("/google/config"))
      return route.fulfill({
        json: { client_id: "test.apps.googleusercontent.com" },
      });
    if (path.endsWith("/google/challenge")) {
      expect(route.request().postDataJSON().purpose).toBe("reauthenticate");
      return route.fulfill({ json: { id: "challenge", nonce: "nonce" } });
    }
    if (path.endsWith("/google"))
      return route.fulfill({
        json: { status: "reauthenticated", profile: null },
      });
    if (path.endsWith("/password")) {
      changed = true;
      expect(route.request().postDataJSON().password).toBe(
        "new-account-password",
      );
      return route.fulfill({
        json: {
          ...profile,
          auth_methods: { ...profile.auth_methods, password: true },
        },
      });
    }
    return route.fulfill({ status: 404 });
  });
  await page.goto("/account");
  await expect(
    page.getByRole("button", { name: "Set password", exact: true }),
  ).toBeDisabled();
  await page.getByRole("button", { name: "Continue with Google" }).click();
  await page
    .getByLabel("New password", { exact: true })
    .fill("new-account-password");
  await page
    .getByLabel("Confirm password", { exact: true })
    .fill("new-account-password");
  await page.getByRole("button", { name: "Set password", exact: true }).click();
  await expect(
    page.getByText("Sign-in settings updated.", { exact: true }),
  ).toBeVisible();
  expect(changed).toBe(true);
  await expect(
    page.getByRole("button", { name: "Disconnect Google" }),
  ).toBeDisabled();
});
