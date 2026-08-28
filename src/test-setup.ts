import "@testing-library/jest-dom/vitest";
import { vi } from "vitest";
import { cloneElement, isValidElement, type ReactNode } from "react";

// jsdom has no layout engine, so Recharts cannot measure a responsive parent.
// Give chart children a stable test-only viewport instead of emitting misleading
// negative-size warnings during unrelated component tests.
vi.mock("recharts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("recharts")>();

  return {
    ...actual,
    ResponsiveContainer: ({ children }: { children: ReactNode }) =>
      isValidElement<{ width?: number; height?: number }>(children)
        ? cloneElement(children, { width: 800, height: 400 })
        : children,
  };
});

// Polyfill localStorage for jsdom environment in Node 22+.
// Node 24+ removed the built-in localStorage implementation and now requires
// the --localstorage-file flag. Vitest's jsdom environment doesn't pass that
// flag, so we provide a minimal in-memory shim here.
if (typeof localStorage === "undefined" || localStorage === null) {
  const store: Record<string, string> = {};
  Object.defineProperty(globalThis, "localStorage", {
    value: {
      getItem(key: string) {
        return key in store ? store[key] : null;
      },
      setItem(key: string, value: string) {
        store[key] = value;
      },
      removeItem(key: string) {
        delete store[key];
      },
      clear() {
        for (const key of Object.keys(store)) {
          delete store[key];
        }
      },
      get length() {
        return Object.keys(store).length;
      },
      key(index: number) {
        const keys = Object.keys(store);
        return keys[index] ?? null;
      },
    },
    writable: false,
    configurable: true,
  });
}
