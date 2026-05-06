import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";

import ConnectStep from "@/components/wizard/ConnectStep";
import InstallStep from "@/components/wizard/InstallStep";
import ReadyStep from "@/components/wizard/ReadyStep";
import WelcomeStep from "@/components/wizard/WelcomeStep";
import WizardProgress from "@/components/wizard/WizardProgress";
import { useSettings } from "@/hooks/useSettings";
import { useWizardProgress } from "@/hooks/useWizardProgress";

import type { SetupStep } from "@/lib/settings";

const STEP_ORDER: SetupStep[] = ["welcome", "connect", "install", "ready"];

export default function Setup() {
  const navigate = useNavigate();
  const { settings, loaded: settingsLoaded } = useSettings();
  const { progress, loaded: progressLoaded, recordStep, complete } =
    useWizardProgress();

  const [step, setStep] = useState<SetupStep>("welcome");
  const [initialised, setInitialised] = useState(false);

  // On first load, resume at persisted step if any.
  useEffect(() => {
    if (!progressLoaded || !settingsLoaded || initialised) return;
    if (progress?.step && STEP_ORDER.includes(progress.step)) {
      setStep(progress.step);
    }
    setInitialised(true);
  }, [progressLoaded, settingsLoaded, progress, initialised]);

  async function advance(next: SetupStep, opts?: { skippedKeys?: boolean }) {
    setStep(next);
    await recordStep(next, opts);
  }

  async function goBack() {
    const idx = STEP_ORDER.indexOf(step);
    if (idx > 0) {
      const prev = STEP_ORDER[idx - 1];
      setStep(prev);
      await recordStep(prev);
    }
  }

  async function finish() {
    await complete();
    navigate("/");
  }

  return (
    <div className="flex min-h-screen flex-col bg-neutral-950">
      {step !== "welcome" && (
        <div className="px-6 pt-8">
          <WizardProgress
            currentStep={step}
            completedSteps={progress?.completedSteps ?? []}
          />
        </div>
      )}

      <div className="flex flex-1 items-center justify-center px-6 pb-10">
        {step === "welcome" && (
          <WelcomeStep
            onNext={() => advance("connect")}
            canSkipToDashboard={settings.wizardCompleted}
            onSkipToDashboard={() => { navigate("/"); }}
          />
        )}

        {step === "connect" && (
          <ConnectStep
            onContinue={({ skippedKeys }) => advance("install", { skippedKeys })}
            onBack={goBack}
          />
        )}

        {step === "install" && (
          <InstallStep onComplete={() => advance("ready")} onBack={goBack} />
        )}

        {step === "ready" && <ReadyStep onGoToDashboard={finish} />}
      </div>
    </div>
  );
}
