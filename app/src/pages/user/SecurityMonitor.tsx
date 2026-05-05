import { ShieldCheck } from "lucide-react";

import StillBuildingCard from "@/components/user/StillBuildingCard";

export default function SecurityMonitor() {
  return (
    <StillBuildingCard
      icon={ShieldCheck}
      title="Security & activity"
      body="Your assistant is already running safely behind a sandbox — we just haven't finished the dashboard for it yet. We'll show you what your assistant has been doing, what it tried to visit, and what got blocked."
    />
  );
}
