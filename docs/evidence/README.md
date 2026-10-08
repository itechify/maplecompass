# Title Region marking evidence

Captured on 7 October 2026 at 1280 × 720 CSS pixels. These browser previews use the real frontend source with simulated Tauri events; their plain backdrop is not a game capture.

- [Before](title-region-before.png): the marking window offers dragging and Escape only, from the implementation at `dde6ad0`.
- [After](title-region-after.png): the instructions expose Arrow keys, Alt+Arrow, Shift, Enter, and Escape. The box shown follows Shift+ArrowRight and Alt+ArrowDown; command inspection confirmed the matching move/resize inputs.

The core's public-input tests verify selection bounds, adjustment steps, confirmation, cancellation, and invalidation when the game disappears or changes size. Native game interaction and screen-reader output were not exercised. These images are review evidence, not visual-regression goldens.
