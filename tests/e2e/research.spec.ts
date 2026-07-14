import { test, expect } from "@playwright/test";

test.describe("Research Assistant E2E Tests", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("http://localhost:5173/#/research");
  });

  test("should display research panel structure", async ({ page }) => {
    // Assert that the Research History sidebar is visible
    const sidebarHeader = page.locator("h2:has-text('Research History')");
    await expect(sidebarHeader).toBeVisible();

    // Verify main content container exists
    const mainSection = page.locator("main");
    await expect(mainSection).toBeVisible();
  });
});
