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
        json: {
          status: "reauthenticated",
          profile: null,
          google_authenticated_until: Math.floor(Date.now() / 1000) + 300,
        },
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
  await expect(
    page.getByText(
      "Confirm your identity with Google to enable password setup",
      {
        exact: false,
      },
    ),
  ).toBeVisible();
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

test("fresh Google sign-in enables password setup without a second Google confirmation", async ({
  page,
}) => {
  await googleScript(page);
  let signedIn = false;
  const until = Math.floor(Date.now() / 1000) + 300;
  let credentialSubmissions = 0;
  await page.route("**/api/account/**", async (route) => {
    const path = new URL(route.request().url()).pathname;
    if (path.endsWith("/me"))
      return signedIn
        ? route.fulfill({
            json: { ...profile, google_authenticated_until: until },
          })
        : route.fulfill({
            status: 401,
            json: {
              error: {
                code: "customer_authentication_required",
                message: "Sign in",
              },
            },
          });
    if (path.endsWith("/orders"))
      return route.fulfill({ json: { orders: [], total: 0, page: 1 } });
    if (path.endsWith("/google/config"))
      return route.fulfill({
        json: { client_id: "test.apps.googleusercontent.com" },
      });
    if (path.endsWith("/google/challenge")) {
      return route.fulfill({ json: { id: "challenge", nonce: "nonce" } });
    }
    if (path.endsWith("/google")) {
      credentialSubmissions += 1;
      signedIn = true;
      return route.fulfill({
        json: {
          status: "authenticated",
          profile,
          google_authenticated_until: until,
        },
      });
    }
    if (path.endsWith("/password")) {
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
  await page.getByRole("button", { name: "Continue with Google" }).click();
  await expect(
    page.getByRole("button", { name: "Set password", exact: true }),
  ).toBeEnabled();
  await page.reload();
  await expect(
    page.getByRole("button", { name: "Set password", exact: true }),
  ).toBeEnabled();
  await expect(
    page.getByText("Use 12–256 characters.", { exact: false }),
  ).toBeVisible();
  await expect(
    page.getByLabel("New password", { exact: true }),
  ).toHaveAttribute("aria-describedby", "profile-password-requirements");
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
  expect(credentialSubmissions).toBe(1);
});

test("Google password confirmation expires and explains how to confirm again", async ({
  page,
}) => {
  await page.clock.install();
  await googleScript(page);
  await page.route("**/api/account/**", async (route) => {
    const path = new URL(route.request().url()).pathname;
    if (path.endsWith("/me")) return route.fulfill({ json: profile });
    if (path.endsWith("/orders"))
      return route.fulfill({ json: { orders: [], total: 0, page: 1 } });
    if (path.endsWith("/google/config"))
      return route.fulfill({
        json: { client_id: "test.apps.googleusercontent.com" },
      });
    if (path.endsWith("/google/challenge"))
      return route.fulfill({ json: { id: "challenge", nonce: "nonce" } });
    if (path.endsWith("/google"))
      return route.fulfill({
        json: {
          status: "reauthenticated",
          profile: null,
          google_authenticated_until: Math.floor(Date.now() / 1000) + 300,
        },
      });
    return route.fulfill({ status: 404 });
  });
  await page.goto("/account");
  await page.getByRole("button", { name: "Continue with Google" }).click();
  await expect(
    page.getByRole("button", { name: "Set password", exact: true }),
  ).toBeEnabled();
  await page.clock.fastForward(301000);
  await expect(
    page.getByRole("button", { name: "Set password", exact: true }),
  ).toBeDisabled();
  await expect(
    page.getByText(
      "Confirm your identity with Google to enable password setup",
      {
        exact: false,
      },
    ),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Continue with Google", exact: true }),
  ).toBeVisible();
});

test("password requirements are visible during registration and recovery", async ({
  page,
}) => {
  await page.route("**/api/account/**", async (route) => {
    if (new URL(route.request().url()).pathname.endsWith("/google/config"))
      return route.fulfill({ json: { client_id: null } });
    return route.fulfill({
      status: 401,
      json: {
        error: { code: "customer_authentication_required", message: "Sign in" },
      },
    });
  });
  await page.goto("/account");
  await page
    .getByRole("button", { name: "Create account", exact: true })
    .click();
  await expect(
    page.getByText("Use 12–256 characters.", { exact: false }),
  ).toBeVisible();
  await expect(page.getByLabel("Password", { exact: true })).toHaveAttribute(
    "aria-describedby",
    "password-requirements",
  );
  await page.goto("/account?reset=example-token");
  await expect(
    page.getByText("Use 12–256 characters.", { exact: false }),
  ).toBeVisible();
  await expect(
    page.getByLabel("New password", { exact: true }),
  ).toHaveAttribute("aria-describedby", "password-requirements");
});

test("connecting Google updates the confirmation state of method controls", async ({
  page,
}) => {
  await googleScript(page);
  const passwordProfile = {
    ...profile,
    auth_methods: { password: true, google_email: null },
  };
  await page.route("**/api/account/**", async (route) => {
    const path = new URL(route.request().url()).pathname;
    if (path.endsWith("/me")) return route.fulfill({ json: passwordProfile });
    if (path.endsWith("/orders"))
      return route.fulfill({ json: { orders: [], total: 0, page: 1 } });
    if (path.endsWith("/reauthenticate")) return route.fulfill({ status: 204 });
    if (path.endsWith("/google/config"))
      return route.fulfill({
        json: { client_id: "test.apps.googleusercontent.com" },
      });
    if (path.endsWith("/google/challenge")) {
      expect(route.request().postDataJSON().purpose).toBe("link");
      return route.fulfill({ json: { id: "challenge", nonce: "nonce" } });
    }
    if (path.endsWith("/google"))
      return route.fulfill({
        json: {
          status: "authenticated",
          profile: {
            ...profile,
            auth_methods: { password: true, google_email: profile.email },
          },
          google_authenticated_until: Math.floor(Date.now() / 1000) + 300,
        },
      });
    return route.fulfill({ status: 404 });
  });
  await page.goto("/account");
  const security = page.getByRole("region", {
    name: "Sign-in methods",
    exact: true,
  });
  await security
    .getByLabel("Current password", { exact: true })
    .fill("existing-account-password");
  await security.getByRole("button", { name: "Confirm", exact: true }).click();
  await security
    .getByRole("button", { name: "Continue with Google", exact: true })
    .click();
  await expect(
    security.getByRole("button", { name: "Disconnect Google", exact: true }),
  ).toBeDisabled();
  await expect(
    security.getByRole("button", {
      name: "Remove password login",
      exact: true,
    }),
  ).toBeEnabled();
});

test("Google sign-in refreshes favorites and completes a pending save", async ({ page }) => {
  await googleScript(page);
  const productId = "00000000-0000-0000-0000-000000000002";
  let signedIn = false;
  let saved = false;
  let favoritesRefreshed = false;
  await page.addInitScript((id) => {
    sessionStorage.setItem("knitnprint-pending-favorite", id);
  }, productId);
  await page.route("**/api/account/**", async (route) => {
    const path = new URL(route.request().url()).pathname;
    if (path.endsWith("/google/config"))
      return route.fulfill({ json: { client_id: "test.apps.googleusercontent.com" } });
    if (path.endsWith("/google/challenge"))
      return route.fulfill({ json: { id: "challenge", nonce: "nonce" } });
    if (path.endsWith("/google")) {
      signedIn = true;
      return route.fulfill({ json: { status: "authenticated", profile } });
    }
    if (path.endsWith(`/favorites/${productId}`)) {
      expect(route.request().method()).toBe("PUT");
      expect(signedIn).toBe(true);
      saved = true;
      return route.fulfill({ status: 204 });
    }
    if (path.endsWith("/favorites") && signedIn) {
      favoritesRefreshed = true;
      return route.fulfill({ json: [] });
    }
    if (path.endsWith("/orders"))
      return route.fulfill({ json: { orders: [], total: 0, page: 1 } });
    return route.fulfill({
      status: 401,
      json: { error: { code: "customer_authentication_required", message: "Sign in" } },
    });
  });
  await page.goto("/account#favorites");
  await page.getByRole("button", { name: "Continue with Google" }).click();
  await expect(page.getByRole("heading", { name: "Sign-in methods" })).toBeVisible();
  await expect.poll(() => saved).toBe(true);
  expect(favoritesRefreshed).toBe(true);
  await expect.poll(() => page.evaluate(() => sessionStorage.getItem("knitnprint-pending-favorite"))).toBeNull();
  await expect(page.locator(".account-favorites .favorites-empty .button")).toHaveAttribute("href", "/products");
});
