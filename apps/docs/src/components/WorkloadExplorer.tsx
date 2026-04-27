import { createSignal, For } from "solid-js";
import type { WorkloadProfile } from "@webox/contracts";

type Props = {
  workloads: WorkloadProfile[];
};

export default function WorkloadExplorer(props: Props) {
  const [selected, setSelected] = createSignal(props.workloads[0]);

  return (
    <section class="panel">
      <div class="eyebrow">Interactive fit check</div>
      <h2>Explore target workloads</h2>
      <p>SolidJS powers the small interactive layer so the docs site stays mostly static while still explaining where webox fits best.</p>
      <div class="button-row">
        <For each={props.workloads}>
          {(workload) => (
            <button
              type="button"
              class="button-link"
              aria-pressed={selected().name === workload.name}
              onClick={() => setSelected(workload)}
            >
              {workload.name}
            </button>
          )}
        </For>
      </div>
      <div class="panel" style={{ margin: "18px 0 0" }}>
        <div class="eyebrow">Selected profile</div>
        <h3>{selected().name}</h3>
        <p>{selected().goal}</p>
        <p>
          Category: <strong>{selected().category}</strong>
        </p>
      </div>
    </section>
  );
}
