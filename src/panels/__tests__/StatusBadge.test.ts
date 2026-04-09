import { describe, it, expect } from "vitest";
import { STATUS_COLORS } from "../types";
import type { ContextWithStatus } from "../types";

/**
 * StatusBadge logic tests.
 * Verifies correct color mapping for each status.
 */
describe("StatusBadge color mapping", () => {
  const testCases: {
    status: ContextWithStatus["status"];
    expectedColor: string;
    description: string;
  }[] = [
    {
      status: "running",
      expectedColor: "#5cb8a5",
      description: "running shows secondary (green) color",
    },
    {
      status: "done",
      expectedColor: "#534AB7",
      description: "done shows primary (purple) color",
    },
    {
      status: "stuck",
      expectedColor: "#ffb4ab",
      description: "stuck shows danger (red) color",
    },
    {
      status: "parked",
      expectedColor: "#928f9e",
      description: "parked shows muted color",
    },
  ];

  for (const { status, expectedColor, description } of testCases) {
    it(description, () => {
      expect(STATUS_COLORS[status]).toBe(expectedColor);
    });
  }

  it("has exactly four status entries", () => {
    expect(Object.keys(STATUS_COLORS)).toHaveLength(4);
  });

  it("all values are valid hex colors", () => {
    for (const color of Object.values(STATUS_COLORS)) {
      expect(color).toMatch(/^#[0-9a-fA-F]{6}$/);
    }
  });
});
