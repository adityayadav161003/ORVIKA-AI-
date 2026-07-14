import { test, expect } from "@playwright/test";

test.describe("Documents Manager E2E Tests", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("http://localhost:5173/#/documents");
  });

  test("should load the documents view", async ({ page }) => {
    // Assert document title or header is present
    const header = page.locator("h2:has-text('Document Library')");
    await expect(header).toBeVisible();

    // Verify upload target zone and button
    const selectFileBtn = page.locator("button:has-text('Select File')").first();
    await expect(selectFileBtn).toBeVisible();
  });
});
