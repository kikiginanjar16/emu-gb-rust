# gbemu

`gbemu` is a Nintendo Gameboy emulator written in C++. It was written as an exercise (and for fun!) so its goals are exploration of modern C++ and clean code rather than total accuracy.

## Requirements

*   C++17 compatible compiler
*   CMake (3.10 or later)
*   SDL2

## Building

Building the emulator requires `cmake` and `SDL2` and has been tested on macOS and Debian. To compile the project, run:

```sh
make
```

This builds two versions of the emulator:

* `gbemu` - the main emulator, using SDL for graphics and input
* `gbemu-test` - a headless version of the emulator for debugging & running tests

## Playing

```
usage: gbemu <rom_file> [--debug] [--trace] [--silent] [--exit-on-infinite-jr] [--print-serial-output]

arguments:
  --debug                   Enable the debugger
  --exit-on-infinite-jr     Stop emulation if an infinite JR loop is detected
  --print-serial-output     Print data sent to the serial port
  --trace                   Enable trace logging
  --silent                  Disable logging
```


The key bindings are:

| Key | Action |
|:---:|:---:|
| <kbd>&uarr;</kbd>, <kbd>&darr;</kbd>, <kbd>&larr;</kbd>, <kbd>&rarr;</kbd> | D-pad |
| <kbd>X</kbd> | A |
| <kbd>Z</kbd> | B |
| <kbd>Enter</kbd> | Start |
| <kbd>Backspace</kbd> | Select |
| <kbd>F5</kbd> | Save Game (Battery) |
| <kbd>F6</kbd> | Load Game (Battery) |

The emulator will also automatically save the cartridge RAM (if supported by the game) to `<rom_filename>.sav` when exiting.

## Tests

The emulator is tested using [Blargg's tests][blarggs] - these can be ran with `./scripts/run_test_roms`.

<img src="./.github/images/blarggs-tests-pass.png" width="400">

## Missing features

Currently, `gbemu` only supports Gameboy games. I'm working on Gameboy Color support off-and-on at the moment. There's also no audio support yet.

## Screenshots

Menu | Gameplay
:-------------------------:|:-------------------------:
<img src="./.github/images/tetris-menu.png" width="400"> | <img src="./.github/images/tetris-gameplay.png" width="400">
<img src="./.github/images/zelda-menu.png" width="400"> | <img src="./.github/images/zelda-gameplay.png" width="400">
<img src="./.github/images/pokemon-menu.png" width="400"> | <img src="./.github/images/pokemon-gameplay.png" width="400">

[blarggs]: http://gbdev.gg8.se/wiki/articles/Test_ROMs

## Core API

The emulator can be embedded into other C++ applications. The core API is exposed through the `Gameboy` class.

### Main Struct

**`Gameboy`** (`src/gameboy.h`)

The main class representing the emulator instance.

### Functions

#### Create

```cpp
#include "gameboy.h"

// Load ROM and Save data (if any) into vectors
std::vector<u8> rom_data = ...;
std::vector<u8> save_data = ...; // Can be empty

// Configure options
Options options;
options.disable_logs = false;

// Create the instance
auto gameboy = std::make_unique<Gameboy>(rom_data, options, save_data);
```

#### Run / Step

The `run` method executes the main loop. It requires two callbacks:
1.  `should_close`: Return `true` to exit the loop.
2.  `vblank`: Called once per frame when the framebuffer is ready.

```cpp
gameboy->run(
    // should_close_callback
    []() -> bool {
        return false; // Return true to stop emulation
    },
    // vblank_callback
    [](const FrameBuffer& buffer) {
        // Render the framebuffer
    }
);
```

### Framebuffer

**`FrameBuffer`** (`src/video/framebuffer.h`)

Passed to the `vblank` callback. You can access pixels using `get_pixel(x, y)`.

```cpp
for (uint y = 0; y < GAMEBOY_HEIGHT; y++) {
    for (uint x = 0; x < GAMEBOY_WIDTH; x++) {
        Color color = buffer.get_pixel(x, y);
        // Map Color::White, LightGray, DarkGray, Black to your display format
    }
}
```
