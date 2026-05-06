import { Shield } from "lucide-react";

import HeroStatusCard from "@/components/user/HeroStatusCard";
import ProactiveAlertsBanner from "@/components/user/ProactiveAlertsBanner";
import SpendingTile from "@/components/user/SpendingTile";
import StatTile, { type TileTone } from "@/components/user/StatTile";
import TipOfTheDay from "@/components/user/TipOfTheDay";
import { useHero, type HeroState } from "@/hooks/useHero";

export default function Home() {
  const { state, loading } = useHero();
  const security = securityFromHero(state);

  return (
    <div className="mx-auto max-w-3xl px-4 py-8 animate-fade-in">
      <ProactiveAlertsBanner />

      <HeroStatusCard state={state} loading={loading} />

      <div className="mt-6 grid grid-cols-1 gap-3 sm:grid-cols-2">
        <StatTile
          icon={Shield}
          iconTint="text-success-400"
          title="Security"
          value={security.value}
          subline={security.subline}
          href="/security"
          tone={security.tone}
        />
        <SpendingTile />
      </div>

      <TipOfTheDay />
    </div>
  );
}

interface SecurityCell {
  value: string;
  subline: string;
  tone: TileTone;
}

function securityFromHero(state: HeroState): SecurityCell {
  switch (state) {
    case "running_safely":
      return { value: "Safe", subline: "Sandbox is active.", tone: "neutral" };
    case "starting":
      return { value: "Starting…", subline: "Sandbox is coming up.", tone: "neutral" };
    case "recovering":
      return {
        value: "Recovering",
        subline: "Sandbox is restarting itself.",
        tone: "warning",
      };
    case "error_perimeter":
      return {
        value: "Needs attention",
        subline: "Sandbox isn't running.",
        tone: "danger",
      };
    case "error_key":
      // Sandbox itself is fine; the assistant just can't reach Anthropic.
      return { value: "Safe", subline: "Sandbox is active.", tone: "neutral" };
    case "not_setup":
      return { value: "Not set up", subline: "Run setup to begin.", tone: "neutral" };
    case "paused_by_user":
      return { value: "Paused", subline: "Sandbox is stopped on purpose.", tone: "neutral" };
  }
}
