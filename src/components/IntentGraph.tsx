import { useEffect, useMemo, useState } from "react";
import {
  ReactFlow,
  type Node,
  type Edge,
  Position,
  MarkerType,
  Background,
  Controls,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import dagre from "dagre";
import type { GraphData } from "../types/models";
import { api } from "../hooks/useTauri";

const NODE_WIDTH = 220;
const NODE_HEIGHT = 60;

function layoutGraph(nodes: Node[], edges: Edge[]): Node[] {
  const g = new dagre.graphlib.Graph();
  g.setDefaultEdgeLabel(() => ({}));
  g.setGraph({ rankdir: "TB", nodesep: 40, ranksep: 60 });

  for (const node of nodes) {
    g.setNode(node.id, { width: NODE_WIDTH, height: NODE_HEIGHT });
  }
  for (const edge of edges) {
    g.setEdge(edge.source, edge.target);
  }
  dagre.layout(g);

  return nodes.map((node) => {
    const pos = g.node(node.id);
    return {
      ...node,
      position: {
        x: pos.x - NODE_WIDTH / 2,
        y: pos.y - NODE_HEIGHT / 2,
      },
      sourcePosition: Position.Bottom,
      targetPosition: Position.Top,
    };
  });
}

interface IntentGraphProps {
  taskId: string;
  onClose: () => void;
}

export function IntentGraph({ taskId, onClose }: IntentGraphProps) {
  const [graphData, setGraphData] = useState<GraphData | null>(null);
  const [driftPrompt, setDriftPrompt] = useState<string | null>(null); // branchId being prompted
  const [driftSummary, setDriftSummary] = useState("");

  const loadGraph = () => {
    api.getGraphData(taskId).then(setGraphData).catch(console.error);
  };

  useEffect(() => {
    loadGraph();
  }, [taskId]);

  const handleMarkDrift = async () => {
    if (!driftPrompt || !driftSummary.trim()) return;
    try {
      await api.markDrift(driftPrompt, driftSummary.trim());
      setDriftPrompt(null);
      setDriftSummary("");
      loadGraph(); // refresh
    } catch (e) {
      console.error("Failed to mark drift:", e);
    }
  };

  const { nodes, edges } = useMemo(() => {
    if (!graphData) return { nodes: [], edges: [] };

    // Limit to last 20 intent nodes to keep graph readable
    const allIntents = graphData.intent_nodes;
    const recentIntents = allIntents.length > 20 ? allIntents.slice(-20) : allIntents;
    const intentIds = new Set(recentIntents.map((i) => i.id));

    const rawNodes: Node[] = recentIntents.map((intent) => ({
      id: intent.id,
      type: "default",
      data: {
        label: (
          <div style={{ fontSize: 11, lineHeight: 1.3 }}>
            <div style={{ fontWeight: 600, marginBottom: 2 }}>
              v{intent.version}
              {intent.id === graphData.current_intent_id && (
                <span style={{ color: "#3b82f6", marginLeft: 4 }}>current</span>
              )}
            </div>
            <div
              style={{
                color: "#374151",
                overflow: "hidden",
                textOverflow: "ellipsis",
                whiteSpace: "nowrap",
                maxWidth: 190,
              }}
            >
              {intent.statement}
            </div>
          </div>
        ),
      },
      position: { x: 0, y: 0 },
      style: {
        width: NODE_WIDTH,
        border:
          intent.id === graphData.current_intent_id
            ? "2px solid #3b82f6"
            : "1px solid #d1d5db",
        borderRadius: 8,
        padding: "8px 10px",
        background: "#fff",
      },
    }));

    // Edges: intent chain (version order)
    const rawEdges: Edge[] = [];
    for (let i = 1; i < recentIntents.length; i++) {
      rawEdges.push({
        id: `intent-${i}`,
        source: recentIntents[i - 1].id,
        target: recentIntents[i].id,
        type: "smoothstep",
        style: { stroke: "#9ca3af", strokeWidth: 2 },
        markerEnd: { type: MarkerType.ArrowClosed, color: "#9ca3af" },
      });
    }

    // Branch edges: show last 5 branches, remap fork point to visible intent if needed
    const lastVisibleIntentId = recentIntents.length > 0 ? recentIntents[recentIntents.length - 1].id : null;
    const recentBranches = graphData.branch_edges.slice(-5);
    for (const branch of recentBranches) {
      const forkTarget = intentIds.has(branch.forked_from_intent_id)
        ? branch.forked_from_intent_id
        : lastVisibleIntentId;
      if (!forkTarget) continue;
      const branchNodeId = `branch-${branch.branch_id}`;
      rawNodes.push({
        id: branchNodeId,
        type: "default",
        data: {
          label: (
            <div style={{ fontSize: 11 }}>
              <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
                <span
                  style={{
                    display: "inline-block",
                    width: 8,
                    height: 8,
                    borderRadius: "50%",
                    background: branch.color || "#6b7280",
                  }}
                />
                <span style={{ fontWeight: 500 }}>{branch.platform}</span>
                <span style={{ color: "#9ca3af" }}>{branch.status}</span>
                {branch.has_drift && (
                  <span
                    style={{
                      fontSize: 9,
                      background: "#fef3c7",
                      color: "#92400e",
                      padding: "1px 4px",
                      borderRadius: 3,
                    }}
                  >
                    drift
                  </span>
                )}
              </div>
              {branch.drift_summary && (
                <div style={{ fontSize: 10, color: "#92400e", marginTop: 2 }}>
                  {branch.drift_summary}
                </div>
              )}
              {!branch.has_drift && (
                <button
                  className="nodrag nopan"
                  onClick={(e) => {
                    e.stopPropagation();
                    setDriftPrompt(branch.branch_id);
                  }}
                  style={{
                    marginTop: 4,
                    fontSize: 10,
                    color: "#f59e0b",
                    background: "none",
                    border: "1px solid #fcd34d",
                    borderRadius: 3,
                    padding: "1px 6px",
                    cursor: "pointer",
                  }}
                >
                  Mark Drift
                </button>
              )}
            </div>
          ),
        },
        position: { x: 0, y: 0 },
        style: {
          width: NODE_WIDTH,
          border: branch.has_drift
            ? "2px solid #f59e0b"
            : `1px solid ${branch.color || "#d1d5db"}`,
          borderRadius: 8,
          padding: "6px 10px",
          background: "#fafafa",
        },
      });

      rawEdges.push({
        id: `fork-${branch.branch_id}`,
        source: forkTarget,
        target: branchNodeId,
        type: "smoothstep",
        style: {
          stroke: branch.color || "#6b7280",
          strokeWidth: 2,
          strokeDasharray: "5,5",
        },
        markerEnd: {
          type: MarkerType.ArrowClosed,
          color: branch.color || "#6b7280",
        },
      });
    }

    const laidOut = layoutGraph(rawNodes, rawEdges);
    return { nodes: laidOut, edges: rawEdges };
  }, [graphData]);

  return (
    <div style={{ height: "100vh", width: "100%", position: "relative" }}>
      <button
        onClick={onClose}
        style={{
          position: "absolute",
          top: 12,
          left: 12,
          zIndex: 10,
          fontSize: 12,
          padding: "4px 10px",
          borderRadius: 4,
          border: "1px solid #d1d5db",
          background: "#fff",
          cursor: "pointer",
        }}
      >
        Back
      </button>

      {/* Drift prompt modal */}
      {driftPrompt && (
        <div
          style={{
            position: "absolute",
            top: 12,
            right: 12,
            zIndex: 10,
            background: "#fff",
            border: "2px solid #f59e0b",
            borderRadius: 8,
            padding: 12,
            width: 250,
            boxShadow: "0 4px 12px rgba(0,0,0,0.15)",
          }}
        >
          <div style={{ fontSize: 12, fontWeight: 600, marginBottom: 6 }}>
            Mark Drift
          </div>
          <input
            autoFocus
            value={driftSummary}
            onChange={(e) => setDriftSummary(e.target.value)}
            placeholder="Describe the drift..."
            style={{
              width: "100%",
              padding: "6px 8px",
              fontSize: 12,
              borderRadius: 4,
              border: "1px solid #d1d5db",
              marginBottom: 8,
              boxSizing: "border-box",
            }}
            onKeyDown={(e) => {
              if (e.key === "Enter") handleMarkDrift();
              if (e.key === "Escape") {
                setDriftPrompt(null);
                setDriftSummary("");
              }
            }}
          />
          <div style={{ display: "flex", gap: 6 }}>
            <button
              onClick={handleMarkDrift}
              disabled={!driftSummary.trim()}
              style={{
                fontSize: 11,
                padding: "3px 10px",
                borderRadius: 4,
                border: "none",
                background: "#f59e0b",
                color: "#fff",
                cursor: "pointer",
              }}
            >
              Confirm
            </button>
            <button
              onClick={() => {
                setDriftPrompt(null);
                setDriftSummary("");
              }}
              style={{
                fontSize: 11,
                padding: "3px 10px",
                borderRadius: 4,
                border: "1px solid #d1d5db",
                background: "#fff",
                cursor: "pointer",
              }}
            >
              Cancel
            </button>
          </div>
        </div>
      )}

      {graphData && nodes.length > 0 ? (
        <ReactFlow nodes={nodes} edges={edges} fitView>
          <Background />
          <Controls />
        </ReactFlow>
      ) : (
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            height: "100%",
            color: "#9ca3af",
            fontSize: 13,
          }}
        >
          {graphData ? "No intent history yet" : "Loading graph..."}
        </div>
      )}
    </div>
  );
}
