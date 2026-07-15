# Studio frontend

The React/Vite frontend is the cross-platform presentation layer embedded by Tauri. It currently provides:

- a searchable Pack and Theme library;
- a true-ratio Avatr 4032 × 284 preview with day/night scenarios;
- Pack Schema-generated color, length/number, select, toggle and text controls;
- manifest-scoped CSS, HTML and JSON source tabs with per-document dirty state;
- source find/replace, syntax diagnostics, keyboard save and external-change protection;
- generated minimal CSS parameter patches with shared source-level undo/redo;
- preview events, diagnostics and theme metadata panels.

The UI state models live in `src/studio/` so project and preview behavior can be tested without a desktop window. Browser development receives a typed in-memory backend. Filesystem access, signing and packaging remain in Rust and are exposed to this layer through typed Tauri commands.

```sh
npm ci
npm run lint
npm test
npm run build
npm run dev
```
