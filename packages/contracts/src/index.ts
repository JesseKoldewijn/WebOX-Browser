export type WorkloadProfile = {
  name: string;
  category: "browser-app" | "visualization" | "game" | "wasm";
  goal: string;
};

export const workloadProfiles: WorkloadProfile[] = [
  {
    name: "CAD-style applications",
    category: "browser-app",
    goal: "Keep large interactive web apps responsive under high memory pressure.",
  },
  {
    name: "Large data visualization",
    category: "visualization",
    goal: "Support heavy DOM, canvas, and worker-driven dashboards.",
  },
  {
    name: "Unity WebGL",
    category: "game",
    goal: "Reduce avoidable OOM conditions for asset-heavy browser games.",
  },
  {
    name: "WASM-heavy tools",
    category: "wasm",
    goal: "Enable near-native workloads with higher per-tab headroom.",
  },
];

export const docsSections = [
  { slug: "architecture", title: "Architecture" },
  { slug: "getting-started", title: "Getting started" },
];
