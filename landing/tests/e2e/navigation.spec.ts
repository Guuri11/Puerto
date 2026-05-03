import { test, expect } from "@playwright/test";

test("home page loads with correct title", async ({ page }) => {
  await page.goto("/");
  await expect(page).toHaveTitle(/Puerto/);
});

test("home hero has tagline", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByRole("heading", { level: 1 })).toContainText(
    "Scaffold",
  );
});

test("docs page loads", async ({ page }) => {
  await page.goto("/docs");
  await expect(page).toHaveTitle(/Getting Started/);
});

test("header Get Started link navigates to docs", async ({ page }) => {
  await page.goto("/");
  await page.click('a[href="/docs"]');
  await expect(page).toHaveURL("/docs");
});

test("quick start section visible on home", async ({ page }) => {
  await page.goto("/");
  await expect(page.locator("#quickstart")).toBeVisible();
});

test("features section visible on home", async ({ page }) => {
  await page.goto("/");
  await expect(page.locator("#features")).toBeVisible();
});

test("footer is present", async ({ page }) => {
  await page.goto("/");
  await expect(page.locator("footer")).toBeVisible();
});

test("docs sidebar links exist", async ({ page }) => {
  await page.goto("/docs");
  await expect(page.locator('a[href="#installation"]')).toBeVisible();
  await expect(page.locator('a[href="#scaffold"]')).toBeVisible();
});
