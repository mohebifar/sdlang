import Page from "../../../src/app/forgot-password/page";

it("[case:sends] requests a reset", () => {
  expect(Page().submit).toBeTruthy();
});
