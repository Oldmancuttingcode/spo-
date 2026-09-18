export const localDate = (date = new Date()) => `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;
export function monthRanges(month: string) {
  const [year, m] = month.split("-").map(Number);
  return Array.from({ length: new Date(year, m, 0).getDate() }, (_, i) => ({ date: localDate(new Date(year, m - 1, i + 1)), start: new Date(year, m - 1, i + 1).toISOString(), end: new Date(year, m - 1, i + 2).toISOString() }));
}
