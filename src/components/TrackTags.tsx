import { invoke } from "@tauri-apps/api/core";
import { useEffect, useRef, useState } from "react";
import "./TrackTags.css";

const categories = [
  ["genre", "Genre"], ["mood", "Mood"], ["sound", "Sound"],
  ["vocal", "Vocal"], ["free", "Free Tag"],
] as const;
type Category = typeof categories[number][0];
interface Tag { id: number; name: string; category: Category }
const errorMessage = (error: unknown) => typeof error === "string" ? error : "Could not load or update tags. Please try again.";

export default function TrackTags({ trackId }: { trackId: number }) {
  const [tags, setTags] = useState<Tag[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [attempt, setAttempt] = useState(0);
  const [category, setCategory] = useState<Category | null>(null);
  const [available, setAvailable] = useState<Tag[]>([]);
  const [pickerLoading, setPickerLoading] = useState(false);
  const [pickerError, setPickerError] = useState<string | null>(null);
  const [pickerAttempt, setPickerAttempt] = useState(0);
  const [search, setSearch] = useState("");
  const [pending, setPending] = useState(false);
  const [mutationError, setMutationError] = useState<string | null>(null);
  const busy = useRef(false);
  const alive = useRef(true);
  const addButtons = useRef<Partial<Record<Category, HTMLButtonElement | null>>>({});
  const input = useRef<HTMLInputElement>(null);
  const returnFocus = useRef<Category | null>(null);

  useEffect(() => {
    if (!pending && returnFocus.current) {
      addButtons.current[returnFocus.current]?.focus();
      returnFocus.current = null;
    }
  }, [pending]);

  useEffect(() => { alive.current = true; return () => { alive.current = false; }; }, []);
  useEffect(() => {
    let active = true;
    setLoading(true);
    setLoadError(null);
    invoke<Tag[]>("track_tags", { trackId })
      .then((result) => { if (active) setTags(result); })
      .catch((error) => { if (active) setLoadError(errorMessage(error)); })
      .finally(() => { if (active) setLoading(false); });
    return () => { active = false; };
  }, [trackId, attempt]);

  useEffect(() => {
    if (!category) return;
    let active = true;
    setPickerLoading(true);
    setPickerError(null);
    setAvailable([]);
    input.current?.focus();
    invoke<Tag[]>("list_tags", { category })
      .then((result) => { if (active) setAvailable(result); })
      .catch((error) => { if (active) setPickerError(errorMessage(error)); })
      .finally(() => { if (active) setPickerLoading(false); });
    return () => { active = false; };
  }, [category, pickerAttempt]);

  function closePicker() {
    if (category) addButtons.current[category]?.focus();
    setCategory(null);
    setSearch("");
  }

  async function mutate(action: "assign" | "create" | "remove", tag?: Tag) {
    if (busy.current) return;
    busy.current = true;
    setPending(true);
    setMutationError(null);
    try {
      const selected = action === "create"
        ? await invoke<Tag>("create_tag", { name: search, category }) : tag!;
      await invoke(action === "remove" ? "remove_tag_from_track" : "assign_tag_to_track", { trackId, tagId: selected.id });
      if (!alive.current) return;
      returnFocus.current = selected.category;
      if (action !== "remove") closePicker();
      try {
        const result = await invoke<Tag[]>("track_tags", { trackId });
        if (alive.current) { setTags(result); setLoadError(null); }
      } catch {
        if (alive.current) setLoadError("The change was saved, but tags could not be refreshed. Please retry.");
      }
    } catch (error) {
      if (alive.current) setMutationError(errorMessage(error));
    } finally {
      busy.current = false;
      if (alive.current) setPending(false);
    }
  }

  const filtered = available.filter((tag) => tag.name.toLowerCase().includes(search.trim().toLowerCase()));
  const exact = available.find((tag) => tag.name.trim().toLowerCase() === search.trim().toLowerCase());
  return <section className="track-tags" aria-label="Track tags" aria-busy={pending}>
    {loading && <p role="status">Loading tags…</p>}
    {loadError && <div role="alert"><p>{loadError}</p><button type="button" className="secondary-button" disabled={loading || pending} onClick={() => setAttempt((value) => value + 1)}>Retry tags</button></div>}
    {mutationError && <p role="alert">{mutationError}</p>}
    {pending && <p role="status">Saving tags…</p>}
    {!loading && !loadError && categories.map(([value, label]) => {
      const assigned = tags.filter((tag) => tag.category === value);
      return <section className="tag-category" key={value} aria-labelledby={`tag-heading-${value}`}>
        <h2 id={`tag-heading-${value}`}>{label}</h2>
        <div className="tag-chips">
          {assigned.length === 0 && <span className="tag-empty">No tags yet</span>}
          {assigned.map((tag) => <span className="tag-chip" key={tag.id}>{tag.name}
            <button type="button" aria-label={`Remove ${tag.name} tag`} disabled={pending} onClick={() => void mutate("remove", tag)}>×</button>
          </span>)}
          <button type="button" className="secondary-button tag-add" ref={(element) => { addButtons.current[value] = element; }}
            aria-label={`Add ${label} tag`} aria-expanded={category === value} aria-controls={category === value ? `tag-picker-${value}` : undefined}
            disabled={pending} onClick={() => { setSearch(""); setMutationError(null); setCategory(category === value ? null : value); }}>+ Add</button>
        </div>
        {category === value && <form className="tag-picker" id={`tag-picker-${value}`} aria-label={`Add ${label} tag`}
          onSubmit={(event) => {
            event.preventDefault();
            if (!search.trim() || pickerLoading || pickerError) return;
            if (!exact) void mutate("create");
            else if (!tags.some((tag) => tag.id === exact.id)) void mutate("assign", exact);
          }}
          onKeyDown={(event) => { if (event.key === "Escape" && !pending) { event.preventDefault(); closePicker(); } }}>
          <label htmlFor={`tag-search-${value}`}>Find or create a {label} tag</label>
          <input ref={input} id={`tag-search-${value}`} value={search} disabled={pending} onChange={(event) => setSearch(event.target.value)} placeholder="Tag name" autoComplete="off" />
          {pickerLoading && <p role="status">Loading available tags…</p>}
          {pickerError && <div role="alert"><p>{pickerError}</p><button type="button" className="secondary-button" disabled={pending} onClick={() => setPickerAttempt((value) => value + 1)}>Retry available tags</button></div>}
          {!pickerLoading && !pickerError && <>
            <ul className="tag-options" aria-label={`Existing ${label} tags`}>
              {filtered.map((tag) => {
                const isAssigned = tags.some((assignedTag) => assignedTag.id === tag.id);
                return <li key={tag.id}><button type="button" disabled={pending || isAssigned} onClick={() => void mutate("assign", tag)}>{tag.name}{isAssigned ? " (added)" : ""}</button></li>;
              })}
            </ul>
            {filtered.length === 0 && <p>No matching tags.</p>}
            {search.trim() && !exact && <button type="submit" className="secondary-button" disabled={pending}>Create & add “{search.trim()}”</button>}
          </>}
          <button type="button" className="secondary-button" disabled={pending} onClick={closePicker}>Cancel</button>
        </form>}
      </section>;
    })}
  </section>;
}
