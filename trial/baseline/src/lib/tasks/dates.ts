// Date formatting for task pages.

/** The date part of an ISO timestamp, as YYYY-MM-DD in UTC. */
export function formatDate(timestamp: string): string {
  return new Date(timestamp).toISOString().slice(0, 10);
}
