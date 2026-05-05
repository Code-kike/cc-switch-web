declare global {
  interface Window {
    __TAURI__?: unknown
    __TAURI_INTERNALS__?: unknown
    __CC_SWITCH_API_BASE__?: string
  }

  interface ImportMetaEnv {
    readonly VITE_TAURI_ENV?: string
  }

  interface ImportMeta {
    readonly env: ImportMetaEnv
  }
}

export {}
