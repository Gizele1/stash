import { usePanelEvents } from "./hooks/usePanelEvents";
import { CardPopupView } from "./CardPopupView";
import { IntentGraphView } from "./IntentGraphView";
import { ManualInputView } from "./ManualInputView";
import { SettingsView } from "./SettingsView";

/**
 * Event-driven router for the panel window.
 * Listens to Tauri events and renders the appropriate view.
 */
export function PanelRouter() {
  const { view, cardPayload, graphPayload, dismiss } = usePanelEvents();

  switch (view) {
    case "card":
      if (!cardPayload) return null;
      return <CardPopupView payload={cardPayload} onDismiss={dismiss} />;

    case "input":
      return <ManualInputView onDismiss={dismiss} />;

    case "graph":
      if (!graphPayload) return null;
      return <IntentGraphView payload={graphPayload} onDismiss={dismiss} />;

    case "settings":
      return <SettingsView onDismiss={dismiss} />;

    case "none":
    default:
      return null;
  }
}
