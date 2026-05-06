import { useEffect } from "react";

import ContactSupport from "@/components/failure/ContactSupport";
import FriendlyRetry from "@/components/failure/FriendlyRetry";
import { MissingRuntimeCard } from "@/components/wizard/install-step/MissingRuntimeCard";
import { StepList } from "@/components/wizard/install-step/StepList";
import { estimateRemaining } from "@/components/wizard/install-step/utils";
import { useInstallPipeline } from "@/hooks/useInstallPipeline";
import { useSettings } from "@/hooks/useSettings";

interface Props {
  onComplete: () => void;
  onBack: () => void;
}

/**
 * Wizard step C: orchestrates the four-phase install pipeline. Owns only
 * render-branch selection and auto-advance on success; pipeline state and
 * lifecycle live in `useInstallPipeline`.
 */
export default function InstallStep({ onComplete, onBack }: Props) {
  const { update } = useSettings();
  const { steps, outcome, tick, retry, escalateToLevel3 } = useInstallPipeline({ update });

  // Auto-advance once succeeded, after a brief pause so the user can see the
  // final green state.
  useEffect(() => {
    if (outcome.kind !== "succeeded") return;
    const t = setTimeout(() => { onComplete(); }, 1000);
    return () => { clearTimeout(t); };
  }, [outcome, onComplete]);

  if (outcome.kind === "missing-runtime") {
    return <MissingRuntimeCard onBack={onBack} onRetry={retry} />;
  }

  if (outcome.kind === "failed" && outcome.level === 3) {
    return (
      <ContactSupport
        classified={outcome.classified}
        onRetry={retry}
        titleOverride="Setup couldn’t finish"
      />
    );
  }

  if (outcome.kind === "failed" && outcome.level === 2) {
    return (
      <FriendlyRetry
        classified={outcome.classified}
        onRetry={retry}
        secondary={{ label: "Go back", action: onBack }}
        onGetHelp={escalateToLevel3}
      />
    );
  }

  return (
    <StepList
      steps={steps}
      remainingMs={estimateRemaining(steps)}
      tick={tick}
      succeeded={outcome.kind === "succeeded"}
    />
  );
}
