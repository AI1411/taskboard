import { useEffect, useRef, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import type { HttpTransport } from "@taskboard/client";
import { TaskboardApp } from "@taskboard/ui";

import { refetchInterval } from "./bootstrap";

export function App(props: {
  transport: HttpTransport;
  initialSequence?: number;
}) {
  const { transport, initialSequence = 0 } = props;
  const lastSeq = useRef(initialSequence);
  const [boardSeq, setBoardSeq] = useState(initialSequence);
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
