import type { TaskDetail } from "@taskboard/types";

import { errorCode, errorMessage } from "../errors";

export async function recoverRevisionConflict(
  err: unknown,
  recover: {
    taskShow: () => Promise<TaskDetail>;
    applyDetail: (task: TaskDetail) => void;
    setToast: (toast: { message: string; error?: boolean }) => void;
  },
): Promise<boolean> {
  if (errorCode(err) !== "revision_conflict") return false;
  recover.setToast({ message: "Updated elsewhere", error: true });
  try {
    recover.applyDetail(await recover.taskShow());
  } catch (refreshErr) {
    recover.setToast({ message: errorMessage(refreshErr), error: true });
  }
  return true;
}
