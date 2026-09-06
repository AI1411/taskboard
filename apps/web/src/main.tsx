import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { createRoot } from "react-dom/client";

import { App } from "./App";
import { fetchBootstrap, transportFromBootstrap } from "./bootstrap";

const queryClient = new QueryClient();

async function main() {
  const bootstrap = await fetchBootstrap();
  const transport = transportFromBootstrap(bootstrap, window.location.origin);
  const root = document.getElementById("root");
  if (!root) {
    throw new Error("missing #root");
  }
  createRoot(root).render(
    <QueryClientProvider client={queryClient}>
      <App transport={transport} initialSequence={bootstrap.activitySequence ?? 0} />
    </QueryClientProvider>,
  );
}

void main();
