# Studio frontend

The React/Vite frontend is the cross-platform presentation layer embedded by Tauri. It currently provides:

- a searchable Pack and Theme library;
- a true-ratio Avatr 4032 × 284 preview with day/night scenarios;
- exact and slider-based theme parameter controls;
- generated minimal CSS parameter patches;
- preview events, diagnostics and theme metadata panels.

The UI state model lives in `src/studio/model.ts` so project and preview behavior can be tested without a desktop window. Filesystem access, signing and packaging remain in Rust and are exposed to this layer through typed Tauri commands.

```sh
npm ci
npm run lint
npm test
npm run build
npm run dev
```
