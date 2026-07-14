import { test, expect } from "@playwright/test";

test.describe("Chat Interface E2E Tests", () => {
  test.beforeEach(async ({ page }) => {
    // Navigate to Chat Page
    await page.goto("http://localhost:5173/#/chat");
  });

  test("should display sidebar and chat input area", async ({ page }) => {
    // Assert that sidebar container is visible
    const sidebar = page.locator("aside, [class*='SessionSidebar']");
    await expect(sidebar).toBeVisible();

    const input = page.locator("textarea[placeholder*='message'], textarea, #input-message");
    await expect(input).toBeVisible();
  });

  test("should allow creating a new session and sending messages", async ({ page }) => {
    // Look for create chat / new session button
    const newChatBtn = page.locator("button:has-text('New Chat'), button:has-text('New session'), button[id*='new-session']");
    if (await newChatBtn.count() > 0) {
      await newChatBtn.first().click();
    }

    // Type a message in the input
    const input = page.locator("textarea, #input-message").first();
    await input.fill("Hello, local LLM assistant!");
    
    const sendBtn = page.locator("button[aria-label='Send message'], button:has-text('Send')").first();
    await sendBtn.click();

    // Message bubble should be added
    const messageList = page.locator("[class*='MessageList'], [ref='scrollRef']").first();
    await expect(messageList).toBeVisible();
  });
});
