import "@testing-library/jest-dom";
import { afterAll, afterEach, beforeAll, vi } from "vitest";
import { act, cleanup } from "@testing-library/react";
import { notifyManager } from "@tanstack/react-query";
import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import { server } from "./msw/server";
import { resetProviderState } from "./msw/state";
import "./msw/tauriMocks";

beforeAll(async () => {
  notifyManager.setNotifyFunction((callback) => {
    act(callback);
  });
  server.listen({ onUnhandledRequest: "warn" });
  await i18n.use(initReactI18next).init({
    lng: "zh",
    fallbackLng: "zh",
    resources: {
      zh: { translation: {} },
      en: { translation: {} },
    },
    interpolation: {
      escapeValue: false,
    },
  });
});

afterEach(() => {
  cleanup();
  resetProviderState();
  server.resetHandlers();
  vi.clearAllMocks();
});

afterAll(() => {
  notifyManager.setNotifyFunction((callback) => {
    callback();
  });
  server.close();
});
