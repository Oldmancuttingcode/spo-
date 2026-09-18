import { useState } from "react";
import Sidebar from "./components/Sidebar";
import "./archive.css";
import { canLeave } from "./unsaved";
import { pages, type PageId } from "./pages";

const mainNavigation = [
  { id: "home", label: "Home" },
  { id: "calendar", label: "Calendar" },
  { id: "digging", label: "Digging" },
  { id: "playlists", label: "Playlists" },
  { id: "library", label: "Library" },
] satisfies Array<{ id: PageId; label: string }>;

const settingsNavigation = { id: "settings", label: "Settings" } satisfies {
  id: PageId;
  label: string;
};

export default function App() {
  const [activePage, setActivePage] = useState<PageId>("home");
  const ActivePage = pages[activePage];

  return (
    <div className="app-shell">
      <Sidebar
        activePage={activePage}
        mainItems={mainNavigation}
        settingsItem={settingsNavigation}
        onNavigate={page => { if (page !== activePage && canLeave()) setActivePage(page); }}
      />

      <main className="main-content">
        <ActivePage />
      </main>
    </div>
  );
}
