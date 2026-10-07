# Observe the game from outside only

MapleCompass must not be seen as interacting with the MapleStory client, because third-party interaction can breach the terms of service and get the player's account banned. So it only reads pixels through the operating system's ordinary screen-capture APIs, and it only draws in its own separate windows. It never reads or writes the game's memory or files at runtime, never injects code or hooks the renderer, and never sends input to the game.

Normally clicks pass through the overlay to the game. Two moments are exceptions, and both are our own windows taking the mouse while they lie over the game, never anything sent to the game: edit mode, where the player drags the panel, and the region-marking window, where the player drags a box around the minimap title at setup. While either is up, a click on it lands on MapleCompass and the game doesn't see it, just as with any other window in front of the game.

The mob search window is a third exception, and the only one that takes the keyboard. A hotkey opens it, and it takes focus like any app window the player switches to, so their typing goes to it; pressing it again while the window is open brings it back in front the same way. Picking a mob or pressing Esc closes it, and Windows gives focus back to the game. Nothing is sent to the game at any point.

Keyboard input comes only through global hotkeys registered with the operating system (`RegisterHotKey` on Windows): the system tells us when the player presses one of our combinations. There are no low-level keyboard hooks (`WH_KEYBOARD_LL`) and no polling of key state (`GetAsyncKeyState`). A registered combination doesn't reach the game, or any other app, while MapleCompass runs. The defaults are Ctrl+Shift+letter. Win+Alt+letter was tried first, to stay clear of what players bind in the game, but Windows and other apps already hold many of those letters (Win+Alt+F was taken on the owner's machine). So we accept that a Ctrl+Shift combination we register is lost to the game and to other apps, and a player who needs one rebinds it in the settings file.

## Consequences

The map has to be inferred from the rendered minimap title (OCR or glyph matching) rather than read directly. Shortcuts such as reading the current map ID from process memory are permanently off the table, even though they would be more accurate.

To stay at the game's level in the z-order, rather than on top of every app, the overlay is kept just above the game by polling the window order and restacking only our own window (`SetWindowPos` with no activation). Making the overlay an owned or child window of the game (`GWLP_HWNDPARENT`, `SetParent`) would do this for free, but it is ruled out: a cross-process owner or parent link attaches our input queue to the game's, which is interaction with the game client. Window-event hooks (`SetWinEventHook`) are avoided for the same reason.

Our windows are not hidden from screen capture, so the panel shows in the player's screenshots and streams. Our own capture of the Title Region would then read the panel if it covered the minimap title, so the core never places the panel there: inside the game window it moves clear of the Title Region, whether at its default spot or where the player dragged it.
