import { afterAll, describe, expect, it } from "vitest";
import i18n, {
  changeAppLanguage,
  i18nReady,
  resolveSupportedLanguage,
  SUPPORTED_LANGUAGES,
} from "./index";

describe("resolveSupportedLanguage", () => {
  it("exposes only English and Thai to the app", () => {
    expect(SUPPORTED_LANGUAGES.map(({ code }) => code)).toEqual(["en", "th"]);
  });

  it("keeps Thai and English matching behavior", () => {
    expect(resolveSupportedLanguage("en-US")).toBe("en");
    expect(resolveSupportedLanguage("th-TH")).toBe("th");
  });

  it("falls back to English for unsupported locales", () => {
    expect(resolveSupportedLanguage("nl-NL")).toBe("en");
    expect(resolveSupportedLanguage("zh-CN")).toBe("en");
  });
});

describe("i18n lazy loading", () => {
  afterAll(async () => {
    await changeAppLanguage("en");
  });

  it("initializes with the active language resources instead of all locales", async () => {
    await i18nReady;

    expect(i18n.hasResourceBundle("en", "translation")).toBe(true);
    expect(i18n.hasResourceBundle("th", "translation")).toBe(false);
  });

  it("loads the Thai locale bundle on demand when the app language changes", async () => {
    await i18nReady;

    await changeAppLanguage("th-TH");
    expect(i18n.language).toBe("th");
    expect(i18n.t("menu.newGame")).toBe("เริ่มเกมใหม่");
    expect(i18n.t("createManager.title")).toBe("สร้างผู้จัดการ");
    expect(i18n.t("worldSelect.startCareer")).toBe("เริ่มอาชีพ");
    expect(i18n.t("generation.title")).toBe("สร้างและเตรียมโลก");
  });
});
