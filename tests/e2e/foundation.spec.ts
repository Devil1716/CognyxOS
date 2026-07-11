import { expect, test } from '@playwright/test';

test.skip('desktop shell e2e coverage begins when the Tauri host is introduced', async ({
  page,
}) => {
  await page.goto('/');
  await expect(page.getByText('CognyxOS')).toBeVisible();
});
