import { useState } from "react";
import { DashboardPanel } from "./components/DashboardPanel";
import { IntentGraph } from "./components/IntentGraph";

function App() {
  const [graphTaskId, setGraphTaskId] = useState<string | null>(null);

  if (graphTaskId) {
    return (
      <IntentGraph
        taskId={graphTaskId}
        onClose={() => setGraphTaskId(null)}
      />
    );
  }

  return <DashboardPanel onOpenGraph={setGraphTaskId} />;
}

export default App;
