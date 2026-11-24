import { mount } from "svelte";
import App from "./App.svelte";
import { loadConfig } from "./lib/config.svelte";
import { refreshProjects } from "./lib/scope.svelte";

await loadConfig();
mount(App, { target: document.getElementById("app")! });
// The backend warms the project list at startup; fill the scope selector without blocking the UI.
void refreshProjects();
