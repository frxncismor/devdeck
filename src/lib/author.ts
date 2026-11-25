import { openUrl } from "@tauri-apps/plugin-opener";

/** The only URL the opener capability allows (see `capabilities/default.json`). */
export const AUTHOR_URL = "https://github.com/frxncismor";

/** Opens the author profile in the system browser, never inside the WebView. */
export function openAuthor(event?: Pick<Event, "preventDefault">): void {
  event?.preventDefault();
  void openUrl(AUTHOR_URL).catch((err) => console.error(err));
}
