# Observe the game from outside only

MapleCompass must not be seen as interacting with the MapleStory client, because third-party interaction can breach the terms of service and get the player's account banned. So it only reads pixels through the operating system's ordinary screen-capture APIs, and it only draws in its own separate always-on-top window that clicks pass through. It never reads or writes the game's memory or files at runtime, never injects code or hooks the renderer, and never sends input to the game.

## Consequences

The map has to be inferred from the rendered minimap title (OCR or glyph matching) rather than read directly. Shortcuts such as reading the current map ID from process memory are permanently off the table, even though they would be more accurate.
