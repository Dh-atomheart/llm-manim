import type { ComponentProps } from "react";

import ProjectJobsView from "./ProjectJobsView";

type HistoryViewProps = Omit<ComponentProps<typeof ProjectJobsView>, "mode">;

export default function HistoryView(props: HistoryViewProps) {
  return <ProjectJobsView mode="history" {...props} />;
}
