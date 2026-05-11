import { listen } from "@tauri-apps/api/event";
import { Shield } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import ActivationModal from "@/components/ActivationModal";
import HeroStatusCard from "@/components/user/HeroStatusCard";
import ProactiveAlertsBanner from "@/components/user/ProactiveAlertsBanner";
import SpendingTile from "@/components/user/SpendingTile";
import StatTile, { type TileTone } from "@/components/user/StatTile";
import TipOfTheDay from "@/components/user/TipOfTheDay";
import { useHero, type HeroState } from "@/hooks/useHero";

export default function Home() {
  const { state, loading, snapshot } = useHero();
  const security = securityFromHero(state);

  const [activationOpen, setActivationOpen] = useState(false);
  const [reCredential, setReCredential] = useState(false);
  const autoOpenFiredRef = useRef(false);

  // Auto-open the activation modal on first load when the shell is ready
  // but the user hasn't activated yet. Only fires once; if the user closes
  // it the button in HeroStatusCard lets them reopen it.
  useEffect(() => {
    if (!loading && state === "shell_ready_absent" && !autoOpenFiredRef.current) {
      autoOpenFiredRef.current = true;
      setActivationOpen(true);
    }
  }, [loading, state]);

  // Listen for the migration re-credential event emitted by auto_activate
  // when the migrated Anthropic key is rejected — open the modal in
  // re-credential mode so the user only needs to re-enter the Anthropic key.
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void listen("migration-needs-recredential", () => {
      setReCredential(true);
      setActivationOpen(true);
    }).then((fn) => { unlisten = fn; });
    return () => { unlisten?.(); };
  }, []);

  return (
    <div className="mx-auto max-w-3xl px-4 py-8 animate-fade-in">
      <ProactiveAlertsBanner />

      <HeroStatusCard
        state={state}
        loading={loading}
        onLaunch={() => setActivationOpen(true)}
        bootstrapFailure={snapshot.bootstrap_failure}
      />

      {activationOpen && (
        <ActivationModal
          onClose={() => { setActivationOpen(false); setReCredential(false); }}
          reCredential={reCredential}
        />
      )}

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
    case "installing":
    case "bootstrapping":
      return { value: "Setting up…", subline: "Sandbox is being built.", tone: "neutral" };
    case "shell_ready_absent":
      return { value: "Ready", subline: "Sandbox is up. Launch your assistant.", tone: "neutral" };
    case "shell_failed":
      return { value: "Needs attention", subline: "Sandbox setup failed.", tone: "danger" };
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
      return { value: "Stopped", subline: "Sandbox is stopped on purpose.", tone: "neutral" };
  }
}
