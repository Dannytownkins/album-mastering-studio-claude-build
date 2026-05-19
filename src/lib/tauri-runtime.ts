// Tauri runtime shim.
//
// When the app runs inside the Tauri WebView, every export here passes
// straight through to the real `@tauri-apps/*` modules. When the app runs
// in a plain browser at `localhost:1420` (Vite dev preview), every export
// resolves to a mock so the UI can render and be screenshot-verified
// without the Rust backend. Used both for `agent-browser`-style visual
// verification and for quick component iteration in any browser.
//
// Detection: Tauri injects `window.__TAURI_INTERNALS__` into the WebView
// before the JS bundle runs. If that global is missing, we're in a plain
// browser. Resolution is lazy (inside each exported function) to avoid
// top-level await, which Vite's target doesn't allow.

import type { UnlistenFn } from "@tauri-apps/api/event";

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
  }
}

export const isTauri = (): boolean =>
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

// ---------------------------------------------------------------------------
// invoke / listen
// ---------------------------------------------------------------------------

type InvokeImpl = <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;
type ListenImpl = <T>(
  channel: string,
  handler: (event: { payload: T }) => void,
) => Promise<UnlistenFn>;

let invokeImplPromise: Promise<InvokeImpl> | null = null;
function getInvokeImpl(): Promise<InvokeImpl> {
  if (!invokeImplPromise) {
    invokeImplPromise = isTauri()
      ? import("@tauri-apps/api/core").then((m) => m.invoke as InvokeImpl)
      : import("./preview-mock").then((m) => m.mockInvoke);
  }
  return invokeImplPromise;
}

let listenImplPromise: Promise<ListenImpl> | null = null;
function getListenImpl(): Promise<ListenImpl> {
  if (!listenImplPromise) {
    listenImplPromise = isTauri()
      ? import("@tauri-apps/api/event").then((m) => m.listen as ListenImpl)
      : import("./preview-mock").then((m) => m.mockListen);
  }
  return listenImplPromise;
}

export async function invoke<T>(
  cmd: string,
  args?: Record<string, unknown>,
): Promise<T> {
  const fn = await getInvokeImpl();
  return fn<T>(cmd, args);
}

export async function listen<T>(
  channel: string,
  handler: (event: { payload: T }) => void,
): Promise<UnlistenFn> {
  const fn = await getListenImpl();
  return fn<T>(channel, handler);
}

// ---------------------------------------------------------------------------
// Dialog (open / save) — used by useTrackMaster for Open / Save As project.
// ---------------------------------------------------------------------------

type OpenDialogOptions = {
  directory?: boolean;
  multiple?: boolean;
  title?: string;
  filters?: Array<{ name: string; extensions: string[] }>;
};
type SaveDialogOptions = {
  defaultPath?: string;
  filters?: Array<{ name: string; extensions: string[] }>;
};

type OpenImpl = (opts?: OpenDialogOptions) => Promise<string | string[] | null>;
type SaveImpl = (opts?: SaveDialogOptions) => Promise<string | null>;

let openImplPromise: Promise<OpenImpl> | null = null;
function getOpenImpl(): Promise<OpenImpl> {
  if (!openImplPromise) {
    openImplPromise = isTauri()
      ? import("@tauri-apps/plugin-dialog").then((m) => m.open as OpenImpl)
      : import("./preview-mock").then((m) => m.mockOpen);
  }
  return openImplPromise;
}

let saveImplPromise: Promise<SaveImpl> | null = null;
function getSaveImpl(): Promise<SaveImpl> {
  if (!saveImplPromise) {
    saveImplPromise = isTauri()
      ? import("@tauri-apps/plugin-dialog").then((m) => m.save as SaveImpl)
      : import("./preview-mock").then((m) => m.mockSave);
  }
  return saveImplPromise;
}

export async function open(
  opts?: OpenDialogOptions,
): Promise<string | string[] | null> {
  const fn = await getOpenImpl();
  return fn(opts);
}

export async function save(opts?: SaveDialogOptions): Promise<string | null> {
  const fn = await getSaveImpl();
  return fn(opts);
}

// ---------------------------------------------------------------------------
// Webview (drag-drop). In browser preview drag-drop is a no-op — there's no
// path access from a plain browser anyway, so the canned mock returns a
// handle whose `onDragDropEvent` does nothing.
// ---------------------------------------------------------------------------

type DragDropEvent = { payload: unknown };
type WebviewHandle = {
  onDragDropEvent: (
    handler: (event: DragDropEvent) => void,
  ) => Promise<UnlistenFn>;
};

// getCurrentWebview is synchronous (returns an object). To avoid surfacing
// async on call sites we cache the live handle by kicking off the import
// eagerly on first access and returning a proxy object whose method just
// awaits the underlying handle.
let webviewHandlePromise: Promise<WebviewHandle> | null = null;
function getWebviewHandle(): Promise<WebviewHandle> {
  if (!webviewHandlePromise) {
    webviewHandlePromise = isTauri()
      ? import("@tauri-apps/api/webview").then(
          (m) => m.getCurrentWebview() as unknown as WebviewHandle,
        )
      : import("./preview-mock").then((m) => m.mockWebview());
  }
  return webviewHandlePromise;
}

export function getCurrentWebview(): WebviewHandle {
  return {
    onDragDropEvent: async (handler) => {
      const handle = await getWebviewHandle();
      return handle.onDragDropEvent(handler);
    },
  };
}
