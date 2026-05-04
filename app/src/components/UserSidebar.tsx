import { NavLink } from "react-router-dom";
import {
  Home as HomeIcon,
  ShieldCheck,
  Sparkles,
  SlidersHorizontal,
  LifeBuoy,
  type LucideIcon,
} from "lucide-react";

interface NavItem {
  to: string;
  end?: boolean;
  label: string;
  icon: LucideIcon;
}

const NAV_ITEMS: NavItem[] = [
  { to: "/", end: true, label: "Home", icon: HomeIcon },
  { to: "/security", label: "Security", icon: ShieldCheck },
  { to: "/discover", label: "Discover", icon: Sparkles },
  { to: "/preferences", label: "Preferences", icon: SlidersHorizontal },
  { to: "/help", label: "Help", icon: LifeBuoy },
];

export default function UserSidebar() {
  return (
    <aside
      className="flex h-full w-20 flex-col items-center border-r border-neutral-800 bg-neutral-900 py-4"
      aria-label="Main navigation"
    >
      <img
        src="/logo-square.png"
        alt=""
        aria-hidden
        className="mb-6 h-12 w-12 rounded-xl"
      />
      <nav className="flex flex-1 flex-col gap-1">
        {NAV_ITEMS.map(({ to, end, label, icon: Icon }) => (
          <NavLink
            key={to}
            to={to}
            end={end}
            aria-label={label}
            className={({ isActive }) =>
              `group flex h-16 w-16 flex-col items-center justify-center gap-1 rounded-xl text-[10px] font-medium uppercase tracking-wide transition-colors duration-150 ${
                isActive
                  ? "bg-primary-500/15 text-primary-400"
                  : "text-neutral-500 hover:bg-neutral-850 hover:text-neutral-200"
              }`
            }
          >
            <Icon
              size={22}
              strokeWidth={1.75}
              className="transition-transform duration-150 group-hover:scale-105"
            />
            <span>{label}</span>
          </NavLink>
        ))}
      </nav>
    </aside>
  );
}
