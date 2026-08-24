import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";

import { startAlbionHost, stopAlbionHost } from "./albionHostService";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const mockedInvoke = vi.mocked(invoke);

describe("albionHostService", () => {
  beforeEach(() => mockedInvoke.mockReset());

  it("starts the bundled host with the supplied private secret", async () => {
    mockedInvoke.mockResolvedValueOnce({ server_url: "http://127.0.0.1:38421" });
    await expect(startAlbionHost("private-code")).resolves.toEqual({ server_url: "http://127.0.0.1:38421" });
    expect(mockedInvoke).toHaveBeenCalledWith("start_albion_host", { joinSecret: "private-code" });
  });

  it("stops the bundled host", async () => {
    mockedInvoke.mockResolvedValueOnce(undefined);
    await expect(stopAlbionHost()).resolves.toBeUndefined();
    expect(mockedInvoke).toHaveBeenCalledWith("stop_albion_host");
  });
});
