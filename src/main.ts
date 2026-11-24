import { mount } from "svelte";
import App from "./App.svelte";
import { loadConfig } from "./lib/config.svelte";
import { refreshProjects } from "./lib/scope.svelte";

await loadConfig();
await refreshProjects();

mount(App, { target: document.getElementById("app")! });
