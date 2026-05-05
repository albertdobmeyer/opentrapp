import { Outlet } from "react-router-dom";

import UserSidebar from "./UserSidebar";

export default function UserLayout() {
  return (
    <div className="flex h-screen flex-col overflow-hidden bg-neutral-900">
      <div className="h-0.5 flex-shrink-0 bg-primary-500" />
      <div className="flex flex-1 overflow-hidden">
        <UserSidebar />
        <main className="flex-1 overflow-y-auto">
          <div className="mx-auto max-w-6xl px-8 py-8">
            <Outlet />
          </div>
        </main>
      </div>
    </div>
  );
}
