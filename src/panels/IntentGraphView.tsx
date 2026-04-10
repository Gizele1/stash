import { useEffect, useState, useCallback, type CSSProperties, type ReactElement } from "react";
import { invoke } from "@tauri-apps/api/core";
import { List } from "react-window";
import type { IntentTimeline, Intent, ShowGraphPayload } from "./types";
import { IntentNode } from "./components/IntentNode";

interface IntentGraphViewProps {
  payload: ShowGraphPayload;
  onDismiss: () => void;
}

/** Detect direction changes by comparing consecutive intent content */
function detectDirectionChanges(intents: Intent[]): Set<string> {
  const changes = new Set<string>();
  for (let i = 1; i < intents.length; i++) {
    const prev = intents[i - 1];
    const curr = intents[i];
    // Direction change: tier goes from label/summary to narrative,
    // or content substantially differs (heuristic: different first 20 chars)
    if (
      (prev.tier === "label" || prev.tier === "summary") &&
      curr.tier === "narrative"
    ) {
      changes.add(curr.id);
    } else if (
      prev.content.substring(0, 20) !== curr.content.substring(0, 20) &&
      curr.tier === "narrative"
    ) {
      changes.add(curr.id);
    }
  }
  return changes;
}

const ROW_HEIGHT = 72;

/** Extra props passed to the virtual list row via rowProps */
interface IntentRowProps {
  intents: Intent[];
  directionChanges: Set<string>;
}

/** Row component for react-window v2 List */
function IntentRow(props: {
  ariaAttributes: {
    "aria-posinset": number;
    "aria-setsize": number;
    role: "listitem";
  };
  index: number;
  style: CSSProperties;
} & IntentRowProps): ReactElement | null {
  const { index, style, intents, directionChanges } = props;
  const intent = intents[index];
  if (!intent) return null;
  return (
    <IntentNode
      intent={intent}
      isDirectionChange={directionChanges.has(intent.id)}
      style={style}
    />
  );
}

export function IntentGraphView({ payload, onDismiss }: IntentGraphViewProps) {
  const [timeline, setTimeline] = useState<IntentTimeline | null>(null);
  const [loading, setLoading] = useState(true);
  const [directionChanges, setDirectionChanges] = useState<Set<string>>(
    new Set(),
  );

  const fetchTimeline = useCallback(
    async (beforeId: string | null = null) => {
      setLoading(true);
      try {
        const result = await invoke<IntentTimeline>("get_intent_timeline", {
          contextId: payload.context_id,
          limit: 50,
          beforeId,
        });
        setTimeline((prev) => {
          if (prev && beforeId) {
            const merged = {
              intents: [...prev.intents, ...result.intents],
              has_more: result.has_more,
              hidden_count: result.hidden_count,
            };
            setDirectionChanges(detectDirectionChanges(merged.intents));
            return merged;
          }
          setDirectionChanges(detectDirectionChanges(result.intents));
          return result;
        });
      } catch {
        // Failed to fetch
      } finally {
        setLoading(false);
      }
    },
    [payload.context_id],
  );

  useEffect(() => {
    fetchTimeline();
  }, [fetchTimeline]);

  // Escape to dismiss
  useEffect(() => {
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onDismiss();
    };
    document.addEventListener("keydown", handleKey);
    return () => document.removeEventListener("keydown", handleKey);
  }, [onDismiss]);

  const handleLoadMore = useCallback(() => {
    if (!timeline || !timeline.has_more) return;
    const lastIntent = timeline.intents[timeline.intents.length - 1];
    if (lastIntent) {
      fetchTimeline(lastIntent.id);
    }
  }, [timeline, fetchTimeline]);

  const intents = timeline?.intents ?? [];
  const useVirtualScroll = intents.length > 50;

  return (
    <div className="panels-intent-graph">
      <div className="panels-graph-header">
        <h2>Intent Timeline</h2>
        <button type="button" className="panels-graph-close" onClick={onDismiss}>
          Close
        </button>
      </div>

      {loading && intents.length === 0 ? (
        <div className="panels-graph-loading">Loading timeline...</div>
      ) : (
        <div className="panels-graph-body">
          {timeline && timeline.hidden_count > 0 && (
            <div className="panels-graph-hidden">
              {timeline.hidden_count} archived intents hidden
            </div>
          )}

          {useVirtualScroll ? (
            <List
              style={{ height: 600, width: "100%" }}
              rowComponent={IntentRow}
              rowCount={intents.length}
              rowHeight={ROW_HEIGHT}
              rowProps={{ intents, directionChanges }}
            />
          ) : (
            intents.map((intent) => (
              <IntentNode
                key={intent.id}
                intent={intent}
                isDirectionChange={directionChanges.has(intent.id)}
              />
            ))
          )}

          {timeline?.has_more && (
            <button
              type="button"
              className="panels-graph-load-more"
              onClick={handleLoadMore}
              disabled={loading}
            >
              {loading ? "Loading..." : "Load more"}
            </button>
          )}
        </div>
      )}
    </div>
  );
}

export { detectDirectionChanges };
