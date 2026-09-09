import type { PageId } from "../pages";

type NavigationItem = {
  id: PageId;
  label: string;
};

type SidebarProps = {
  activePage: PageId;
  mainItems: NavigationItem[];
  settingsItem: NavigationItem;
  onNavigate: (page: PageId) => void;
};

export default function Sidebar({
  activePage,
  mainItems,
  settingsItem,
  onNavigate,
}: SidebarProps) {
  const renderItem = (item: NavigationItem) => {
    const isActive = item.id === activePage;

    return (
      <button
        key={item.id}
        type="button"
        className={isActive ? "nav-item active" : "nav-item"}
        aria-current={isActive ? "page" : undefined}
        onClick={() => onNavigate(item.id)}
      >
        {item.label}
      </button>
    );
  };

  return (
    <aside className="sidebar" aria-label="Primary navigation">
      <div className="sidebar-header">
        <div className="app-title">Music Archive</div>
      </div>

      <nav className="nav-group" aria-label="Main navigation">
        {mainItems.map(renderItem)}
      </nav>

      <nav className="nav-group settings-nav" aria-label="Settings">
        {renderItem(settingsItem)}
      </nav>
    </aside>
  );
}
