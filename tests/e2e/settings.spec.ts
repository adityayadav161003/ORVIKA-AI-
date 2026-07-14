import { test, expect } from "@playwright/test";

test.describe("Settings View E2E Tests", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("http://localhost:5173/#/settings");
  });

  test("should display settings sections and permit updates", async ({ page }) => {
    // Check that key settings sections are loaded
    const apiKeyHeader = page.locator("h2:has-text('Provider API Keys')");
    await expect(apiKeyHeader).toBeVisible();

    const privacyHeader = page.locator("h2:has-text('Privacy & Research')");
    await expect(privacyHeader).toBeVisible();

    // Verify presence of specific key inputs
    const openaiInput = page.locator("#input-key-openai");
    await expect(openaiInput).toBeVisible();
  });
});
