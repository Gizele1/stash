import { DashboardPanel } from "./components/DashboardPanel";
import { api } from "./hooks/useTauri";

function App() {
  const handleOpenGraph = (taskId: string) => {
    api.openGraphWindow(taskId).catch(console.error);
  };

  return <DashboardPanel onOpenGraph={handleOpenGraph} />;
}

export default App;
