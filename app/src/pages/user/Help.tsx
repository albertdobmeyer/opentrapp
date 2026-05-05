import { LifeBuoy } from "lucide-react";

import StillBuildingCard from "@/components/user/StillBuildingCard";

export default function Help() {
  return (
    <StillBuildingCard
      icon={LifeBuoy}
      title="Help & support"
      body="Plain-language answers and a copy-the-diagnostic-bundle button are coming. Until then, talk to your assistant on Telegram if you get stuck — it can usually help."
    />
  );
}
