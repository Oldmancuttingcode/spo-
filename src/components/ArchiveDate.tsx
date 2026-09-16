const dateFormat = new Intl.DateTimeFormat(undefined, {
  year: "numeric", month: "short", day: "numeric",
});

export default function ArchiveDate({ value, emptyText }: { value: string | null; emptyText: string }) {
  if (!value) return <>{emptyText}</>;
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? <>Unknown date</> : (
    <time dateTime={value} title={date.toLocaleString()}>{dateFormat.format(date)}</time>
  );
}
