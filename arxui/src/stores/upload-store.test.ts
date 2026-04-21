import { describe, it, expect, beforeEach } from "vitest";
import { useUploadStore } from "./upload-store";

beforeEach(() => {
  useUploadStore.setState({ items: [] });
});

describe("upload-store", () => {
  const mockFile = (name: string, size = 1000) =>
    new File(["x".repeat(size)], name, { type: "text/plain" });

  it("starts with no items", () => {
    expect(useUploadStore.getState().items).toHaveLength(0);
  });

  it("add enqueues files with queued status", () => {
    const files = [mockFile("a.txt"), mockFile("b.txt")];
    useUploadStore.getState().add(files, "vault-1");
    const { items } = useUploadStore.getState();
    expect(items).toHaveLength(2);
    expect(items[0].status).toBe("queued");
    expect(items[1].fileName).toBe("b.txt");
    expect(items[0].vaultId).toBe("vault-1");
  });

  it("add returns the newly created items", () => {
    const files = [mockFile("x.txt")];
    const result = useUploadStore.getState().add(files, "vault-1");
    expect(result).toHaveLength(1);
    expect(result[0].fileName).toBe("x.txt");
  });

  it("update patches a specific item by id", () => {
    const [item] = useUploadStore.getState().add([mockFile("c.txt")], "v");
    useUploadStore
      .getState()
      .update(item.id, { status: "uploading", bytesUploaded: 500 });
    const updated = useUploadStore.getState().items.find((i) => i.id === item.id);
    expect(updated?.status).toBe("uploading");
    expect(updated?.bytesUploaded).toBe(500);
  });

  it("update does not affect other items", () => {
    const [a, b] = useUploadStore
      .getState()
      .add([mockFile("a.txt"), mockFile("b.txt")], "v");
    useUploadStore.getState().update(a.id, { status: "done" });
    const bItem = useUploadStore.getState().items.find((i) => i.id === b.id);
    expect(bItem?.status).toBe("queued");
  });

  it("remove deletes an item by id", () => {
    const [item] = useUploadStore.getState().add([mockFile("r.txt")], "v");
    useUploadStore.getState().remove(item.id);
    expect(useUploadStore.getState().items).toHaveLength(0);
  });

  it("clearDone removes only done items", () => {
    const [a, b] = useUploadStore
      .getState()
      .add([mockFile("a.txt"), mockFile("b.txt")], "v");
    useUploadStore.getState().update(a.id, { status: "done" });
    useUploadStore.getState().clearDone();
    const remaining = useUploadStore.getState().items;
    expect(remaining).toHaveLength(1);
    expect(remaining[0].id).toBe(b.id);
  });
});
