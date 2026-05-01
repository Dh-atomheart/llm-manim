import type { ComponentProps } from "react";

import ProjectJobsView from "./ProjectJobsView";

type WorkbenchViewProps = Omit<ComponentProps<typeof ProjectJobsView>, "mode">;

export default function WorkbenchView(props: WorkbenchViewProps) {
  return <ProjectJobsView mode="workbench" {...props} />;
}
