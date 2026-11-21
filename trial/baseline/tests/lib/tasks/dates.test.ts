import { formatDate } from "../../../src/lib/tasks/dates";

it("[case:iso_date] formatDate returns the date part of a timestamp as YYYY-MM-DD in UTC", () => {
  expect(formatDate("2026-03-04T05:06:07.000Z")).toBe("2026-03-04");
  expect(formatDate("2026-03-04T23:30:00.000-02:00")).toBe("2026-03-05");
});
