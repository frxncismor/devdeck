// Drives a running DevDeck (WebView2) over CDP. Start it with scripts/qa/qa-app.ps1.
// Usage: pnpm exec tsx scripts/qa/cdp.mts <action> [args]   (see docs/testing/qa-fixtures.md)
import { mkdirSync, readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { chromium, type Page } from "playwright-core";

const endpoint = `http://127.0.0.1:${process.env.DEVDECK_CDP_PORT ?? 9222}`;
const timeout = Number(process.env.DEVDECK_CDP_TIMEOUT ?? 15000);

async function run(page: Page, [action, ...args]: string[]): Promise<void> {
  const need = (i: number): string => {
    if (args[i] === undefined) throw new Error(`${action}: missing argument`);
    return args[i];
  };
  const target = () => page.locator(need(0)).first();
  switch (action) {
    case "captura": {
      const file = resolve(need(0));
      mkdirSync(dirname(file), { recursive: true });
      await page.screenshot({ path: file });
      console.log(file);
      break;
    }
    case "texto":
      console.log(
        await page
          .locator(args[0] ?? "body")
          .first()
          .innerText(),
      );
      break;
    case "clic":
      await target().click();
      break;
    case "tecla":
      await page.keyboard.press(need(0));
      break;
    case "escribir":
      await target().fill(need(1));
      break;
    case "esperar":
      await target().waitFor({ state: "visible" });
      break;
    case "pausa":
      await page.waitForTimeout(Number(need(0)));
      break;
    case "eval":
      console.log(JSON.stringify(await page.evaluate(need(0)), null, 2));
      break;
    case "abrir-json": {
      // The file picker is a native dialog (out of reach of CDP): seed the "recent" list instead.
      const path = resolve(need(0));
      await page.evaluate((p) => {
        localStorage.setItem("devdeck.json.recent", JSON.stringify([p]));
      }, path);
      await page.reload();
      await page
        .getByRole("button", { name: /^(JSON Viewer|Visor JSON)/ })
        .click();
      await page.locator("button.recent-open").first().click();
      break;
    }
    default:
      throw new Error(`unknown action: ${action}`);
  }
}

const [action, ...args] = process.argv.slice(2);
if (!action) {
  console.error(
    "actions: captura <png> | texto [sel] | clic <sel> | tecla <keys> | escribir <sel> <text> | " +
      "esperar <sel> | pausa <ms> | eval <js> | abrir-json <file> | pasos <steps.json>",
  );
  process.exit(2);
}
const steps: string[][] =
  action === "pasos"
    ? (JSON.parse(readFileSync(args[0], "utf8")) as string[][])
    : [[action, ...args]];

// connectOverCDP attaches to the existing WebView2; no browser is downloaded.
const browser = await chromium.connectOverCDP(endpoint);
try {
  const page = browser.contexts()[0]?.pages()[0];
  if (!page) throw new Error("no page found over CDP");
  page.setDefaultTimeout(timeout);
  for (const step of steps) await run(page, step.map(String));
} finally {
  await browser.close(); // only disconnects: the app keeps running
}
