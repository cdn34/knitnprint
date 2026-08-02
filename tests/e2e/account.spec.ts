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
