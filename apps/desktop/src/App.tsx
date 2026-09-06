import { useEffect, useMemo, useRef, useState } from "react";
import { QueryClient, QueryClientProvider, useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { TauriTransport, type InvokeFn } from "@taskboard/client";
import { TaskboardApp } from "@taskboard/ui";

// Never import HttpTransport under apps/desktop. Desktop uses TauriTransport + invoke only.

function refetchInterval(visibility: DocumentVisibilityState): number | false {
  return visibility === "visible" ? 1000 : false;
}

function BoardShell({ invoke: invokeFn }: { invoke: InvokeFn }) {
  const transport = useMemo(() => new TauriTransport(invokeFn), [invokeFn]);
  const lastSeq = useRef(0);
  const [boardSeq, setBoardSeq] = useState(0);
  const [visible, setVisible] = useState(
    () => typeof document === "undefined" || document.visibilityState === "visible",
  );

  useEffect(() => {
    const onChange = () => setVisible(document.visibilityState === "visible");
    document.addEventListener("visibilitychange", onChange);
    return () => document.removeEventListener("visibilitychange", onChange);
  }, []);

  useQuery({
    queryKey: ["sync"],
    queryFn: async () => {
      const delta = await transport.sync(lastSeq.current);
      if (delta.sequence !== lastSeq.current) {
        lastSeq.current = delta.sequence;
        setBoardSeq(delta.sequence);
      }
      return delta;
    },
    refetchInterval: refetchInterval(visible ? "visible" : "hidden"),
  });

  return <TaskboardApp transport={transport} sequence={boardSeq} />;
}

export function AppWithInvoke({ invoke }: { invoke: InvokeFn }) {
  const [queryClient] = useState(() => new QueryClient());
  return (
    <QueryClientProvider client={queryClient}>
      <BoardShell invoke={invoke} />
    </QueryClientProvider>
  );
}

export function App() {
  return <AppWithInvoke invoke={invoke} />;
}
