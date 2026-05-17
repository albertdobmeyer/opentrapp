import {
  Activity,
  Boxes,
  FileCode,
  Gauge,
  KeyRound,
  LogOut,
  ScrollText,
  Settings as SettingsIcon,
  ShieldCheck,
  Terminal,
  type LucideIcon,
} from "lucide-react";
import { NavLink, Outlet, useNavigate } from "react-router-dom";

import { useAppContext } from "@/hooks/useAppContext";

interface NavItem {
  to: string;
  label: string;
  icon: LucideIcon;
  end?: boolean;
}

interface NavSection {
  heading: string;
  items: NavItem[];
}

const NAV_SECTIONS: NavSection[] = [
  {
    heading: "System",
    items: [
      { to: "/dev", label: "Overview", icon: Gauge, end: true },
      { to: "/dev/logs", label: "Logs", icon: ScrollText },
    ],
  },
  {
    heading: "Components",
    items: [{ to: "/dev/components", label: "All components", icon: Boxes }],
  },
  {
    heading: "Security",
    items: [
      { to: "/dev/security", label: "Audit", icon: ShieldCheck },
      { to: "/dev/allowlist", label: "Allowlist", icon: KeyRound },
      { to: "/dev/shell-levels", label: "Shell levels", icon: Terminal },
    ],
  },
  {
    heading: "Inspection",
    items: [{ to: "/dev/manifests", label: "Manifests", icon: FileCode }],
  },
  {
    heading: "Preferences",
    items: [
      { to: "/dev/preferences", label: "Settings", icon: SettingsIcon },
    ],
  },
];

export default function DevLayout() {
  const { setMode } = useAppContext();
  const navigate = useNavigate();

  async function exitDevMode() {
    await setMode("user");
    navigate("/", { replace: true });
  }

  return (
    <div className="h-screen flex flex-col bg-neutral-950 text-neutral-200">
      <header className="h-10 flex items-center justify-between px-4 border-b border-neutral-800 bg-neutral-900">
        <div className="flex items-center gap-2 text-sm font-medium">
          <Activity size={14} className="text-info-400" aria-hidden />
          <span>OpenTrApp</span>
          <span className="text-neutral-500">·</span>
          <span className="text-neutral-400">Advanced Mode</span>
        </div>
        <div className="flex items-center gap-3 text-xs">
          <kbd className="px-1.5 py-0.5 rounded border border-neutral-700 text-neutral-400 font-mono">
            ⌘⇧D
          </kbd>
          <button
            type="button"
            onClick={exitDevMode}
            className="btn btn-ghost btn-sm"
          >
            <LogOut size={14} aria-hidden />
            <span>Exit Advanced</span>
          </button>
        </div>
      </header>
      <div className="flex flex-1 min-h-0">
        <nav
          aria-label="Developer navigation"
          className="w-60 shrink-0 border-r border-neutral-800 bg-neutral-900 overflow-y-auto py-3"
        >
          {NAV_SECTIONS.map((section) => (
            <div key={section.heading} className="mb-4">
              <div className="px-4 mb-1 text-[10px] font-semibold uppercase tracking-wider text-neutral-500">
                {section.heading}
              </div>
              <ul>
                {section.items.map(({ to, label, icon: Icon, end }) => (
                  <li key={to}>
                    <NavLink
                      to={to}
                      end={end}
                      className={({ isActive }) =>
                        [
                          "flex items-center gap-2 px-4 py-1.5 text-sm",
                          "transition-colors duration-150",
                          isActive
                            ? "bg-neutral-800 text-neutral-100 border-l-2 border-info-500"
                            : "text-neutral-400 hover:text-neutral-200 hover:bg-neutral-850 border-l-2 border-transparent",
                        ].join(" ")
                      }
                    >
                      <Icon size={14} aria-hidden />
                      <span>{label}</span>
                    </NavLink>
                  </li>
                ))}
              </ul>
            </div>
          ))}
        </nav>
        <main className="flex-1 min-w-0 overflow-y-auto p-4">
          <Outlet />
        </main>
      </div>
    </div>
  );
}
