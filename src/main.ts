import "./styles/fonts.css";
import "./styles/tokens.css";
import { mount } from "svelte";
import App from "./App.svelte";
import { loadConfig } from "./lib/config.svelte";
import { refreshProjects } from "./lib/scope.svelte";
import { initTheme } from "./lib/theme.svelte";

// Theme <html> synchronously, before any await/paint: no flash of the wrong theme.
initTheme();
await loadConfig();
mount(App, { target: document.getElementById("app")! });
// The backend warms the project list at startup; fill the scope selector without blocking the UI.
void refreshProjects();
