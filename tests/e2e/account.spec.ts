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
  await page.getByLabel("Password", { exact: true }).fill(password);
  await page.getByLabel("Confirm password", { exact: true }).fill("not-the-same-password");
  await page.getByRole("button", { name: "Show Confirm password" }).click();
  await expect(page.getByLabel("Confirm password", { exact: true })).toHaveAttribute("type", "text");
  await page.getByRole("button", { name: "Hide Confirm password" }).click();
  await expect(page.getByLabel("Confirm password", { exact: true })).toHaveAttribute("type", "password");
  await page.getByRole("button", { name: "Create account" }).last().click();
  await expect(page.getByRole("alert")).toHaveText("The passwords do not match. Please try again.");
  await page.getByLabel("Confirm password", { exact: true }).fill(password);
  await page.getByRole("button", { name: "Create account" }).last().click();

  await expect(
    page.getByRole("heading", { name: "Verify your email to continue" }),
  ).toBeVisible();
  await expect(page.getByText(email)).toBeVisible();

  await page.getByRole("button", { name: "Back to sign in" }).click();
  await page.getByLabel("Email address").fill(email);
  await page.getByLabel("Password", { exact: true }).fill(password);
  await page.getByRole("button", { name: "Sign in" }).last().click();
  await expect(page.getByRole("heading", { name: "Verify your email to continue" })).toBeVisible();

  const verification = await latestDevelopmentEmail(page, email, "email_verification");
  await page.goto(verification.action_url);
  await expect(page.getByRole("status")).toHaveText("Your email address is verified.");
  await page.getByLabel("Email address").fill(email);
  await page.getByLabel("Password", { exact: true }).fill(password);
  await page.getByRole("button", { name: "Sign in" }).last().click();

  await expect(page.getByRole("heading", { name: "Welcome, Marta." })).toBeVisible();
  await page.reload();
  await expect(
    page.getByRole("heading", { name: "Welcome, Marta." }),
  ).toBeVisible();

  await expect(page.getByRole("heading", { name: "Current orders" })).toBeVisible();
  await expect(page.getByText("No orders in progress.", { exact: false })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Order history" })).toBeVisible();
  await expect(page.getByText("Completed and cancelled orders will appear here.")).toBeVisible();

  await page.getByLabel("First name").fill("Maria");
  await page.getByLabel("Last name").fill("Lopes");
  await page.getByLabel("Phone (optional)", { exact: true }).fill("+351 210 123 456");
  await page.getByRole("button", { name: "Save details" }).click();
  await expect(page.locator(".account-profile-form").getByRole("status")).toHaveText("Your details have been updated.");
  await page.reload();
  await expect(page.getByRole("heading", { name: "Welcome, Maria." })).toBeVisible();
  await expect(page.getByLabel("Last name")).toHaveValue("Lopes");
  await expect(page.getByLabel("Phone (optional)", { exact: true })).toHaveValue("+351 210 123 456");

  const updatedEmail = `updated-${email}`;
  await page.getByLabel("New email address").fill(updatedEmail);
  await page.getByRole("button", { name: "Send confirmation link" }).click();
  await expect(page.getByRole("status")).toHaveText(
    `Check ${updatedEmail} for a confirmation link. Your current email remains active until you use it.`,
  );
  await expect(page.getByText(`Current email: ${email}`)).toBeVisible();
  const emailChange = await latestDevelopmentEmail(page, updatedEmail, "email_change");
  await page.goto(emailChange.action_url);
  await expect(page.getByText(`Current email: ${updatedEmail}`)).toBeVisible();

  await page.getByLabel("Recipient name").fill("Marta Silva");
  await page.getByLabel("Address line 1").fill("12 Rua das Flores");
  await page.getByLabel("City").fill("Porto");
  await page.getByLabel("Postal code").fill("4050-265");
  await page.getByRole("combobox", { name: "Country", exact: true }).selectOption("PT");
  await page.getByRole("button", { name: "Save address" }).click();

  await expect(page.locator(".account-message--success")).toHaveText("Address saved.");
  const address = page
    .getByRole("article")
    .filter({ hasText: "12 Rua das Flores" });
  await expect(address).toContainText("4050-265 Porto");
  await expect(address).toContainText("PT");
  await address.getByRole("button", { name: "Edit address" }).click();
  const addressForm = page.locator(".address-edit-form");
  await addressForm.getByLabel("City").fill("Coimbra");
  await addressForm.getByRole("button", { name: "Save changes" }).click();
  await expect(address).toContainText("4050-265 Coimbra");

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
  await page.getByLabel("Email address").fill(updatedEmail);
  await page.getByLabel("Password", { exact: true }).fill(password);
  await page.getByRole("button", { name: "Sign in" }).last().click();

  await expect(
    page.getByRole("heading", { name: "Welcome, Maria." }),
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
  await page.getByLabel("Password", { exact: true }).fill(oldPassword);
  await page.getByLabel("Confirm password", { exact: true }).fill(oldPassword);
  await page.getByRole("button", { name: "Create account" }).last().click();

  await expect(page.getByRole("heading", { name: "Verify your email to continue" })).toBeVisible();
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
  await page.getByLabel("Email address").fill(email);
  await page.getByLabel("Password", { exact: true }).fill(oldPassword);
  await page.getByRole("button", { name: "Sign in" }).last().click();
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
  await page.getByLabel("Password", { exact: true }).fill(newPassword);
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
  kind: "email_verification" | "password_reset" | "email_change",
) {
  const response = await page.request.get(
    `/api/development/emails/latest?to=${encodeURIComponent(email)}&kind=${kind}`,
  );
  expect(response.ok()).toBe(true);
  return (await response.json()) as { action_url: string };
}
