import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

test("customer account persists addresses and supports returning sign in", async ({
  page,
}, testInfo) => {
  const unique = `${testInfo.project.name}-${Date.now()}-${Math.random().toString(36).slice(2)}`;
  const email = `account-${unique}@example.test`;
  const password = "thread-and-form-2026";

  await page.goto("/account");
  await expect(
    page.getByRole("heading", { name: "A home for your details." }),
  ).toBeVisible();

  await page.getByRole("button", { name: "Create account" }).click();
  await page.getByLabel("First name").fill("Marta");
  await page.getByLabel("Last name").fill("Silva");
  await page.getByLabel("Email address").fill(email);
  await page.getByLabel("Password").fill(password);
  await page.getByRole("button", { name: "Create account" }).last().click();

  await expect(
    page.getByRole("heading", { name: "Welcome, Marta." }),
  ).toBeVisible();
  await expect(page.getByText(email)).toBeVisible();

  await page.reload();
  await expect(
    page.getByRole("heading", { name: "Welcome, Marta." }),
  ).toBeVisible();

  await page.getByLabel("Recipient name").fill("Marta Silva");
  await page.getByLabel("Address line 1").fill("12 Rua das Flores");
  await page.getByLabel("City").fill("Porto");
  await page.getByLabel("Postal code").fill("4050-265");
  await page.getByLabel("Country code").fill("pt");
  await page.getByRole("button", { name: "Save address" }).click();

  await expect(page.getByRole("status")).toHaveText("Address saved.");
  const address = page
    .getByRole("article")
    .filter({ hasText: "12 Rua das Flores" });
  await expect(address).toContainText("4050-265 Porto");
  await expect(address).toContainText("PT");

  const results = await new AxeBuilder({ page })
    .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa"])
    .analyze();
  expect(results.violations).toEqual([]);

  await page.setViewportSize({ width: 390, height: 844 });
  const dimensions = await page.evaluate(() => ({
    viewport: document.documentElement.clientWidth,
    content: document.documentElement.scrollWidth,
  }));
  expect(dimensions.content).toBeLessThanOrEqual(dimensions.viewport);

  await page.getByRole("button", { name: "Sign out" }).click();
  await expect(
    page.getByRole("heading", { name: "A home for your details." }),
  ).toBeVisible();
  await page.getByLabel("Email address").fill(email);
  await page.getByLabel("Password").fill(password);
  await page.getByRole("button", { name: "Sign in" }).last().click();

  await expect(
    page.getByRole("heading", { name: "Welcome, Marta." }),
  ).toBeVisible();
  await expect(page.getByText("12 Rua das Flores")).toBeVisible();
});

test("customer verifies email and recovers a forgotten password", async ({
  page,
}, testInfo) => {
  const unique = `${testInfo.project.name}-${Date.now()}-${Math.random().toString(36).slice(2)}`;
  const email = `recovery-${unique}@example.test`;
  const oldPassword = "first-thread-passphrase";
  const newPassword = "second-thread-passphrase";

  await page.goto("/account");
  await page.getByRole("button", { name: "Create account" }).click();
  await page.getByLabel("First name").fill("Marta");
  await page.getByLabel("Last name").fill("Silva");
  await page.getByLabel("Email address").fill(email);
  await page.getByLabel("Password").fill(oldPassword);
  await page.getByRole("button", { name: "Create account" }).last().click();

  await expect(page.getByText("Email verification needed")).toBeVisible();
  const verification = await latestDevelopmentEmail(
    page,
    email,
    "email_verification",
  );
  await page.goto(verification.action_url);
  await expect(page).toHaveURL(/\/account$/);
  await expect(page.getByRole("status")).toHaveText(
    "Your email address is verified.",
  );
  await expect(page.getByText("Email address verified")).toBeVisible();

  await page.getByRole("button", { name: "Sign out" }).click();
  await page.getByRole("button", { name: "Forgot your password?" }).click();
  await expect(
    page.getByRole("heading", { name: "Reset your password" }),
  ).toBeVisible();
  await page.getByLabel("Email address").fill(email);
  await page.getByRole("button", { name: "Send reset link" }).click();
  await expect(page.getByRole("status")).toHaveText(
    "If an account exists for that address, a password-reset email is on its way.",
  );

  const reset = await latestDevelopmentEmail(page, email, "password_reset");
  await page.goto(reset.action_url);
  await expect(page).toHaveURL(/\/account$/);
  await expect(
    page.getByRole("heading", { name: "Choose a new password" }),
  ).toBeVisible();
  await page.getByLabel("New password", { exact: true }).fill(newPassword);
  await page.getByRole("button", { name: "Change password" }).click();
  await expect(page.getByRole("status")).toHaveText(
    "Your password has been changed. Sign in with the new password.",
  );
  await page.getByLabel("Email address").fill(email);
  await page.getByLabel("Password").fill(newPassword);
  await page.getByRole("button", { name: "Sign in" }).last().click();
  await expect(
    page.getByRole("heading", { name: "Welcome, Marta." }),
  ).toBeVisible();
  await expect(page.getByText("Email address verified")).toBeVisible();

  const results = await new AxeBuilder({ page })
    .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa"])
    .analyze();
  expect(results.violations).toEqual([]);

  await page.setViewportSize({ width: 390, height: 844 });
  const dimensions = await page.evaluate(() => ({
    viewport: document.documentElement.clientWidth,
    content: document.documentElement.scrollWidth,
  }));
  expect(dimensions.content).toBeLessThanOrEqual(dimensions.viewport);
});

async function latestDevelopmentEmail(
  page: import("@playwright/test").Page,
  email: string,
  kind: "email_verification" | "password_reset",
) {
  const response = await page.request.get(
    `/api/development/emails/latest?to=${encodeURIComponent(email)}&kind=${kind}`,
  );
  expect(response.ok()).toBe(true);
  return (await response.json()) as { action_url: string };
}
