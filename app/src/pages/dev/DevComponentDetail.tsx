import { useParams } from "react-router-dom";

import DevPlaceholder from "./DevPlaceholder";

export default function DevComponentDetail() {
  const { id } = useParams<{ id: string }>();
  return (
    <DevPlaceholder
      title={id ?? "Component"}
      summary="Per-component dashboard — commands, configs, workflows, logs, manifest."
      specRef="ui-rebuild-2026-04-21/developer-mode/14-component-operations.md"
    />
  );
}
