import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

test("web application mounts from the local page", async () => {
  const html = await readFile(new URL("../index.html", import.meta.url), "utf8");
  const entry = await readFile(new URL("../src/main.tsx", import.meta.url), "utf8");
  assert.match(html, /id="root"/);
  assert.match(html, /src="\/src\/main\.tsx"/);
  assert.match(entry, /createRoot\(rootElement\)/);
});

test("catalog UI exposes workspace selection, search, source and capability state", async () => {
  const app = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
  assert.match(app, /Workspace folder path/);
  assert.match(app, /Open workspace/);
  assert.match(app, /Search files and readable text/);
  assert.match(app, /\/api\/source\//);
  assert.match(app, /capability_level/);
  assert.match(app, /Binary file/);
});

test("workspace explorer exposes the repository hierarchy", async () => {
  const app = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
  assert.match(app, /\/api\/tree/);
  assert.match(app, /Repository file tree/);
  assert.match(app, /is_directory/);
  assert.match(app, /Load more/);
});

test("Java analysis view exposes evidence states from the local API", async () => {
  const app = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
  const server = await readFile(new URL("../../crates/server/src/lib.rs", import.meta.url), "utf8");
  assert.match(app, /Java call analysis/);
  assert.match(app, /\/api\/analysis/);
  assert.match(app, /resolution_state/);
  assert.match(app, /Refresh analysis/);
  assert.match(app, /\.codewiki\/analysis/);
  assert.match(server, /route\("\/api\/analysis"/);
  assert.match(server, /unresolved_calls/);
});

test("catalog UI makes no external font or service requests", async () => {
  const app = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
  const css = await readFile(new URL("../src/App.css", import.meta.url), "utf8");
  assert.doesNotMatch(app, /fetch\(["']https?:/);
  assert.doesNotMatch(css, /https?:\/\//);
});
