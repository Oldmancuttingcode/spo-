import type { JSX } from "react";
import Calendar from "./Calendar";
import Digging from "./Digging";
import Home from "./Home";
import Library from "./Library";
import Playlists from "./Playlists";
import Settings from "./Settings";

export type PageId =
  | "home"
  | "calendar"
  | "digging"
  | "playlists"
  | "library"
  | "settings";

export const pages = {
  home: Home,
  calendar: Calendar,
  digging: Digging,
  playlists: Playlists,
  library: Library,
  settings: Settings,
} satisfies Record<PageId, () => JSX.Element>;
