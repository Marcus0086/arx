import { describe, it, expect, beforeEach } from "vitest";
import { useFileSelectionStore } from "./file-selection-store";

// Reset store state between tests
beforeEach(() => {
  useFileSelectionStore.setState({
    vaultId: null,
    selected: new Set(),
    anchor: null,
  });
});

describe("file-selection-store", () => {
  it("starts with empty selection", () => {
    const { selected } = useFileSelectionStore.getState();
    expect(selected.size).toBe(0);
  });

  it("toggle adds a path to selection", () => {
    useFileSelectionStore.getState().toggle("file-a.txt");
    expect(useFileSelectionStore.getState().selected.has("file-a.txt")).toBe(true);
  });

  it("toggle removes an already-selected path", () => {
    useFileSelectionStore.getState().toggle("file-a.txt");
    useFileSelectionStore.getState().toggle("file-a.txt");
    expect(useFileSelectionStore.getState().selected.has("file-a.txt")).toBe(false);
  });

  it("toggle sets anchor to the toggled path", () => {
    useFileSelectionStore.getState().toggle("file-b.txt");
    expect(useFileSelectionStore.getState().anchor).toBe("file-b.txt");
  });

  it("selectAll selects all given paths", () => {
    const paths = ["a.txt", "b.txt", "c.txt"];
    useFileSelectionStore.getState().selectAll(paths);
    const { selected } = useFileSelectionStore.getState();
    for (const p of paths) expect(selected.has(p)).toBe(true);
    expect(selected.size).toBe(3);
  });

  it("clear resets selection and anchor", () => {
    useFileSelectionStore.getState().selectAll(["a.txt", "b.txt"]);
    useFileSelectionStore.getState().toggle("a.txt"); // sets anchor
    useFileSelectionStore.getState().clear();
    const { selected, anchor } = useFileSelectionStore.getState();
    expect(selected.size).toBe(0);
    expect(anchor).toBeNull();
  });

  it("selectRange selects items between anchor and target (inclusive)", () => {
    const allPaths = ["a.txt", "b.txt", "c.txt", "d.txt", "e.txt"];
    useFileSelectionStore.getState().toggle("b.txt"); // anchor = b
    useFileSelectionStore.getState().selectRange(allPaths, "d.txt"); // range b→d
    const { selected } = useFileSelectionStore.getState();
    expect(selected.has("b.txt")).toBe(true);
    expect(selected.has("c.txt")).toBe(true);
    expect(selected.has("d.txt")).toBe(true);
    expect(selected.has("a.txt")).toBe(false);
    expect(selected.has("e.txt")).toBe(false);
  });

  it("selectRange works in reverse order (target before anchor)", () => {
    const allPaths = ["a.txt", "b.txt", "c.txt", "d.txt"];
    useFileSelectionStore.getState().toggle("d.txt"); // anchor = d
    useFileSelectionStore.getState().selectRange(allPaths, "b.txt"); // range d→b (reverse)
    const { selected } = useFileSelectionStore.getState();
    expect(selected.has("b.txt")).toBe(true);
    expect(selected.has("c.txt")).toBe(true);
    expect(selected.has("d.txt")).toBe(true);
    expect(selected.has("a.txt")).toBe(false);
  });

  it("setVault clears selection when vaultId changes", () => {
    useFileSelectionStore.getState().setVault("vault-1");
    useFileSelectionStore.getState().selectAll(["x.txt", "y.txt"]);
    useFileSelectionStore.getState().setVault("vault-2");
    expect(useFileSelectionStore.getState().selected.size).toBe(0);
  });

  it("setVault does not clear selection if vaultId unchanged", () => {
    useFileSelectionStore.getState().setVault("vault-1");
    useFileSelectionStore.getState().selectAll(["x.txt"]);
    useFileSelectionStore.getState().setVault("vault-1"); // same vault
    expect(useFileSelectionStore.getState().selected.size).toBe(1);
  });
});
