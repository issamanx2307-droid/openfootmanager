import { create } from "zustand";
import { invoke, isTauri } from "@tauri-apps/api/core";

export interface AppSettings {
  theme: "dark" | "light" | "system";
  language: string;
  currency: "EUR" | "GBP" | "USD";
  default_match_mode: "live" | "spectator" | "delegate";
  auto_save: boolean;
  match_speed: "slow" | "normal" | "fast";
  show_match_commentary: boolean;
  confirm_advance: boolean;
  continue_to_next_event: boolean;
  ui_scale: "small" | "normal" | "large" | "xlarge";
  high_contrast: boolean;
  reduced_motion: boolean;
  match_camera_mode: "full" | "follow-ball";
  match_highlight_mode: "key" | "extended" | "full";
  show_match_player_names: boolean;
  show_match_role_labels: boolean;
  show_match_formation_shape: boolean;
}

export interface CurrencyDefinition {
  code: AppSettings["currency"];
  symbol: string;
  exchange_rate: number;
}

interface SettingsResponse {
  settings: Partial<AppSettings>;
  currency: CurrencyDefinition;
  supported_currencies: CurrencyDefinition[];
}

const DEFAULT_SETTINGS: AppSettings = {
  theme: "dark",
  language: "en",
  currency: "EUR",
  default_match_mode: "live",
  auto_save: true,
  match_speed: "normal",
  show_match_commentary: true,
  confirm_advance: false,
  continue_to_next_event: false,
  ui_scale: "normal",
  high_contrast: false,
  reduced_motion: false,
  match_camera_mode: "full",
  match_highlight_mode: "full",
  show_match_player_names: false,
  show_match_role_labels: false,
  show_match_formation_shape: false,
};

const DEFAULT_CURRENCY: CurrencyDefinition = {
  code: "EUR",
  symbol: "€",
  exchange_rate: 1,
};

const WEB_SETTINGS_STORAGE_KEY = "openfootmanager.settings";

function loadWebSettings(): Partial<AppSettings> {
  if (typeof window === "undefined") return {};

  try {
    const stored = window.localStorage.getItem(WEB_SETTINGS_STORAGE_KEY);
    return stored ? JSON.parse(stored) as Partial<AppSettings> : {};
  } catch {
    return {};
  }
}

function persistWebSettings(settings: AppSettings) {
  if (typeof window !== "undefined") {
    window.localStorage.setItem(WEB_SETTINGS_STORAGE_KEY, JSON.stringify(settings));
  }
}

function mergeWithDefaultSettings(settings: Partial<AppSettings> = {}): AppSettings {
  return { ...DEFAULT_SETTINGS, ...settings };
}

async function persistSettings(settings: AppSettings) {
  if (!isTauri()) {
    persistWebSettings(settings);
    return;
  }

  await invoke("save_settings", { settings });
}

function indexSupportedCurrencies(
  currencies: CurrencyDefinition[] = [],
): Record<string, CurrencyDefinition> {
  const index = currencies.reduce<Record<string, CurrencyDefinition>>((acc, currency) => {
    acc[currency.code] = currency;
    return acc;
  }, {});

  if (!index[DEFAULT_CURRENCY.code]) {
    index[DEFAULT_CURRENCY.code] = DEFAULT_CURRENCY;
  }

  return index;
}

function resolveCurrency(
  code: AppSettings["currency"],
  supportedCurrencies: Record<string, CurrencyDefinition>,
): CurrencyDefinition {
  return supportedCurrencies[code]
    ?? supportedCurrencies[DEFAULT_SETTINGS.currency]
    ?? DEFAULT_CURRENCY;
}

interface SettingsStore {
  settings: AppSettings;
  currency: CurrencyDefinition;
  supportedCurrencies: Record<string, CurrencyDefinition>;
  loaded: boolean;
  loadSettings: () => Promise<void>;
  updateSettings: (partial: Partial<AppSettings>) => Promise<void>;
}

export const useSettingsStore = create<SettingsStore>((set, get) => ({
  settings: mergeWithDefaultSettings(),
  currency: DEFAULT_CURRENCY,
  supportedCurrencies: indexSupportedCurrencies(),
  loaded: false,

  loadSettings: async () => {
    if (!isTauri()) {
      const settings = mergeWithDefaultSettings(loadWebSettings());
      const supportedCurrencies = indexSupportedCurrencies();
      set({
        settings,
        currency: resolveCurrency(settings.currency, supportedCurrencies),
        supportedCurrencies,
        loaded: true,
      });
      return;
    }

    try {
      const response = await invoke<SettingsResponse>("get_settings");
      const settings = mergeWithDefaultSettings(response.settings);
      const supportedCurrencies = indexSupportedCurrencies(response.supported_currencies);

      set({
        settings,
        currency: resolveCurrency(settings.currency, supportedCurrencies),
        supportedCurrencies,
        loaded: true,
      });
    } catch {
      set({
        settings: mergeWithDefaultSettings(),
        currency: DEFAULT_CURRENCY,
        supportedCurrencies: indexSupportedCurrencies(),
        loaded: true,
      });
    }
  },

  updateSettings: async (partial) => {
    const previousState = get();
    const previousSettings = previousState.settings;
    const merged = mergeWithDefaultSettings({ ...previousSettings, ...partial });
    const currency = resolveCurrency(merged.currency, previousState.supportedCurrencies);

    set({ settings: merged, currency });
    try {
      await persistSettings(merged);
    } catch (err) {
      set({
        settings: previousSettings,
        currency: previousState.currency,
        supportedCurrencies: previousState.supportedCurrencies,
      });
      console.error("Failed to save settings:", err);
    }
  },
}));
